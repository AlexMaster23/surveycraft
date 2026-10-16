//! Bounded, deterministic interchange formats for SurveyCraft.

use csv::{ReaderBuilder, StringRecord, Terminator, Trim, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;
use surveycraft_coding::{
    CodeBook, CodingError, CompiledCodeBook, FeaturePlan, MAX_CODE_BOOK_BYTES,
};
use surveycraft_core::{
    CoreError, Crs, LinearUnit, NumericPolicy, QualityFlag, SourceRef, SurveyPoint, SurveyProject,
    deterministic_point_id, migrate_project,
};
use surveycraft_geometry::{CoordinateError, Point3};
use uuid::Uuid;

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ROWS: usize = 1_000_000;
pub const MAX_FIELD_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub enum IoError {
    InputTooLarge { actual: usize, maximum: usize },
    TooManyRows { maximum: usize },
    MissingHeader(&'static str),
    DuplicateHeader(String),
    Csv(csv::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    Utf8(Utf8Error),
    Coding(CodingError),
    Core(CoreError),
    UnknownPoint(Uuid),
}

impl Display for IoError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "input is {actual} bytes; limit is {maximum} bytes"
                )
            }
            Self::TooManyRows { maximum } => write!(formatter, "CSV exceeds {maximum} data rows"),
            Self::MissingHeader(header) => {
                write!(formatter, "required CSV header '{header}' is missing")
            }
            Self::DuplicateHeader(header) => {
                write!(formatter, "CSV header '{header}' appears more than once")
            }
            Self::Csv(error) => write!(formatter, "CSV error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Toml(error) => write!(formatter, "TOML error: {error}"),
            Self::Utf8(error) => write!(formatter, "input is not valid UTF-8: {error}"),
            Self::Coding(error) => write!(formatter, "code-book validation error: {error}"),
            Self::Core(error) => write!(formatter, "project validation error: {error}"),
            Self::UnknownPoint(id) => {
                write!(formatter, "selected point id {id} is not in the project")
            }
        }
    }
}

impl Error for IoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Toml(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Coding(error) => Some(error),
            Self::Core(error) => Some(error),
            _ => None,
        }
    }
}

impl From<csv::Error> for IoError {
    fn from(error: csv::Error) -> Self {
        Self::Csv(error)
    }
}

impl From<serde_json::Error> for IoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<toml::de::Error> for IoError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

impl From<Utf8Error> for IoError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

impl From<CodingError> for IoError {
    fn from(error: CodingError) -> Self {
        Self::Coding(error)
    }
}

impl From<CoreError> for IoError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

#[derive(Debug, Clone)]
pub struct CsvImportOptions {
    pub project_id: Uuid,
    pub project_name: String,
    pub source_name: String,
    pub crs: Crs,
    pub linear_unit: LinearUnit,
    pub numeric_policy: NumericPolicy,
    pub delimiter: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub row: usize,
    pub field: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub input_rows: usize,
    pub imported_points: usize,
    pub skipped_rows: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub project: SurveyProject,
    pub diagnostics: Vec<Diagnostic>,
    pub summary: ImportSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBookFormat {
    Json,
    Toml,
}

/// Imports point CSV v1 from an in-memory byte slice suitable for native and WASM callers.
pub fn import_point_csv(input: &[u8], options: &CsvImportOptions) -> Result<ImportReport, IoError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(IoError::InputTooLarge {
            actual: input.len(),
            maximum: MAX_INPUT_BYTES,
        });
    }

    let mut reader = ReaderBuilder::new()
        .delimiter(options.delimiter)
        .trim(Trim::All)
        .flexible(false)
        .from_reader(input);
    let headers = reader.headers()?.clone();
    let columns = Columns::from_headers(&headers)?;
    let mut project = SurveyProject::new(
        options.project_id,
        options.project_name.clone(),
        options.crs.clone(),
        options.linear_unit,
        options.numeric_policy,
    )?;
    let mut diagnostics = Vec::new();
    let mut input_rows = 0usize;
    let mut duplicates: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for record_result in reader.records() {
        input_rows += 1;
        if input_rows > MAX_ROWS {
            return Err(IoError::TooManyRows { maximum: MAX_ROWS });
        }
        let row = input_rows + 1;
        let record = match record_result {
            Ok(record) => record,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    row,
                    None,
                    "invalid-csv-row",
                    error.to_string(),
                ));
                continue;
            }
        };

        if let Some(field_index) = record
            .iter()
            .position(|field| field.len() > MAX_FIELD_BYTES)
        {
            let field = headers.get(field_index).map(str::to_owned);
            diagnostics.push(Diagnostic::error(
                row,
                field,
                "field-too-large",
                format!("field exceeds {MAX_FIELD_BYTES} bytes"),
            ));
            continue;
        }

        match parse_point(&record, &columns, options, row) {
            Ok(point) => {
                let index = project.points.len();
                duplicates
                    .entry(point.number.to_lowercase())
                    .or_default()
                    .push(index);
                project.add_point(point);
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    for indices in duplicates.values().filter(|indices| indices.len() > 1) {
        for index in indices {
            if let Some(point) = project.points.get_mut(*index) {
                point.add_quality_flag(QualityFlag::DuplicateNumber);
                diagnostics.push(Diagnostic::error(
                    point.source.row,
                    Some("number".to_owned()),
                    "duplicate-point-number",
                    format!("point number '{}' occurs more than once", point.number),
                ));
            }
        }
    }

    diagnostics.sort_by(|left, right| {
        left.row
            .cmp(&right.row)
            .then_with(|| left.code.cmp(&right.code))
    });
    project.validate()?;
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    let imported_points = project.points.len();
    Ok(ImportReport {
        project,
        diagnostics,
        summary: ImportSummary {
            input_rows,
            imported_points,
            skipped_rows: input_rows.saturating_sub(imported_points),
            warnings,
            errors,
        },
    })
}

pub fn read_project_json(input: &[u8]) -> Result<SurveyProject, IoError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(IoError::InputTooLarge {
            actual: input.len(),
            maximum: MAX_INPUT_BYTES,
        });
    }
    let project: SurveyProject = serde_json::from_slice(input)?;
    Ok(migrate_project(project)?)
}

pub fn write_project_json(project: &SurveyProject) -> Result<Vec<u8>, IoError> {
    project.validate()?;
    let mut output = serde_json::to_vec_pretty(project)?;
    output.push(b'\n');
    Ok(output)
}

pub fn write_report_json(report: &ImportReport) -> Result<Vec<u8>, IoError> {
    let mut output = serde_json::to_vec_pretty(report)?;
    output.push(b'\n');
    Ok(output)
}

pub fn read_code_book(input: &[u8], format: CodeBookFormat) -> Result<CodeBook, IoError> {
    if input.len() > MAX_CODE_BOOK_BYTES {
        return Err(IoError::InputTooLarge {
            actual: input.len(),
            maximum: MAX_CODE_BOOK_BYTES,
        });
    }
    let book: CodeBook = match format {
        CodeBookFormat::Json => serde_json::from_slice(input)?,
        CodeBookFormat::Toml => toml::from_str(std::str::from_utf8(input)?)?,
    };
    CompiledCodeBook::compile(book.clone())?;
    Ok(book)
}

pub fn write_code_book_json(book: &CodeBook) -> Result<Vec<u8>, IoError> {
    CompiledCodeBook::compile(book.clone())?;
    write_pretty_json(book)
}

pub fn write_feature_plan_json(plan: &FeaturePlan) -> Result<Vec<u8>, IoError> {
    write_pretty_json(plan)
}

fn write_pretty_json<T: Serialize>(value: &T) -> Result<Vec<u8>, IoError> {
    let mut output = serde_json::to_vec_pretty(value)?;
    output.push(b'\n');
    Ok(output)
}

/// Export point CSV v1 in a stable point-number order. An empty selection exports every point.
pub fn export_point_csv(
    project: &SurveyProject,
    selected: &BTreeSet<Uuid>,
) -> Result<Vec<u8>, IoError> {
    project.validate()?;
    if let Some(id) = selected
        .iter()
        .find(|id| !project.points.iter().any(|point| point.id == **id))
    {
        return Err(IoError::UnknownPoint(*id));
    }
    let mut points = project
        .points
        .iter()
        .filter(|point| selected.is_empty() || selected.contains(&point.id))
        .collect::<Vec<_>>();
    points.sort_by(|left, right| {
        compare_point_numbers(&left.number, &right.number)
            .then_with(|| left.number.cmp(&right.number))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut writer = WriterBuilder::new()
        .terminator(Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record([
        "number",
        "easting",
        "northing",
        "elevation",
        "code",
        "description",
    ])?;
    for point in points {
        writer.write_record([
            point.number.clone(),
            point.position.x().to_string(),
            point.position.y().to_string(),
            point.position.z().to_string(),
            point.code.clone().unwrap_or_default(),
            point.description.clone().unwrap_or_default(),
        ])?;
    }
    writer
        .into_inner()
        .map_err(|error| IoError::Csv(error.into_error().into()))
}

fn compare_point_numbers(left: &str, right: &str) -> Ordering {
    match (left.trim().parse::<i128>(), right.trim().parse::<i128>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.to_lowercase().cmp(&right.to_lowercase()),
    }
}

#[derive(Debug, Clone, Copy)]
struct Columns {
    number: usize,
    easting: usize,
    northing: usize,
    elevation: usize,
    code: Option<usize>,
    description: Option<usize>,
}

impl Columns {
    fn from_headers(headers: &StringRecord) -> Result<Self, IoError> {
        let mut positions: BTreeMap<String, usize> = BTreeMap::new();
        for (index, raw_header) in headers.iter().enumerate() {
            let header = raw_header.trim_start_matches('\u{feff}').to_lowercase();
            if positions.insert(header.clone(), index).is_some() {
                return Err(IoError::DuplicateHeader(header));
            }
        }
        Ok(Self {
            number: required_column(&positions, "number")?,
            easting: required_column(&positions, "easting")?,
            northing: required_column(&positions, "northing")?,
            elevation: required_column(&positions, "elevation")?,
            code: positions.get("code").copied(),
            description: positions.get("description").copied(),
        })
    }
}

fn required_column(
    positions: &BTreeMap<String, usize>,
    name: &'static str,
) -> Result<usize, IoError> {
    positions
        .get(name)
        .copied()
        .ok_or(IoError::MissingHeader(name))
}

fn parse_point(
    record: &StringRecord,
    columns: &Columns,
    options: &CsvImportOptions,
    row: usize,
) -> Result<SurveyPoint, Diagnostic> {
    let number = field(record, columns.number);
    if number.is_empty() {
        return Err(Diagnostic::error(
            row,
            Some("number".to_owned()),
            "missing-point-number",
            "point number is required".to_owned(),
        ));
    }
    let easting = parse_coordinate(record, columns.easting, row, "easting")?;
    let northing = parse_coordinate(record, columns.northing, row, "northing")?;
    let elevation = parse_coordinate(record, columns.elevation, row, "elevation")?;
    let position = Point3::new(easting, northing, elevation)
        .map_err(|error| coordinate_diagnostic(row, "position", error))?;
    let id = deterministic_point_id(options.project_id, row, number);
    SurveyPoint::new(
        id,
        number,
        position,
        optional_field(record, columns.code),
        optional_field(record, columns.description),
        SourceRef {
            source: options.source_name.clone(),
            row,
        },
    )
    .map_err(|error| {
        Diagnostic::error(
            row,
            Some("number".to_owned()),
            "invalid-point",
            error.to_string(),
        )
    })
}

fn parse_coordinate(
    record: &StringRecord,
    index: usize,
    row: usize,
    name: &'static str,
) -> Result<f64, Diagnostic> {
    let raw = field(record, index);
    let value = raw.parse::<f64>().map_err(|_| {
        Diagnostic::error(
            row,
            Some(name.to_owned()),
            "invalid-coordinate",
            format!("'{raw}' is not a decimal number"),
        )
    })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Diagnostic::error(
            row,
            Some(name.to_owned()),
            "non-finite-coordinate",
            "coordinate must be finite".to_owned(),
        ))
    }
}

fn field(record: &StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or_default().trim()
}

fn optional_field(record: &StringRecord, index: Option<usize>) -> Option<String> {
    let value = index
        .map(|column| field(record, column))
        .unwrap_or_default();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn coordinate_diagnostic(row: usize, field: &str, error: CoordinateError) -> Diagnostic {
    Diagnostic::error(
        row,
        Some(field.to_owned()),
        "invalid-coordinate",
        error.to_string(),
    )
}

impl Diagnostic {
    fn error(
        row: usize,
        field: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            row,
            field,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CsvImportOptions, export_point_csv, import_point_csv, read_project_json};
    use std::collections::BTreeSet;
    use std::error::Error;
    use surveycraft_core::{Crs, LinearUnit, NumericPolicy, QualityFlag};
    use uuid::Uuid;

    fn options() -> CsvImportOptions {
        CsvImportOptions {
            project_id: Uuid::nil(),
            project_name: "Test".to_owned(),
            source_name: "inline.csv".to_owned(),
            crs: Crs::local_engineering(),
            linear_unit: LinearUnit::Metre,
            numeric_policy: NumericPolicy::default(),
            delimiter: b',',
        }
    }

    #[test]
    fn imports_valid_points() -> Result<(), Box<dyn Error>> {
        let csv = b"number,easting,northing,elevation,code\n1,500,200,10,STN\n";
        let report = import_point_csv(csv, &options())?;
        assert_eq!(report.summary.imported_points, 1);
        assert_eq!(report.summary.errors, 0);
        assert_eq!(report.project.points[0].position.x(), 500.0);
        Ok(())
    }

    #[test]
    fn flags_duplicates_and_skips_bad_rows() -> Result<(), Box<dyn Error>> {
        let csv = b"number,easting,northing,elevation\n1,0,0,0\n1,1,1,1\n2,nope,2,2\n";
        let report = import_point_csv(csv, &options())?;
        assert_eq!(report.summary.input_rows, 3);
        assert_eq!(report.summary.imported_points, 2);
        assert_eq!(report.summary.skipped_rows, 1);
        assert_eq!(report.summary.errors, 3);
        assert!(
            report.project.points[0]
                .quality_flags
                .contains(&QualityFlag::DuplicateNumber)
        );
        Ok(())
    }

    #[test]
    fn reads_and_migrates_schema_one_projects() -> Result<(), Box<dyn Error>> {
        let legacy = br#"{
            "schemaVersion": 1,
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "Legacy",
            "crs": {"authority": null, "code": null, "name": "Local engineering grid"},
            "linearUnit": "metre",
            "numericPolicy": {
                "coordinateTolerance": 1e-8,
                "distanceTolerance": 1e-6,
                "areaTolerance": 1e-6,
                "displayDecimals": 3
            },
            "points": []
        }"#;

        let project = read_project_json(legacy)?;

        assert_eq!(
            project.schema_version,
            surveycraft_core::PROJECT_SCHEMA_VERSION
        );
        assert_eq!(project.styles.labels.number_layer, "SC-POINT-NUMBERS");
        Ok(())
    }

    #[test]
    fn exports_selected_points_in_deterministic_number_order() -> Result<(), Box<dyn Error>> {
        let csv = b"number,easting,northing,elevation,code,description\n10,10,20,30,B,Second\n2,1,2,3,A,First\n";
        let report = import_point_csv(csv, &options())?;
        let all = export_point_csv(&report.project, &BTreeSet::new())?;
        let text = String::from_utf8(all)?;
        assert_eq!(
            text,
            "number,easting,northing,elevation,code,description\r\n2,1,2,3,A,First\r\n10,10,20,30,B,Second\r\n"
        );

        let selected = BTreeSet::from([report.project.points[0].id]);
        let one = String::from_utf8(export_point_csv(&report.project, &selected)?)?;
        assert!(one.contains("10,10,20,30,B,Second"));
        assert!(!one.contains("2,1,2,3,A,First"));
        Ok(())
    }
}
