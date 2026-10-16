//! Validated, CAD-independent vector symbol packs and placement geometry.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_coding::{CodeBook, GraphicRule, SymbolPlan};
use surveycraft_core::LinearUnit;
use surveycraft_geometry::Point3;
use uuid::Uuid;

pub const SYMBOL_PACK_SCHEMA_VERSION: u32 = 1;
pub const MAX_SYMBOL_PACK_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SYMBOLS_PER_PACK: usize = 512;
pub const MAX_PRIMITIVES_PER_SYMBOL: usize = 256;
pub const MAX_POINTS_PER_PRIMITIVE: usize = 1_024;
pub const MAX_CATALOG_PACKS: usize = 64;
pub const MAX_CATALOG_SYMBOLS: usize = 4_096;
const MAX_PRIMITIVES_PER_PACK: usize = 16_384;
const MAX_POINTS_PER_PACK: usize = 65_536;
const MAX_AUTHORS: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_NAME_BYTES: usize = 256;
const MAX_NOTICE_BYTES: usize = 2_048;
const MAX_LOCAL_COORDINATE: f64 = 10_000.0;
const MAX_STROKE_WIDTH_MM: f64 = 100.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    UnsupportedSchemaVersion(u32),
    TooManySymbols,
    InvalidAuthors,
    TooManyPrimitives(String),
    TooManyPrimitivePoints(String),
    TooManyPackPrimitives,
    TooManyPackPoints,
    TooManyCatalogPacks,
    TooManyCatalogSymbols,
    InvalidText(&'static str),
    InvalidColor(String),
    InvalidCoordinate(String),
    InvalidPrimitive {
        symbol: String,
        reason: &'static str,
    },
    DuplicatePackId(String),
    DuplicateSymbolId {
        symbol: String,
        first_pack: String,
        second_pack: String,
    },
    UnknownSymbol(String),
    MissingCodeBookSymbols(Vec<String>),
    InvalidPlacementScale,
    InvalidPlacementCoordinate,
}

impl Display for SymbolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported symbol-pack schema version {version}"
                )
            }
            Self::TooManySymbols => {
                write!(
                    formatter,
                    "symbol pack exceeds {MAX_SYMBOLS_PER_PACK} symbols"
                )
            }
            Self::InvalidAuthors => write!(
                formatter,
                "symbol pack must contain between 1 and {MAX_AUTHORS} authors"
            ),
            Self::TooManyPrimitives(symbol) => write!(
                formatter,
                "symbol '{symbol}' exceeds {MAX_PRIMITIVES_PER_SYMBOL} primitives"
            ),
            Self::TooManyPrimitivePoints(symbol) => write!(
                formatter,
                "symbol '{symbol}' has a primitive exceeding {MAX_POINTS_PER_PRIMITIVE} points"
            ),
            Self::TooManyPackPrimitives => write!(
                formatter,
                "symbol pack exceeds {MAX_PRIMITIVES_PER_PACK} total primitives"
            ),
            Self::TooManyPackPoints => write!(
                formatter,
                "symbol pack exceeds {MAX_POINTS_PER_PACK} total primitive points"
            ),
            Self::TooManyCatalogPacks => write!(
                formatter,
                "symbol catalog exceeds {MAX_CATALOG_PACKS} packs"
            ),
            Self::TooManyCatalogSymbols => write!(
                formatter,
                "symbol catalog exceeds {MAX_CATALOG_SYMBOLS} symbols"
            ),
            Self::InvalidText(field) => write!(formatter, "symbol-pack field '{field}' is invalid"),
            Self::InvalidColor(color) => write!(formatter, "invalid symbol color '{color}'"),
            Self::InvalidCoordinate(symbol) => {
                write!(
                    formatter,
                    "symbol '{symbol}' contains an invalid coordinate"
                )
            }
            Self::InvalidPrimitive { symbol, reason } => {
                write!(
                    formatter,
                    "symbol '{symbol}' has an invalid primitive: {reason}"
                )
            }
            Self::DuplicatePackId(id) => write!(formatter, "duplicate symbol-pack id '{id}'"),
            Self::DuplicateSymbolId {
                symbol,
                first_pack,
                second_pack,
            } => write!(
                formatter,
                "symbol '{symbol}' is defined by both '{first_pack}' and '{second_pack}'"
            ),
            Self::UnknownSymbol(id) => write!(formatter, "symbol '{id}' is not installed"),
            Self::MissingCodeBookSymbols(ids) => {
                write!(
                    formatter,
                    "code book requires missing symbols: {}",
                    ids.join(", ")
                )
            }
            Self::InvalidPlacementScale => {
                write!(
                    formatter,
                    "symbol placement scale must be finite and positive"
                )
            }
            Self::InvalidPlacementCoordinate => {
                write!(
                    formatter,
                    "symbol placement produced a non-finite coordinate"
                )
            }
        }
    }
}

impl Error for SymbolError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolPack {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: SymbolLicense,
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    pub symbols: Vec<SymbolDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolLicense {
    pub spdx: String,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub anchor: SymbolPoint,
    pub primitives: Vec<SymbolPrimitive>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SymbolPrimitive {
    Line {
        from: SymbolPoint,
        to: SymbolPoint,
        stroke_width_mm: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    Circle {
        center: SymbolPoint,
        radius: f64,
        stroke_width_mm: f64,
        #[serde(default)]
        filled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    Polyline {
        points: Vec<SymbolPoint>,
        #[serde(default)]
        closed: bool,
        stroke_width_mm: f64,
        #[serde(default)]
        filled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
}

#[derive(Debug)]
pub struct SymbolCatalog {
    packs: Vec<SymbolPack>,
    symbols: BTreeMap<String, (usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolAttribution {
    pub symbol_id: String,
    pub symbol_name: String,
    pub pack_id: String,
    pub pack_name: String,
    pub pack_version: String,
    pub spdx: String,
    pub notice: String,
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedSymbol {
    pub id: Uuid,
    pub point_id: Uuid,
    pub rule_id: String,
    pub symbol_id: String,
    pub pack_id: String,
    pub layer: Option<String>,
    pub insertion: Point3,
    pub scale: f64,
    pub rotation_degrees: f64,
    pub attribute: Option<String>,
    pub primitives: Vec<PlacedPrimitive>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum PlacedPrimitive {
    Line {
        from: Point3,
        to: Point3,
        stroke_width_mm: f64,
        color: Option<String>,
    },
    Circle {
        center: Point3,
        radius: f64,
        stroke_width_mm: f64,
        filled: bool,
        color: Option<String>,
    },
    Polyline {
        points: Vec<Point3>,
        closed: bool,
        stroke_width_mm: f64,
        filled: bool,
        color: Option<String>,
    },
}

impl SymbolCatalog {
    pub fn compile(packs: Vec<SymbolPack>) -> Result<Self, SymbolError> {
        if packs.len() > MAX_CATALOG_PACKS {
            return Err(SymbolError::TooManyCatalogPacks);
        }
        let total_symbols = packs
            .iter()
            .try_fold(0usize, |total, pack| total.checked_add(pack.symbols.len()));
        if total_symbols.is_none_or(|total| total > MAX_CATALOG_SYMBOLS) {
            return Err(SymbolError::TooManyCatalogSymbols);
        }
        let mut pack_ids = HashSet::with_capacity(packs.len());
        let mut symbols: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (pack_index, pack) in packs.iter().enumerate() {
            validate_pack(pack)?;
            if !pack_ids.insert(pack.id.clone()) {
                return Err(SymbolError::DuplicatePackId(pack.id.clone()));
            }
            for (symbol_index, symbol) in pack.symbols.iter().enumerate() {
                if let Some((first_pack_index, _)) = symbols.get(&symbol.id) {
                    return Err(SymbolError::DuplicateSymbolId {
                        symbol: symbol.id.clone(),
                        first_pack: packs[*first_pack_index].id.clone(),
                        second_pack: pack.id.clone(),
                    });
                }
                symbols.insert(symbol.id.clone(), (pack_index, symbol_index));
            }
        }
        Ok(Self { packs, symbols })
    }

    #[must_use]
    pub fn contains(&self, symbol_id: &str) -> bool {
        self.symbols.contains_key(symbol_id)
    }

    #[must_use]
    pub fn symbol_ids(&self) -> Vec<&str> {
        self.symbols.keys().map(String::as_str).collect()
    }

    #[must_use]
    pub fn definition(&self, symbol_id: &str) -> Option<&SymbolDefinition> {
        let (pack_index, symbol_index) = self.symbols.get(symbol_id)?;
        self.packs
            .get(*pack_index)
            .and_then(|pack| pack.symbols.get(*symbol_index))
    }

    #[must_use]
    pub fn attribution(&self, symbol_id: &str) -> Option<SymbolAttribution> {
        let (pack_index, symbol_index) = self.symbols.get(symbol_id)?;
        let pack = self.packs.get(*pack_index)?;
        let symbol = pack.symbols.get(*symbol_index)?;
        Some(SymbolAttribution {
            symbol_id: symbol.id.clone(),
            symbol_name: symbol.name.clone(),
            pack_id: pack.id.clone(),
            pack_name: pack.name.clone(),
            pack_version: pack.version.clone(),
            spdx: pack.license.spdx.clone(),
            notice: pack.license.notice.clone(),
            authors: pack.authors.clone(),
            homepage: pack.homepage.clone(),
        })
    }

    pub fn validate_code_book(&self, book: &CodeBook) -> Result<(), SymbolError> {
        let missing = book
            .rules
            .iter()
            .filter_map(|rule| match &rule.graphic {
                GraphicRule::Symbol { symbol, .. } if !self.contains(symbol) => {
                    Some(symbol.clone())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(SymbolError::MissingCodeBookSymbols(missing))
        }
    }

    pub fn place(
        &self,
        plan: &SymbolPlan,
        insertion: Point3,
        drawing_unit: LinearUnit,
    ) -> Result<PlacedSymbol, SymbolError> {
        if !plan.scale.is_finite() || plan.scale <= 0.0 {
            return Err(SymbolError::InvalidPlacementScale);
        }
        let (pack_index, symbol_index) = self
            .symbols
            .get(&plan.symbol)
            .copied()
            .ok_or_else(|| SymbolError::UnknownSymbol(plan.symbol.clone()))?;
        let pack = self
            .packs
            .get(pack_index)
            .ok_or_else(|| SymbolError::UnknownSymbol(plan.symbol.clone()))?;
        let symbol = pack
            .symbols
            .get(symbol_index)
            .ok_or_else(|| SymbolError::UnknownSymbol(plan.symbol.clone()))?;
        let factor = plan.scale / drawing_unit.metres_per_unit();
        if !factor.is_finite() || factor <= 0.0 {
            return Err(SymbolError::InvalidPlacementScale);
        }
        let radians = plan.rotation_degrees.to_radians();
        let sine = radians.sin();
        let cosine = radians.cos();
        let mut primitives = Vec::with_capacity(symbol.primitives.len());
        for primitive in &symbol.primitives {
            primitives.push(place_primitive(
                primitive,
                symbol.anchor,
                insertion,
                factor,
                sine,
                cosine,
            )?);
        }
        Ok(PlacedSymbol {
            id: plan.id,
            point_id: plan.point_id,
            rule_id: plan.rule_id.clone(),
            symbol_id: plan.symbol.clone(),
            pack_id: pack.id.clone(),
            layer: plan.layer.clone(),
            insertion,
            scale: plan.scale,
            rotation_degrees: plan.rotation_degrees,
            attribute: plan.attribute.clone(),
            primitives,
        })
    }
}

pub fn validate_pack(pack: &SymbolPack) -> Result<(), SymbolError> {
    if pack.schema_version != SYMBOL_PACK_SCHEMA_VERSION {
        return Err(SymbolError::UnsupportedSchemaVersion(pack.schema_version));
    }
    validate_id(&pack.id, "id")?;
    validate_text(&pack.name, MAX_NAME_BYTES, "name")?;
    validate_text(&pack.version, MAX_ID_BYTES, "version")?;
    validate_text(&pack.license.spdx, MAX_ID_BYTES, "license.spdx")?;
    validate_text(&pack.license.notice, MAX_NOTICE_BYTES, "license.notice")?;
    if pack.authors.is_empty() || pack.authors.len() > MAX_AUTHORS {
        return Err(SymbolError::InvalidAuthors);
    }
    for author in &pack.authors {
        validate_text(author, MAX_NAME_BYTES, "authors")?;
    }
    if let Some(homepage) = &pack.homepage {
        validate_text(homepage, MAX_NAME_BYTES, "homepage")?;
    }
    if pack.symbols.is_empty() || pack.symbols.len() > MAX_SYMBOLS_PER_PACK {
        return Err(SymbolError::TooManySymbols);
    }
    let total_primitives = pack.symbols.iter().try_fold(0usize, |total, symbol| {
        total.checked_add(symbol.primitives.len())
    });
    if total_primitives.is_none_or(|total| total > MAX_PRIMITIVES_PER_PACK) {
        return Err(SymbolError::TooManyPackPrimitives);
    }
    let total_points = pack.symbols.iter().try_fold(0usize, |total, symbol| {
        symbol
            .primitives
            .iter()
            .try_fold(total, |subtotal, primitive| {
                subtotal.checked_add(primitive_point_count(primitive))
            })
    });
    if total_points.is_none_or(|total| total > MAX_POINTS_PER_PACK) {
        return Err(SymbolError::TooManyPackPoints);
    }
    let mut ids = HashSet::with_capacity(pack.symbols.len());
    for symbol in &pack.symbols {
        validate_symbol(symbol)?;
        if !ids.insert(symbol.id.clone()) {
            return Err(SymbolError::DuplicateSymbolId {
                symbol: symbol.id.clone(),
                first_pack: pack.id.clone(),
                second_pack: pack.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_symbol(symbol: &SymbolDefinition) -> Result<(), SymbolError> {
    validate_id(&symbol.id, "symbols.id")?;
    validate_text(&symbol.name, MAX_NAME_BYTES, "symbols.name")?;
    validate_point(symbol.anchor, &symbol.id)?;
    if symbol.primitives.is_empty() || symbol.primitives.len() > MAX_PRIMITIVES_PER_SYMBOL {
        return Err(SymbolError::TooManyPrimitives(symbol.id.clone()));
    }
    for primitive in &symbol.primitives {
        validate_primitive(primitive, &symbol.id)?;
    }
    Ok(())
}

fn validate_primitive(primitive: &SymbolPrimitive, symbol: &str) -> Result<(), SymbolError> {
    match primitive {
        SymbolPrimitive::Line {
            from,
            to,
            stroke_width_mm,
            color,
        } => {
            validate_point(*from, symbol)?;
            validate_point(*to, symbol)?;
            validate_stroke(*stroke_width_mm, symbol)?;
            validate_color(color.as_deref())?;
            if from == to {
                return Err(SymbolError::InvalidPrimitive {
                    symbol: symbol.to_owned(),
                    reason: "line endpoints are equal",
                });
            }
        }
        SymbolPrimitive::Circle {
            center,
            radius,
            stroke_width_mm,
            color,
            ..
        } => {
            validate_point(*center, symbol)?;
            validate_stroke(*stroke_width_mm, symbol)?;
            validate_color(color.as_deref())?;
            if !radius.is_finite() || *radius <= 0.0 || *radius > MAX_LOCAL_COORDINATE {
                return Err(SymbolError::InvalidPrimitive {
                    symbol: symbol.to_owned(),
                    reason: "circle radius must be finite, positive and bounded",
                });
            }
        }
        SymbolPrimitive::Polyline {
            points,
            closed,
            filled,
            stroke_width_mm,
            color,
        } => {
            if points.len() > MAX_POINTS_PER_PRIMITIVE {
                return Err(SymbolError::TooManyPrimitivePoints(symbol.to_owned()));
            }
            let minimum = if *closed { 3 } else { 2 };
            if points.len() < minimum {
                return Err(SymbolError::InvalidPrimitive {
                    symbol: symbol.to_owned(),
                    reason: "polyline has too few points",
                });
            }
            if *filled && !closed {
                return Err(SymbolError::InvalidPrimitive {
                    symbol: symbol.to_owned(),
                    reason: "an open polyline cannot be filled",
                });
            }
            for point in points {
                validate_point(*point, symbol)?;
            }
            validate_stroke(*stroke_width_mm, symbol)?;
            validate_color(color.as_deref())?;
        }
    }
    Ok(())
}

const fn primitive_point_count(primitive: &SymbolPrimitive) -> usize {
    match primitive {
        SymbolPrimitive::Line { .. } => 2,
        SymbolPrimitive::Circle { .. } => 1,
        SymbolPrimitive::Polyline { points, .. } => points.len(),
    }
}

fn validate_point(point: SymbolPoint, symbol: &str) -> Result<(), SymbolError> {
    if point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= MAX_LOCAL_COORDINATE
        && point.y.abs() <= MAX_LOCAL_COORDINATE
    {
        Ok(())
    } else {
        Err(SymbolError::InvalidCoordinate(symbol.to_owned()))
    }
}

fn validate_stroke(value: f64, symbol: &str) -> Result<(), SymbolError> {
    if value.is_finite() && value > 0.0 && value <= MAX_STROKE_WIDTH_MM {
        Ok(())
    } else {
        Err(SymbolError::InvalidPrimitive {
            symbol: symbol.to_owned(),
            reason: "stroke width must be finite, positive and bounded",
        })
    }
}

fn validate_color(color: Option<&str>) -> Result<(), SymbolError> {
    let Some(color) = color else {
        return Ok(());
    };
    if color.len() == 7
        && color.starts_with('#')
        && color
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(SymbolError::InvalidColor(color.to_owned()))
    }
}

fn validate_id(value: &str, field: &'static str) -> Result<(), SymbolError> {
    if value.is_empty()
        || value.len() > MAX_ID_BYTES
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        Err(SymbolError::InvalidText(field))
    } else {
        Ok(())
    }
}

fn validate_text(value: &str, maximum: usize, field: &'static str) -> Result<(), SymbolError> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        Err(SymbolError::InvalidText(field))
    } else {
        Ok(())
    }
}

fn place_primitive(
    primitive: &SymbolPrimitive,
    anchor: SymbolPoint,
    insertion: Point3,
    factor: f64,
    sine: f64,
    cosine: f64,
) -> Result<PlacedPrimitive, SymbolError> {
    match primitive {
        SymbolPrimitive::Line {
            from,
            to,
            stroke_width_mm,
            color,
        } => Ok(PlacedPrimitive::Line {
            from: place_point(*from, anchor, insertion, factor, sine, cosine)?,
            to: place_point(*to, anchor, insertion, factor, sine, cosine)?,
            stroke_width_mm: *stroke_width_mm,
            color: color.clone(),
        }),
        SymbolPrimitive::Circle {
            center,
            radius,
            stroke_width_mm,
            filled,
            color,
        } => {
            let radius = radius * factor;
            if !radius.is_finite() || radius <= 0.0 {
                return Err(SymbolError::InvalidPlacementScale);
            }
            Ok(PlacedPrimitive::Circle {
                center: place_point(*center, anchor, insertion, factor, sine, cosine)?,
                radius,
                stroke_width_mm: *stroke_width_mm,
                filled: *filled,
                color: color.clone(),
            })
        }
        SymbolPrimitive::Polyline {
            points,
            closed,
            stroke_width_mm,
            filled,
            color,
        } => {
            let points = points
                .iter()
                .map(|point| place_point(*point, anchor, insertion, factor, sine, cosine))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PlacedPrimitive::Polyline {
                points,
                closed: *closed,
                stroke_width_mm: *stroke_width_mm,
                filled: *filled,
                color: color.clone(),
            })
        }
    }
}

fn place_point(
    point: SymbolPoint,
    anchor: SymbolPoint,
    insertion: Point3,
    factor: f64,
    sine: f64,
    cosine: f64,
) -> Result<Point3, SymbolError> {
    let local_x = (point.x - anchor.x) * factor;
    let local_y = (point.y - anchor.y) * factor;
    let x = insertion.x() + local_x * cosine - local_y * sine;
    let y = insertion.y() + local_x * sine + local_y * cosine;
    Point3::new(x, y, insertion.z()).map_err(|_| SymbolError::InvalidPlacementCoordinate)
}

#[cfg(test)]
mod tests {
    use super::{
        PlacedPrimitive, SymbolCatalog, SymbolDefinition, SymbolLicense, SymbolPack, SymbolPoint,
        SymbolPrimitive,
    };
    use std::error::Error;
    use surveycraft_coding::{
        CodeBook, CodeRule, DirectiveAliases, GraphicRule, MatchMode, SymbolPlan,
    };
    use surveycraft_core::LinearUnit;
    use surveycraft_geometry::Point3;
    use uuid::Uuid;

    fn pack() -> SymbolPack {
        SymbolPack {
            schema_version: 1,
            id: "surveycraft.test".to_owned(),
            name: "Synthetic symbols".to_owned(),
            version: "1.0.0".to_owned(),
            license: SymbolLicense {
                spdx: "MIT".to_owned(),
                notice: "Original synthetic test geometry.".to_owned(),
            },
            authors: vec!["SurveyCraft tests".to_owned()],
            homepage: None,
            symbols: vec![SymbolDefinition {
                id: "surveycraft.test-line".to_owned(),
                name: "Test line".to_owned(),
                anchor: SymbolPoint::default(),
                primitives: vec![SymbolPrimitive::Line {
                    from: SymbolPoint::default(),
                    to: SymbolPoint { x: 1.0, y: 0.0 },
                    stroke_width_mm: 0.2,
                    color: None,
                }],
            }],
        }
    }

    #[test]
    fn validates_catalog_and_reports_missing_code_book_symbols() -> Result<(), Box<dyn Error>> {
        let catalog = SymbolCatalog::compile(vec![pack()])?;
        assert!(catalog.contains("surveycraft.test-line"));
        let book = CodeBook {
            schema_version: 1,
            name: "Synthetic".to_owned(),
            case_sensitive: false,
            token_separators: vec!['-'],
            directives: DirectiveAliases::default(),
            layers: Vec::new(),
            rules: vec![CodeRule {
                id: "missing".to_owned(),
                pattern: "M".to_owned(),
                match_mode: MatchMode::Exact,
                description: "Missing symbol".to_owned(),
                optimization_rank: 1,
                layer: None,
                graphic: GraphicRule::Symbol {
                    symbol: "surveycraft.missing".to_owned(),
                    scale: 1.0,
                    attribute_from_suffix: false,
                },
            }],
        };
        assert!(catalog.validate_code_book(&book).is_err());
        Ok(())
    }

    #[test]
    fn places_vectors_with_scale_rotation_and_anchor() -> Result<(), Box<dyn Error>> {
        let catalog = SymbolCatalog::compile(vec![pack()])?;
        let plan = SymbolPlan {
            id: Uuid::nil(),
            point_id: Uuid::nil(),
            rule_id: "test".to_owned(),
            symbol: "surveycraft.test-line".to_owned(),
            layer: None,
            scale: 2.0,
            rotation_degrees: 90.0,
            attribute: None,
        };
        let placed = catalog.place(&plan, Point3::new(10.0, 20.0, 3.0)?, LinearUnit::Metre)?;
        let Some(PlacedPrimitive::Line { from, to, .. }) = placed.primitives.first() else {
            return Err("expected one placed line".into());
        };
        assert!((from.x() - 10.0).abs() < 1.0e-12);
        assert!((from.y() - 20.0).abs() < 1.0e-12);
        assert!((to.x() - 10.0).abs() < 1.0e-12);
        assert!((to.y() - 22.0).abs() < 1.0e-12);
        assert_eq!(to.z(), 3.0);
        Ok(())
    }

    #[test]
    fn rejects_duplicate_symbols_across_packs() {
        let first = pack();
        let mut second = pack();
        second.id = "surveycraft.second".to_owned();
        assert!(SymbolCatalog::compile(vec![first, second]).is_err());
    }
}
