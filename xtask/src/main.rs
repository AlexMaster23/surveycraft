use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use surveycraft_coding::{CodeBook, CompiledCodeBook};
use surveycraft_symbols::{SymbolCatalog, SymbolPack};
use wit_component::ComponentEncoder;
use zip::write::SimpleFileOptions;

fn main() {
    if let Err(error) = package_surveycraft() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
struct TaskError(String);

impl Display for TaskError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for TaskError {}

fn package_surveycraft() -> Result<(), Box<dyn Error>> {
    let root = workspace_root()?;
    let module_path = root.join("target/wasm32-unknown-unknown/release/surveycraft_extension.wasm");
    let manifest_path = root.join("extension/reference/extension.json");
    let symbol_pack_path = root.join("assets/symbols/surveycraft-basic.symbol-pack.json");
    let code_book_path = root.join("assets/codebooks/surveycraft-basic.code-book.json");
    let module = fs::read(&module_path).map_err(|error| {
        TaskError(format!(
            "build the release WASM guest first ({}): {error}",
            module_path.display()
        ))
    })?;
    let manifest = fs::read(&manifest_path)?;
    validate_extension_manifest(&manifest)?;
    let symbol_pack = fs::read(&symbol_pack_path)?;
    let parsed_symbol_pack: SymbolPack = serde_json::from_slice(&symbol_pack)?;
    let catalog = SymbolCatalog::compile(vec![parsed_symbol_pack])?;
    let code_book = fs::read(&code_book_path)?;
    let parsed_code_book: CodeBook = serde_json::from_slice(&code_book)?;
    CompiledCodeBook::compile(parsed_code_book.clone())?;
    catalog.validate_code_book(&parsed_code_book)?;
    let component = ComponentEncoder::default()
        .module(&module)?
        .validate(true)
        .encode()?;

    let output_directory = root.join("target/extension-package");
    fs::create_dir_all(&output_directory)?;
    let component_name = "surveycraft.wasm";
    fs::write(output_directory.join(component_name), &component)?;

    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    archive.start_file("extension.json", options)?;
    archive.write_all(&manifest)?;
    archive.start_file(component_name, options)?;
    archive.write_all(&component)?;
    archive.start_file("assets/symbols/surveycraft-basic.symbol-pack.json", options)?;
    archive.write_all(&symbol_pack)?;
    archive.start_file("assets/codebooks/surveycraft-basic.code-book.json", options)?;
    archive.write_all(&code_book)?;
    let package = archive.finish()?.into_inner();
    let package_path = output_directory.join("surveycraft.cadcraft-extension");
    fs::write(&package_path, package)?;
    println!("{}", package_path.display());
    Ok(())
}

fn validate_extension_manifest(manifest: &[u8]) -> Result<(), TaskError> {
    let value: serde_json::Value = serde_json::from_slice(manifest)
        .map_err(|error| TaskError(format!("invalid extension manifest JSON: {error}")))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| TaskError("extension manifest has no string version".into()))?;
    if version != env!("CARGO_PKG_VERSION") {
        return Err(TaskError(format!(
            "extension manifest version {version} does not match workspace version {}",
            env!("CARGO_PKG_VERSION")
        )));
    }
    let commands = value
        .get("commands")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| TaskError("extension manifest has no command array".into()))?;
    for (command_index, command) in commands.iter().enumerate() {
        let Some(ui) = command.get("ui") else {
            continue;
        };
        let mut keys = HashSet::new();
        for property in ["idKey", "entityIdsKey"] {
            if let Some(key) = ui.get(property).and_then(serde_json::Value::as_str) {
                validate_manifest_field(key, command_index, property, &mut keys)?;
            }
        }
        let Some(columns) = ui.get("columns").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for (column_index, column) in columns.iter().enumerate() {
            let key = column
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    TaskError(format!(
                        "commands[{command_index}].ui.columns[{column_index}] has no string key"
                    ))
                })?;
            validate_manifest_field(
                key,
                command_index,
                &format!("columns[{column_index}].key"),
                &mut keys,
            )?;
        }
    }
    Ok(())
}

fn validate_manifest_field(
    key: &str,
    command_index: usize,
    property: &str,
    keys: &mut HashSet<String>,
) -> Result<(), TaskError> {
    let mut characters = key.chars();
    let valid = key.len() <= 64
        && characters
            .next()
            .is_some_and(|value| value.is_ascii_lowercase())
        && characters.all(|value| {
            value.is_ascii_lowercase() || value.is_ascii_digit() || matches!(value, '.' | '_' | '-')
        });
    if !valid || !keys.insert(key.to_ascii_lowercase()) {
        return Err(TaskError(format!(
            "commands[{command_index}].ui.{property} must be a unique lower-case field name"
        )));
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf, TaskError> {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_directory
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| TaskError("xtask has no workspace parent".into()))
}
