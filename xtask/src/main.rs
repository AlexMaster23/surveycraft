use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
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
    let module = fs::read(&module_path).map_err(|error| {
        TaskError(format!(
            "build the release WASM guest first ({}): {error}",
            module_path.display()
        ))
    })?;
    let manifest = fs::read(&manifest_path)?;
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
    let package = archive.finish()?.into_inner();
    let package_path = output_directory.join("surveycraft.cadcraft-extension");
    fs::write(&package_path, package)?;
    println!("{}", package_path.display());
    Ok(())
}

fn workspace_root() -> Result<PathBuf, TaskError> {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_directory
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| TaskError("xtask has no workspace parent".into()))
}
