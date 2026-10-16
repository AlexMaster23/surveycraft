use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use surveycraft_coding::{CompiledCodeBook, GenerateOptions, MAX_CODE_BOOK_BYTES};
use surveycraft_core::{Crs, LinearUnit, NumericPolicy};
use surveycraft_io::{
    CodeBookFormat, CsvImportOptions, ImportReport, MAX_INPUT_BYTES, import_point_csv,
    read_code_book, read_project_json, read_symbol_pack_json, write_feature_plan_json,
    write_project_json, write_report_json,
};
use surveycraft_symbols::{
    MAX_CATALOG_PACKS, MAX_SYMBOL_PACK_BYTES, SymbolAttribution, SymbolCatalog, SymbolDefinition,
};
use uuid::Uuid;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), CliError> {
    let command = parse_command(&arguments)?;
    match command {
        Command::Validate(options) => {
            let report = load_csv(&options)?;
            let json = write_report_json(&report)?;
            print_utf8(&json)?;
            if report.summary.errors > 0 {
                return Err(CliError::ValidationFailed(report.summary.errors));
            }
        }
        Command::Import(options) => {
            let report = load_csv(&options.common)?;
            if report.summary.errors > 0 {
                let json = write_report_json(&report)?;
                print_utf8(&json)?;
                return Err(CliError::ValidationFailed(report.summary.errors));
            }
            let output = write_project_json(&report.project)?;
            fs::write(&options.output, output)?;
            println!(
                "Imported {} points into {}",
                report.summary.imported_points,
                options.output.display()
            );
        }
        Command::Inspect(path) => {
            let bytes = read_bounded(&path, MAX_INPUT_BYTES)?;
            let project = read_project_json(&bytes)?;
            let summary = ProjectSummary {
                schema_version: project.schema_version,
                id: project.id,
                name: project.name,
                linear_unit: project.linear_unit,
                crs_name: project.crs.name,
                points: project.points.len(),
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::CodeBookValidate(path) => {
            let book = load_code_book(&path)?;
            println!(
                "Valid code book '{}' ({} layers, {} rules)",
                book.name,
                book.layers.len(),
                book.rules.len()
            );
        }
        Command::CodeBookPreview { path, code } => {
            let book = CompiledCodeBook::compile(load_code_book(&path)?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&book.interpret_code(&code)?)?
            );
        }
        Command::CodeBookGenerate(options) => {
            let book = CompiledCodeBook::compile(load_code_book(&options.book)?)?;
            let project = read_project_json(&read_bounded(&options.project, MAX_INPUT_BYTES)?)?;
            let plan = book.generate_features(
                project.id,
                &project.points,
                GenerateOptions {
                    plan_scale_denominator: options.plan_scale_denominator,
                    include_unmarked: options.include_unmarked,
                    coordinate_tolerance: project.numeric_policy.coordinate_tolerance,
                },
            )?;
            fs::write(&options.output, write_feature_plan_json(&plan)?)?;
            println!(
                "Generated {} path(s), {} circle(s), and {} symbol(s) into {}",
                plan.paths.len(),
                plan.circles.len(),
                plan.symbols.len(),
                options.output.display()
            );
        }
        Command::SymbolPackValidate(paths) => {
            let catalog = load_symbol_catalog(&paths)?;
            println!(
                "Valid symbol catalog ({} pack(s), {} symbol(s))",
                paths.len(),
                catalog.symbol_ids().len()
            );
        }
        Command::SymbolPackInspect { path, symbol } => {
            let catalog = load_symbol_catalog(&[path])?;
            let definition = catalog
                .definition(&symbol)
                .cloned()
                .ok_or_else(|| surveycraft_symbols::SymbolError::UnknownSymbol(symbol.clone()))?;
            let attribution = catalog
                .attribution(&symbol)
                .ok_or_else(|| surveycraft_symbols::SymbolError::UnknownSymbol(symbol.clone()))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&SymbolDetails {
                    attribution,
                    definition,
                })?
            );
        }
        Command::SymbolPackCheckCodeBook { book, packs } => {
            let book = load_code_book(&book)?;
            let catalog = load_symbol_catalog(&packs)?;
            catalog.validate_code_book(&book)?;
            println!(
                "Code book '{}' resolves all symbols across {} pack(s)",
                book.name,
                packs.len()
            );
        }
        Command::Help => print_help(),
    }
    Ok(())
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(std::io::Error),
    SurveyIo(surveycraft_io::IoError),
    Coding(surveycraft_coding::CodingError),
    Symbol(surveycraft_symbols::SymbolError),
    Json(serde_json::Error),
    Utf8(std::str::Utf8Error),
    InvalidUuid(uuid::Error),
    ValidationFailed(usize),
    InputTooLarge { path: PathBuf, maximum: usize },
    TooManySymbolPacks { actual: usize, maximum: usize },
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => {
                write!(formatter, "{message}\nRun 'surveycraft --help' for usage.")
            }
            Self::Io(error) => write!(formatter, "file operation failed: {error}"),
            Self::SurveyIo(error) => Display::fmt(error, formatter),
            Self::Coding(error) => Display::fmt(error, formatter),
            Self::Symbol(error) => Display::fmt(error, formatter),
            Self::Json(error) => write!(formatter, "could not produce JSON: {error}"),
            Self::Utf8(error) => write!(formatter, "internal JSON was not UTF-8: {error}"),
            Self::InvalidUuid(error) => write!(formatter, "invalid project UUID: {error}"),
            Self::ValidationFailed(count) => {
                write!(
                    formatter,
                    "validation found {count} error(s); no project was written"
                )
            }
            Self::InputTooLarge { path, maximum } => write!(
                formatter,
                "input '{}' exceeds the {maximum}-byte safety limit",
                path.display()
            ),
            Self::TooManySymbolPacks { actual, maximum } => write!(
                formatter,
                "symbol catalog contains {actual} pack paths; the safety limit is {maximum}"
            ),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::SurveyIo(error) => Some(error),
            Self::Coding(error) => Some(error),
            Self::Symbol(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::InvalidUuid(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<surveycraft_io::IoError> for CliError {
    fn from(error: surveycraft_io::IoError) -> Self {
        Self::SurveyIo(error)
    }
}

impl From<surveycraft_coding::CodingError> for CliError {
    fn from(error: surveycraft_coding::CodingError) -> Self {
        Self::Coding(error)
    }
}

impl From<surveycraft_symbols::SymbolError> for CliError {
    fn from(error: surveycraft_symbols::SymbolError) -> Self {
        Self::Symbol(error)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::str::Utf8Error> for CliError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

impl From<uuid::Error> for CliError {
    fn from(error: uuid::Error) -> Self {
        Self::InvalidUuid(error)
    }
}

#[derive(Debug, Clone)]
enum Command {
    Validate(CsvOptions),
    Import(ImportOptions),
    Inspect(PathBuf),
    CodeBookValidate(PathBuf),
    CodeBookPreview { path: PathBuf, code: String },
    CodeBookGenerate(CodeBookGenerateOptions),
    SymbolPackValidate(Vec<PathBuf>),
    SymbolPackInspect { path: PathBuf, symbol: String },
    SymbolPackCheckCodeBook { book: PathBuf, packs: Vec<PathBuf> },
    Help,
}

#[derive(Debug, Clone)]
struct CsvOptions {
    input: PathBuf,
    project_name: String,
    project_id: Uuid,
}

#[derive(Debug, Clone)]
struct ImportOptions {
    common: CsvOptions,
    output: PathBuf,
}

#[derive(Debug, Clone)]
struct CodeBookGenerateOptions {
    book: PathBuf,
    project: PathBuf,
    output: PathBuf,
    plan_scale_denominator: f64,
    include_unmarked: bool,
}

fn parse_command(arguments: &[String]) -> Result<Command, CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(Command::Help);
    };
    match command {
        "--help" | "-h" | "help" => Ok(Command::Help),
        "validate" => Ok(Command::Validate(
            parse_csv_options(&arguments[1..], false)?.0,
        )),
        "import" => {
            let (common, output) = parse_csv_options(&arguments[1..], true)?;
            let output =
                output.ok_or_else(|| CliError::Usage("--output is required".to_owned()))?;
            Ok(Command::Import(ImportOptions { common, output }))
        }
        "inspect" => {
            if arguments.len() != 2 {
                return Err(CliError::Usage(
                    "inspect requires exactly one project JSON path".to_owned(),
                ));
            }
            Ok(Command::Inspect(PathBuf::from(&arguments[1])))
        }
        "codebook" => parse_code_book_command(&arguments[1..]),
        "symbolpack" => parse_symbol_pack_command(&arguments[1..]),
        unknown => Err(CliError::Usage(format!("unknown command '{unknown}'"))),
    }
}

fn parse_symbol_pack_command(arguments: &[String]) -> Result<Command, CliError> {
    let Some(action) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "symbolpack requires validate, inspect, or check-codebook".to_owned(),
        ));
    };
    match action {
        "validate" if arguments.len() >= 2 => Ok(Command::SymbolPackValidate(
            arguments[1..].iter().map(PathBuf::from).collect(),
        )),
        "inspect" if arguments.len() == 3 => Ok(Command::SymbolPackInspect {
            path: PathBuf::from(&arguments[1]),
            symbol: arguments[2].clone(),
        }),
        "check-codebook" if arguments.len() >= 3 => Ok(Command::SymbolPackCheckCodeBook {
            book: PathBuf::from(&arguments[1]),
            packs: arguments[2..].iter().map(PathBuf::from).collect(),
        }),
        "validate" | "inspect" | "check-codebook" => Err(CliError::Usage(format!(
            "symbolpack {action} received the wrong number of arguments"
        ))),
        unknown => Err(CliError::Usage(format!(
            "unknown symbolpack command '{unknown}'"
        ))),
    }
}

fn parse_code_book_command(arguments: &[String]) -> Result<Command, CliError> {
    let Some(action) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "codebook requires validate, preview, or generate".to_owned(),
        ));
    };
    match action {
        "validate" if arguments.len() == 2 => {
            Ok(Command::CodeBookValidate(PathBuf::from(&arguments[1])))
        }
        "preview" if arguments.len() == 3 => Ok(Command::CodeBookPreview {
            path: PathBuf::from(&arguments[1]),
            code: arguments[2].clone(),
        }),
        "generate" => parse_code_book_generate(&arguments[1..]),
        "validate" | "preview" => Err(CliError::Usage(format!(
            "codebook {action} received the wrong number of arguments"
        ))),
        unknown => Err(CliError::Usage(format!(
            "unknown codebook command '{unknown}'"
        ))),
    }
}

fn parse_code_book_generate(arguments: &[String]) -> Result<Command, CliError> {
    let book = arguments
        .first()
        .filter(|value| !value.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| CliError::Usage("a code-book path is required".to_owned()))?;
    let project = arguments
        .get(1)
        .filter(|value| !value.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| CliError::Usage("a project JSON path is required".to_owned()))?;
    let mut output = None;
    let mut plan_scale_denominator = 1_000.0;
    let mut include_unmarked = false;
    let mut index = 2usize;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--join-unmarked" => {
                include_unmarked = true;
                index += 1;
            }
            "--output" | "--plan-scale" => {
                let flag = &arguments[index];
                let value = arguments
                    .get(index + 1)
                    .ok_or_else(|| CliError::Usage(format!("option '{flag}' requires a value")))?;
                if flag == "--output" {
                    output = Some(PathBuf::from(value));
                } else {
                    plan_scale_denominator = value.parse::<f64>().map_err(|_| {
                        CliError::Usage(format!("'{value}' is not a valid plan scale"))
                    })?;
                }
                index += 2;
            }
            unknown => {
                return Err(CliError::Usage(format!("unknown option '{unknown}'")));
            }
        }
    }
    let output = output.ok_or_else(|| CliError::Usage("--output is required".to_owned()))?;
    Ok(Command::CodeBookGenerate(CodeBookGenerateOptions {
        book,
        project,
        output,
        plan_scale_denominator,
        include_unmarked,
    }))
}

fn parse_csv_options(
    arguments: &[String],
    accept_output: bool,
) -> Result<(CsvOptions, Option<PathBuf>), CliError> {
    let input = arguments
        .first()
        .filter(|value| !value.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| CliError::Usage("a point CSV path is required".to_owned()))?;
    let mut name = default_project_name(&input);
    let mut id = None;
    let mut output = None;
    let mut index = 1usize;
    while index < arguments.len() {
        let flag = &arguments[index];
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| CliError::Usage(format!("option '{flag}' requires a value")))?;
        match flag.as_str() {
            "--name" => name = value.clone(),
            "--project-id" => id = Some(Uuid::parse_str(value)?),
            "--output" if accept_output => output = Some(PathBuf::from(value)),
            _ => return Err(CliError::Usage(format!("unknown option '{flag}'"))),
        }
        index += 1;
    }
    let project_id = id.unwrap_or_else(|| Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes()));
    Ok((
        CsvOptions {
            input,
            project_name: name,
            project_id,
        },
        output,
    ))
}

fn default_project_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("SurveyCraft project")
        .to_owned()
}

fn load_csv(options: &CsvOptions) -> Result<ImportReport, CliError> {
    let bytes = read_bounded(&options.input, MAX_INPUT_BYTES)?;
    let source_name = options
        .input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("points.csv")
        .to_owned();
    let import_options = CsvImportOptions {
        project_id: options.project_id,
        project_name: options.project_name.clone(),
        source_name,
        crs: Crs::local_engineering(),
        linear_unit: LinearUnit::Metre,
        numeric_policy: NumericPolicy::default(),
        delimiter: b',',
    };
    Ok(import_point_csv(&bytes, &import_options)?)
}

fn load_code_book(path: &Path) -> Result<surveycraft_coding::CodeBook, CliError> {
    let format = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => CodeBookFormat::Json,
        Some("toml") => CodeBookFormat::Toml,
        _ => {
            return Err(CliError::Usage(
                "code-book path must end in .json or .toml".to_owned(),
            ));
        }
    };
    Ok(read_code_book(
        &read_bounded(path, MAX_CODE_BOOK_BYTES)?,
        format,
    )?)
}

fn load_symbol_catalog(paths: &[PathBuf]) -> Result<SymbolCatalog, CliError> {
    if paths.len() > MAX_CATALOG_PACKS {
        return Err(CliError::TooManySymbolPacks {
            actual: paths.len(),
            maximum: MAX_CATALOG_PACKS,
        });
    }
    let packs = paths
        .iter()
        .map(|path| {
            Ok(read_symbol_pack_json(&read_bounded(
                path,
                MAX_SYMBOL_PACK_BYTES,
            )?)?)
        })
        .collect::<Result<Vec<_>, CliError>>()?;
    Ok(SymbolCatalog::compile(packs)?)
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, CliError> {
    read_bounded_from(File::open(path)?, path, maximum)
}

fn read_bounded_from(reader: impl Read, path: &Path, maximum: usize) -> Result<Vec<u8>, CliError> {
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::with_capacity(maximum.min(64 * 1024));
    reader.take(limit).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(CliError::InputTooLarge {
            path: path.to_path_buf(),
            maximum,
        });
    }
    Ok(bytes)
}

fn print_utf8(bytes: &[u8]) -> Result<(), CliError> {
    print!("{}", std::str::from_utf8(bytes)?);
    Ok(())
}

fn print_help() {
    println!(
        "SurveyCraft CLI\n\n\
         Usage:\n  \
         surveycraft validate <points.csv> [--name NAME] [--project-id UUID]\n  \
         surveycraft import <points.csv> --output <project.json> [--name NAME] [--project-id UUID]\n  \
         surveycraft inspect <project.json>\n  \
         surveycraft codebook validate <book.json|book.toml>\n  \
         surveycraft codebook preview <book.json|book.toml> <code>\n  \
         surveycraft codebook generate <book.json|book.toml> <project.json> --output <plan.json> [--plan-scale SCALE] [--join-unmarked]\n  \
         surveycraft symbolpack validate <pack.json> [pack.json...]\n  \
         surveycraft symbolpack inspect <pack.json> <symbol-id>\n  \
         surveycraft symbolpack check-codebook <book.json|book.toml> <pack.json> [pack.json...]"
    );
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    schema_version: u32,
    id: Uuid,
    name: String,
    linear_unit: LinearUnit,
    crs_name: String,
    points: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SymbolDetails {
    attribution: SymbolAttribution,
    definition: SymbolDefinition,
}

#[cfg(test)]
mod tests {
    use super::{CliError, Command, parse_command, read_bounded_from};
    use std::error::Error;
    use std::io::Cursor;
    use std::path::Path;
    use uuid::Uuid;

    #[test]
    fn bounded_reader_stops_after_the_declared_limit() {
        let error = read_bounded_from(Cursor::new(vec![0_u8; 9]), Path::new("large.bin"), 8).err();
        assert!(matches!(
            error,
            Some(CliError::InputTooLarge { maximum: 8, .. })
        ));
    }

    #[test]
    fn parses_import_options() -> Result<(), Box<dyn Error>> {
        let id = Uuid::nil();
        let command = parse_command(&[
            "import".to_owned(),
            "points.csv".to_owned(),
            "--output".to_owned(),
            "project.json".to_owned(),
            "--project-id".to_owned(),
            id.to_string(),
        ])?;
        match command {
            Command::Import(options) => {
                assert_eq!(options.common.project_id, id);
                assert_eq!(options.output.to_string_lossy(), "project.json");
            }
            _ => return Err("expected import command".into()),
        }
        Ok(())
    }

    #[test]
    fn parses_code_book_generation_options() -> Result<(), Box<dyn Error>> {
        let command = parse_command(&[
            "codebook".to_owned(),
            "generate".to_owned(),
            "field.toml".to_owned(),
            "project.json".to_owned(),
            "--output".to_owned(),
            "plan.json".to_owned(),
            "--plan-scale".to_owned(),
            "500".to_owned(),
            "--join-unmarked".to_owned(),
        ])?;
        match command {
            Command::CodeBookGenerate(options) => {
                assert_eq!(options.plan_scale_denominator, 500.0);
                assert!(options.include_unmarked);
                assert_eq!(options.output.to_string_lossy(), "plan.json");
            }
            _ => return Err("expected code-book generation command".into()),
        }
        Ok(())
    }

    #[test]
    fn parses_symbol_pack_code_book_check() -> Result<(), Box<dyn Error>> {
        let command = parse_command(&[
            "symbolpack".to_owned(),
            "check-codebook".to_owned(),
            "field.json".to_owned(),
            "basic.symbol-pack.json".to_owned(),
        ])?;
        match command {
            Command::SymbolPackCheckCodeBook { book, packs } => {
                assert_eq!(book.to_string_lossy(), "field.json");
                assert_eq!(packs.len(), 1);
            }
            _ => return Err("expected symbol-pack code-book check".into()),
        }
        Ok(())
    }
}
