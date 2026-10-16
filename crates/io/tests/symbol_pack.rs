use std::error::Error;
use surveycraft_coding::SymbolPlan;
use surveycraft_core::LinearUnit;
use surveycraft_geometry::Point3;
use surveycraft_io::{
    CodeBookFormat, read_code_book, read_symbol_pack_json, write_symbol_pack_json,
};
use surveycraft_symbols::{PlacedPrimitive, SymbolCatalog};
use uuid::Uuid;

const BASIC_PACK: &[u8] =
    include_bytes!("../../../assets/symbols/surveycraft-basic.symbol-pack.json");
const SYNTHETIC_CODE_BOOK: &[u8] =
    include_bytes!("../../../tests/fixtures/codebooks/synthetic-field.json");
const STARTER_CODE_BOOK: &[u8] =
    include_bytes!("../../../assets/codebooks/surveycraft-basic.code-book.json");

#[test]
fn bundled_pack_validates_and_round_trips() -> Result<(), Box<dyn Error>> {
    let pack = read_symbol_pack_json(BASIC_PACK)?;
    assert_eq!(pack.symbols.len(), 6);
    let round_trip = read_symbol_pack_json(&write_symbol_pack_json(&pack)?)?;
    assert_eq!(round_trip, pack);
    Ok(())
}

#[test]
fn bundled_tree_places_in_project_units() -> Result<(), Box<dyn Error>> {
    let catalog = SymbolCatalog::compile(vec![read_symbol_pack_json(BASIC_PACK)?])?;
    let plan = SymbolPlan {
        id: Uuid::nil(),
        point_id: Uuid::nil(),
        rule_id: "tree".to_owned(),
        symbol: "surveycraft.tree".to_owned(),
        layer: Some("symbols".to_owned()),
        scale: 2.0,
        rotation_degrees: 0.0,
        attribute: None,
    };
    let placed = catalog.place(
        &plan,
        Point3::new(1_000.0, 2_000.0, 3_000.0)?,
        LinearUnit::Millimetre,
    )?;
    let Some(PlacedPrimitive::Circle { radius, .. }) = placed.primitives.first() else {
        return Err("expected the tree crown circle".into());
    };
    assert!((*radius - 4_800.0).abs() < 1.0e-9);
    Ok(())
}

#[test]
fn bundled_pack_resolves_the_synthetic_code_book() -> Result<(), Box<dyn Error>> {
    let catalog = SymbolCatalog::compile(vec![read_symbol_pack_json(BASIC_PACK)?])?;
    let book = read_code_book(SYNTHETIC_CODE_BOOK, CodeBookFormat::Json)?;
    catalog.validate_code_book(&book)?;
    let starter = read_code_book(STARTER_CODE_BOOK, CodeBookFormat::Json)?;
    catalog.validate_code_book(&starter)?;
    Ok(())
}
