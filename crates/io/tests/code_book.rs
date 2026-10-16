use std::error::Error;
use surveycraft_coding::{CompiledCodeBook, GenerateOptions, JoinDirective};
use surveycraft_io::{
    CodeBookFormat, read_code_book, read_project_json, write_code_book_json,
    write_feature_plan_json,
};

const JSON_BOOK: &[u8] = include_bytes!("../../../tests/fixtures/codebooks/synthetic-field.json");
const TOML_BOOK: &[u8] = include_bytes!("../../../tests/fixtures/codebooks/synthetic-field.toml");
const CODED_PROJECT: &[u8] = include_bytes!("../../../tests/fixtures/codebooks/coded.project.json");
const FEATURE_PLAN_GOLDEN: &[u8] = include_bytes!("../../../tests/golden/coded.feature-plan.json");

#[test]
fn json_and_toml_code_books_are_equivalent() -> Result<(), Box<dyn Error>> {
    let json = read_code_book(JSON_BOOK, CodeBookFormat::Json)?;
    let toml = read_code_book(TOML_BOOK, CodeBookFormat::Toml)?;
    assert_eq!(json, toml);

    let round_trip = read_code_book(&write_code_book_json(&json)?, CodeBookFormat::Json)?;
    assert_eq!(round_trip, json);
    Ok(())
}

#[test]
fn fixture_has_stable_case_insensitive_results() -> Result<(), Box<dyn Error>> {
    let book = CompiledCodeBook::compile(read_code_book(JSON_BOOK, CodeBookFormat::Json)?)?;
    let edge = book.interpret_code("edg-sta")?;
    assert_eq!(edge.primary_rule_id.as_deref(), Some("edge"));
    assert!(edge.directives.contains(&JoinDirective::Start));

    let marker = book.interpret_code("pin-17")?;
    assert_eq!(
        marker.symbol_attributes.get("pin").map(String::as_str),
        Some("17")
    );
    Ok(())
}

#[test]
fn coded_project_matches_the_committed_feature_plan() -> Result<(), Box<dyn Error>> {
    let book = CompiledCodeBook::compile(read_code_book(JSON_BOOK, CodeBookFormat::Json)?)?;
    let project = read_project_json(CODED_PROJECT)?;
    let plan = book.generate_features(
        project.id,
        &project.points,
        GenerateOptions {
            plan_scale_denominator: 2_000.0,
            coordinate_tolerance: project.numeric_policy.coordinate_tolerance,
            include_unmarked: false,
        },
    )?;
    assert_eq!(write_feature_plan_json(&plan)?, FEATURE_PLAN_GOLDEN);
    Ok(())
}
