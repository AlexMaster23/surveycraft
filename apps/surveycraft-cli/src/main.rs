use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use surveycraft_core::{Crs, LinearUnit, NumericPolicy};
use surveycraft_io::{
    CsvImportOptions, ImportReport, import_point_csv, read_project_json, write_project_json,
    write_report_json,
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
            let bytes = fs::read(path)?;
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
        Command::Help => print_help(),
    }
    Ok(())
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(std::io::Error),
    SurveyIo(surveycraft_io::IoError),
    Json(serde_json::Error),
    Utf8(std::str::Utf8Error),
    InvalidUuid(uuid::Error),
    ValidationFailed(usize),
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => {
                write!(formatter, "{message}\nRun 'surveycraft --help' for usage.")
            }
            Self::Io(error) => write!(formatter, "file operation failed: {error}"),
            Self::SurveyIo(error) => Display::fmt(error, formatter),
            Self::Json(error) => write!(formatter, "could not produce JSON: {error}"),
            Self::Utf8(error) => write!(formatter, "internal JSON was not UTF-8: {error}"),
            Self::InvalidUuid(error) => write!(formatter, "invalid project UUID: {error}"),
            Self::ValidationFailed(count) => {
                write!(
                    formatter,
                    "validation found {count} error(s); no project was written"
                )
            }
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::SurveyIo(error) => Some(error),
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
        unknown => Err(CliError::Usage(format!("unknown command '{unknown}'"))),
    }
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
    let bytes = fs::read(&options.input)?;
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
         surveycraft inspect <project.json>"
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

#[cfg(test)]
mod tests {
    use super::{Command, parse_command};
    use std::error::Error;
    use uuid::Uuid;

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
}
