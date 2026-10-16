use std::error::Error;
use surveycraft_core::{Crs, LinearUnit, NumericPolicy};
use surveycraft_io::{CsvImportOptions, import_point_csv, read_project_json, write_project_json};
use uuid::Uuid;

const BASIC_CSV: &[u8] = include_bytes!("../../../tests/fixtures/points/basic.csv");
const INVALID_CSV: &[u8] = include_bytes!("../../../tests/fixtures/points/invalid.csv");
const BASIC_GOLDEN: &[u8] = include_bytes!("../../../tests/golden/basic.project.json");

fn options(source_name: &str) -> Result<CsvImportOptions, uuid::Error> {
    Ok(CsvImportOptions {
        project_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111")?,
        project_name: "Phase 0 Synthetic Survey".to_owned(),
        source_name: source_name.to_owned(),
        crs: Crs::local_engineering(),
        linear_unit: LinearUnit::Metre,
        numeric_policy: NumericPolicy::default(),
        delimiter: b',',
    })
}

#[test]
fn basic_csv_matches_the_committed_golden_project() -> Result<(), Box<dyn Error>> {
    let report = import_point_csv(BASIC_CSV, &options("basic.csv")?)?;
    assert_eq!(report.summary.errors, 0);
    assert_eq!(write_project_json(&report.project)?, BASIC_GOLDEN);

    let round_trip = read_project_json(BASIC_GOLDEN)?;
    assert_eq!(round_trip, report.project);
    Ok(())
}

#[test]
fn invalid_fixture_has_stable_diagnostics() -> Result<(), Box<dyn Error>> {
    let report = import_point_csv(INVALID_CSV, &options("invalid.csv")?)?;
    let diagnostic_codes: Vec<&str> = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert_eq!(report.summary.input_rows, 4);
    assert_eq!(report.summary.imported_points, 2);
    assert_eq!(report.summary.skipped_rows, 2);
    assert_eq!(report.summary.errors, 4);
    assert_eq!(
        diagnostic_codes,
        [
            "duplicate-point-number",
            "duplicate-point-number",
            "invalid-coordinate",
            "missing-point-number"
        ]
    );
    Ok(())
}
