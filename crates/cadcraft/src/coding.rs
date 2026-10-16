//! CADCraft adapter for code-book editing, preview and transactional feature generation.

use super::*;
use surveycraft_coding::{
    CodeBook, CodeRule, CompiledCodeBook, DiagnosticSeverity, FeatureSegment, GenerateOptions,
    GraphicRule, MatchMode,
};
use surveycraft_io::{CodeBookFormat, read_code_book, read_symbol_pack_json, write_code_book_json};
use surveycraft_symbols::{PlacedPrimitive, SymbolCatalog};

const CODE_BOOK_METADATA_KEY: &str = "code-book";
pub(crate) const FEATURE_SET_METADATA_KEY: &str = "coding-feature-set";
pub(crate) const FEATURE_SET_METADATA_VALUE: &str = "v1";
const FEATURE_ID_METADATA_KEY: &str = "feature-id";
const RULE_ID_METADATA_KEY: &str = "rule-id";
const SOURCE_POINTS_METADATA_KEY: &str = "source-point-ids";
const MAX_RULE_EDITS: usize = 512;
const MAX_GENERATED_ENTITIES: usize = 4_000;
const MAX_RESULT_DIAGNOSTICS: usize = 200;
const STARTER_CODE_BOOK: &[u8] =
    include_bytes!("../../../assets/codebooks/surveycraft-basic.code-book.json");
const BASIC_SYMBOL_PACK: &[u8] =
    include_bytes!("../../../assets/symbols/surveycraft-basic.symbol-pack.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CodeBookManagerRequest {
    List,
    Preview {
        rows: Vec<CodeRuleEdit>,
    },
    Update {
        rows: Vec<CodeRuleEdit>,
    },
    Delete {
        ids: Vec<String>,
    },
    Export {
        #[serde(default)]
        ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CodeRuleEdit {
    pub record: String,
    pub rule_id: String,
    pub pattern: String,
    pub match_mode: String,
    pub description: String,
    pub optimization_rank: f64,
    #[serde(default)]
    pub layer: String,
    pub graphic_kind: String,
    #[serde(default)]
    pub style_or_symbol: String,
    pub scale_or_width: f64,
    #[serde(default)]
    pub attribute_from_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodeRuleRow {
    pub record: String,
    pub entities: Vec<u64>,
    pub rule_id: String,
    pub pattern: String,
    pub match_mode: String,
    pub description: String,
    pub optimization_rank: u8,
    pub layer: String,
    pub graphic_kind: String,
    pub style_or_symbol: String,
    pub scale_or_width: f64,
    pub attribute_from_suffix: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookManagerSnapshot {
    pub rows: Vec<CodeRuleRow>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookManagerMutation {
    pub message: String,
    pub affected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookManagerExport {
    pub file_name: String,
    pub content: String,
    pub rules: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CodeBookManagerResult {
    Snapshot(CodeBookManagerSnapshot),
    Mutation(CodeBookManagerMutation),
    Export(CodeBookManagerExport),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateCodedFeaturesRequest {
    #[serde(default = "default_plan_scale")]
    pub plan_scale_denominator: f64,
    #[serde(default)]
    pub include_unmarked: bool,
}

impl Default for GenerateCodedFeaturesRequest {
    fn default() -> Self {
        Self {
            plan_scale_denominator: default_plan_scale(),
            include_unmarked: false,
        }
    }
}

const fn default_plan_scale() -> f64 {
    1_000.0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedDiagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCodedFeaturesResult {
    pub paths: usize,
    pub circles: usize,
    pub symbols: usize,
    pub entities: usize,
    pub replaced_entities: usize,
    pub diagnostics: Vec<GeneratedDiagnostic>,
    pub omitted_diagnostics: usize,
}

/// Load, preview or mutate the drawing's persisted code book through CADCraft's native manager.
pub fn manage_code_book(
    host: &mut impl CadcraftHost,
    request: &CodeBookManagerRequest,
) -> Result<CodeBookManagerResult, AdapterError> {
    match request {
        CodeBookManagerRequest::List => {
            let book = load_code_book(host)?;
            let snapshot = code_book_snapshot(host, &book)?;
            commit_read_only(
                host,
                "List SurveyCraft code rules",
                "complete code-rule listing",
            )?;
            Ok(CodeBookManagerResult::Snapshot(snapshot))
        }
        CodeBookManagerRequest::Preview { rows } => {
            let current = load_code_book(host)?;
            let candidate = replace_rules(&current, rows)
                .map_err(|error| AdapterError::new("preview code rules", error, None))?;
            validate_code_book(&candidate)
                .map_err(|error| AdapterError::new("preview code rules", error, None))?;
            let mut snapshot = code_book_snapshot(host, &candidate)?;
            for (row, edit) in snapshot.rows.iter_mut().zip(rows) {
                row.record = edit.record.clone();
            }
            commit_read_only(
                host,
                "Preview SurveyCraft code rules",
                "complete code-rule preview",
            )?;
            Ok(CodeBookManagerResult::Snapshot(snapshot))
        }
        CodeBookManagerRequest::Update { rows } => {
            if rows.is_empty() || rows.len() > MAX_RULE_EDITS {
                return Err(AdapterError::new(
                    "update code rules",
                    HostError::new(format!(
                        "the request must contain 1 to {MAX_RULE_EDITS} rule edits"
                    )),
                    None,
                ));
            }
            let mut book = load_code_book(host)?;
            apply_rule_edits(&mut book, rows)
                .map_err(|error| AdapterError::new("update code rules", error, None))?;
            validate_code_book(&book)
                .map_err(|error| AdapterError::new("update code rules", error, None))?;
            persist_code_book(host, &book, "Update SurveyCraft code rules")?;
            Ok(CodeBookManagerResult::Mutation(CodeBookManagerMutation {
                message: format!(
                    "Saved {} code rule{}",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                ),
                affected: rows.len(),
            }))
        }
        CodeBookManagerRequest::Delete { ids } => {
            if ids.is_empty() || ids.len() > MAX_RULE_EDITS {
                return Err(AdapterError::new(
                    "delete code rules",
                    HostError::new(format!(
                        "the request must contain 1 to {MAX_RULE_EDITS} rule ids"
                    )),
                    None,
                ));
            }
            let selected = ids.iter().map(|id| id.trim()).collect::<BTreeSet<_>>();
            if selected.len() != ids.len() {
                return Err(AdapterError::new(
                    "delete code rules",
                    HostError::new("a code-rule id occurs more than once"),
                    None,
                ));
            }
            let mut book = load_code_book(host)?;
            let before = book.rules.len();
            book.rules
                .retain(|rule| !selected.contains(rule.id.as_str()));
            let removed = before.saturating_sub(book.rules.len());
            if removed != selected.len() {
                return Err(AdapterError::new(
                    "delete code rules",
                    HostError::new("one or more code-rule ids do not exist"),
                    None,
                ));
            }
            validate_code_book(&book)
                .map_err(|error| AdapterError::new("delete code rules", error, None))?;
            persist_code_book(host, &book, "Delete SurveyCraft code rules")?;
            Ok(CodeBookManagerResult::Mutation(CodeBookManagerMutation {
                message: format!(
                    "Deleted {removed} code rule{}",
                    if removed == 1 { "" } else { "s" }
                ),
                affected: removed,
            }))
        }
        CodeBookManagerRequest::Export { ids } => {
            if ids.len() > MAX_RULE_EDITS {
                return Err(AdapterError::new(
                    "export code book",
                    HostError::new(format!(
                        "the request cannot contain more than {MAX_RULE_EDITS} rule ids"
                    )),
                    None,
                ));
            }
            let mut book = load_code_book(host)?;
            if !ids.is_empty() {
                let selected = ids.iter().map(|id| id.trim()).collect::<BTreeSet<_>>();
                if selected.len() != ids.len() {
                    return Err(AdapterError::new(
                        "export code book",
                        HostError::new("a code-rule id occurs more than once"),
                        None,
                    ));
                }
                book.rules
                    .retain(|rule| selected.contains(rule.id.as_str()));
                if book.rules.len() != selected.len() {
                    return Err(AdapterError::new(
                        "export code book",
                        HostError::new("one or more code-rule ids do not exist"),
                        None,
                    ));
                }
            }
            let bytes = write_code_book_json(&book).map_err(|error| {
                AdapterError::new("export code book", HostError::new(error.to_string()), None)
            })?;
            let content = String::from_utf8(bytes).map_err(|error| {
                AdapterError::new("export code book", HostError::new(error.to_string()), None)
            })?;
            commit_read_only(
                host,
                "Export SurveyCraft code book",
                "complete code-book export",
            )?;
            Ok(CodeBookManagerResult::Export(CodeBookManagerExport {
                file_name: "surveycraft-code-book.json".into(),
                content,
                rules: book.rules.len(),
            }))
        }
    }
}

/// Replace prior generated coding output and create the new editable linework in one transaction.
pub fn generate_coded_features(
    host: &mut impl CadcraftHost,
    request: GenerateCodedFeaturesRequest,
) -> Result<GenerateCodedFeaturesResult, AdapterError> {
    let stored = required_drawing_project(host)?;
    let book = load_code_book(host)?;
    let compiled = CompiledCodeBook::compile(book.clone()).map_err(|error| {
        AdapterError::new("compile code book", HostError::new(error.to_string()), None)
    })?;
    let catalog =
        symbol_catalog().map_err(|error| AdapterError::new("load symbol catalog", error, None))?;
    catalog.validate_code_book(&book).map_err(|error| {
        AdapterError::new(
            "validate symbol references",
            HostError::new(error.to_string()),
            None,
        )
    })?;
    let plan = compiled
        .generate_features(
            stored.project.id,
            &stored.project.points,
            GenerateOptions {
                plan_scale_denominator: request.plan_scale_denominator,
                include_unmarked: request.include_unmarked,
                coordinate_tolerance: stored.project.numeric_policy.coordinate_tolerance,
            },
        )
        .map_err(|error| {
            AdapterError::new(
                "plan coded features",
                HostError::new(error.to_string()),
                None,
            )
        })?;
    let projected_entities = projected_entity_count(&plan, &catalog, &stored.project)?;
    if projected_entities > MAX_GENERATED_ENTITIES {
        return Err(AdapterError::new(
            "plan coded features",
            HostError::new(format!(
                "coded output requires {projected_entities} entities; the release limit is {MAX_GENERATED_ENTITIES}"
            )),
            None,
        ));
    }
    let existing = host
        .find_entities_by_metadata(FEATURE_SET_METADATA_KEY, FEATURE_SET_METADATA_VALUE)
        .map_err(|error| AdapterError::new("find prior coded features", error, None))?;
    if existing.len().saturating_add(projected_entities) > MAX_GENERATED_ENTITIES.saturating_mul(2)
    {
        return Err(AdapterError::new(
            "replace coded features",
            HostError::new("the replacement would exceed the bounded transaction size"),
            None,
        ));
    }
    let transaction = host
        .begin_transaction("Generate SurveyCraft coded features")
        .map_err(|error| AdapterError::new("begin coded-feature transaction", error, None))?;
    for entity in &existing {
        host.delete_entity(transaction, entity.id)
            .map_err(|error| {
                rollback_error(host, transaction, "delete prior coded feature", error)
            })?;
    }
    let point_positions = stored
        .project
        .points
        .iter()
        .map(|point| (point.id, point.position))
        .collect::<BTreeMap<_, _>>();
    let mut created = 0usize;
    for path in &plan.paths {
        let (layer, layer_color) = drawing_layer_appearance(&book, path.layer.as_deref());
        for segment in &path.segments {
            let (entity, source_ids) = match segment {
                FeatureSegment::Line { start, end } => {
                    let first = required_position_in_transaction(
                        host,
                        transaction,
                        &point_positions,
                        *start,
                    )?;
                    let second = required_position_in_transaction(
                        host,
                        transaction,
                        &point_positions,
                        *end,
                    )?;
                    let entity = host
                        .create_line(
                            transaction,
                            &LineRequest {
                                start: first,
                                end: second,
                                layer: layer.clone(),
                            },
                        )
                        .map_err(|error| {
                            rollback_error(host, transaction, "create coded line", error)
                        })?;
                    (entity, vec![*start, *end])
                }
                FeatureSegment::Arc {
                    start,
                    through,
                    end,
                } => {
                    let first = required_position_in_transaction(
                        host,
                        transaction,
                        &point_positions,
                        *start,
                    )?;
                    let middle = required_position_in_transaction(
                        host,
                        transaction,
                        &point_positions,
                        *through,
                    )?;
                    let last = required_position_in_transaction(
                        host,
                        transaction,
                        &point_positions,
                        *end,
                    )?;
                    let arc = arc_request(first, middle, last, layer.clone()).map_err(|error| {
                        rollback_error(host, transaction, "prepare coded arc", error)
                    })?;
                    let entity = host.create_arc(transaction, &arc).map_err(|error| {
                        rollback_error(host, transaction, "create coded arc", error)
                    })?;
                    (entity, vec![*start, *through, *end])
                }
            };
            apply_entity_appearance(
                host,
                transaction,
                entity,
                layer_color.clone(),
                Some(path.paper_width_mm),
                "style coded path",
            )?;
            tag_generated_entity(
                host,
                transaction,
                entity,
                path.id,
                &path.rule_id,
                &source_ids,
                "path",
            )?;
            created = created.saturating_add(1);
        }
    }
    for circle in &plan.circles {
        let points = circle
            .points
            .iter()
            .map(|id| required_position_in_transaction(host, transaction, &point_positions, *id))
            .collect::<Result<Vec<_>, _>>()?;
        let Some([first, second, third]) = points.as_slice().get(0..3) else {
            return Err(rollback_error(
                host,
                transaction,
                "prepare coded circle",
                HostError::new("circle plan does not contain three points"),
            ));
        };
        let (layer, layer_color) = drawing_layer_appearance(&book, circle.layer.as_deref());
        let request = circle_request(*first, *second, *third, layer)
            .map_err(|error| rollback_error(host, transaction, "prepare coded circle", error))?;
        let entity = host
            .create_circle(transaction, &request)
            .map_err(|error| rollback_error(host, transaction, "create coded circle", error))?;
        apply_entity_appearance(
            host,
            transaction,
            entity,
            layer_color,
            Some(circle.paper_width_mm),
            "style coded circle",
        )?;
        tag_generated_entity(
            host,
            transaction,
            entity,
            circle.id,
            &circle.rule_id,
            &circle.points,
            "circle",
        )?;
        created = created.saturating_add(1);
    }
    for symbol in &plan.symbols {
        let insertion =
            required_position_in_transaction(host, transaction, &point_positions, symbol.point_id)?;
        let placed = catalog
            .place(symbol, insertion, stored.project.linear_unit)
            .map_err(|error| {
                rollback_error(
                    host,
                    transaction,
                    "place coded symbol",
                    HostError::new(error.to_string()),
                )
            })?;
        let (layer, layer_color) = drawing_layer_appearance(&book, symbol.layer.as_deref());
        for primitive in &placed.primitives {
            let entities = create_symbol_primitive(
                host,
                transaction,
                primitive,
                &layer,
                layer_color.as_deref(),
            )?;
            for entity in entities {
                tag_generated_entity(
                    host,
                    transaction,
                    entity,
                    symbol.id,
                    &symbol.rule_id,
                    &[symbol.point_id],
                    "symbol",
                )?;
                created = created.saturating_add(1);
            }
        }
    }
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(
            "commit coded-feature transaction",
            error,
            rollback_error,
        ));
    }
    let diagnostic_count = plan.diagnostics.len();
    let diagnostics = plan
        .diagnostics
        .iter()
        .take(MAX_RESULT_DIAGNOSTICS)
        .map(|diagnostic| GeneratedDiagnostic {
            severity: match diagnostic.severity {
                DiagnosticSeverity::Warning => "warning".into(),
                DiagnosticSeverity::Error => "error".into(),
            },
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            point_id: diagnostic.point_id,
        })
        .collect::<Vec<_>>();
    Ok(GenerateCodedFeaturesResult {
        paths: plan.paths.len(),
        circles: plan.circles.len(),
        symbols: plan.symbols.len(),
        entities: created,
        replaced_entities: existing.len(),
        omitted_diagnostics: diagnostic_count.saturating_sub(diagnostics.len()),
        diagnostics,
    })
}

fn load_code_book(host: &mut impl CadcraftHost) -> Result<CodeBook, AdapterError> {
    let stored = host
        .get_drawing_metadata(CODE_BOOK_METADATA_KEY)
        .map_err(|error| AdapterError::new("read code-book metadata", error, None))?;
    let input = stored.as_deref().map_or(STARTER_CODE_BOOK, str::as_bytes);
    read_code_book(input, CodeBookFormat::Json).map_err(|error| {
        AdapterError::new("read code book", HostError::new(error.to_string()), None)
    })
}

fn symbol_catalog() -> Result<SymbolCatalog, HostError> {
    let pack = read_symbol_pack_json(BASIC_SYMBOL_PACK)
        .map_err(|error| HostError::new(error.to_string()))?;
    SymbolCatalog::compile(vec![pack]).map_err(|error| HostError::new(error.to_string()))
}

fn validate_code_book(book: &CodeBook) -> Result<(), HostError> {
    CompiledCodeBook::compile(book.clone()).map_err(|error| HostError::new(error.to_string()))?;
    symbol_catalog()?
        .validate_code_book(book)
        .map_err(|error| HostError::new(error.to_string()))
}

fn persist_code_book(
    host: &mut impl CadcraftHost,
    book: &CodeBook,
    label: &str,
) -> Result<(), AdapterError> {
    let bytes = write_code_book_json(book).map_err(|error| {
        AdapterError::new("encode code book", HostError::new(error.to_string()), None)
    })?;
    let value = String::from_utf8(bytes).map_err(|error| {
        AdapterError::new("encode code book", HostError::new(error.to_string()), None)
    })?;
    let transaction = host
        .begin_transaction(label)
        .map_err(|error| AdapterError::new("begin code-book transaction", error, None))?;
    host.set_drawing_metadata(transaction, CODE_BOOK_METADATA_KEY, &value)
        .map_err(|error| rollback_error(host, transaction, "persist code book", error))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(
            "commit code-book transaction",
            error,
            rollback_error,
        ));
    }
    Ok(())
}

fn replace_rules(book: &CodeBook, rows: &[CodeRuleEdit]) -> Result<CodeBook, HostError> {
    if rows.is_empty() || rows.len() > MAX_RULE_EDITS {
        return Err(HostError::new(format!(
            "the preview must contain 1 to {MAX_RULE_EDITS} code rules"
        )));
    }
    let mut candidate = book.clone();
    candidate.rules = rows
        .iter()
        .map(rule_from_edit)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(candidate)
}

fn apply_rule_edits(book: &mut CodeBook, rows: &[CodeRuleEdit]) -> Result<(), HostError> {
    let existing_ids = book
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut replacements = BTreeMap::new();
    let mut additions = Vec::new();
    for edit in rows {
        let rule = rule_from_edit(edit)?;
        if edit.record.starts_with("__cadcraft_new_") {
            additions.push(rule);
        } else {
            if !existing_ids.contains(edit.record.as_str()) {
                return Err(HostError::new(format!(
                    "code rule '{}' does not exist",
                    edit.record
                )));
            }
            if replacements.insert(edit.record.clone(), rule).is_some() {
                return Err(HostError::new(format!(
                    "code rule '{}' occurs more than once in the edit request",
                    edit.record
                )));
            }
        }
    }
    for existing in &mut book.rules {
        if let Some(replacement) = replacements.remove(&existing.id) {
            *existing = replacement;
        }
    }
    book.rules.extend(additions);
    Ok(())
}

fn rule_from_edit(edit: &CodeRuleEdit) -> Result<CodeRule, HostError> {
    let match_mode = match edit.match_mode.trim().to_ascii_lowercase().as_str() {
        "exact" => MatchMode::Exact,
        "prefix" => MatchMode::Prefix,
        "regex" => MatchMode::Regex,
        _ => return Err(HostError::new("match mode must be exact, prefix or regex")),
    };
    if !edit.optimization_rank.is_finite()
        || edit.optimization_rank.fract() != 0.0
        || !(0.0..=255.0).contains(&edit.optimization_rank)
    {
        return Err(HostError::new(
            "optimization rank must be an integer from 0 to 255",
        ));
    }
    if !edit.scale_or_width.is_finite() || edit.scale_or_width <= 0.0 {
        return Err(HostError::new("scale or width must be finite and positive"));
    }
    let graphic = match edit.graphic_kind.trim().to_ascii_lowercase().as_str() {
        "none" => GraphicRule::None,
        "line" => GraphicRule::Line {
            style: edit.style_or_symbol.trim().to_owned(),
            paper_width_mm: edit.scale_or_width,
        },
        "symbol" => GraphicRule::Symbol {
            symbol: edit.style_or_symbol.trim().to_owned(),
            scale: edit.scale_or_width,
            attribute_from_suffix: parse_boolean(&edit.attribute_from_suffix)?,
        },
        _ => return Err(HostError::new("graphic kind must be none, line or symbol")),
    };
    Ok(CodeRule {
        id: edit.rule_id.trim().to_owned(),
        pattern: edit.pattern.trim().to_owned(),
        match_mode,
        description: edit.description.trim().to_owned(),
        optimization_rank: edit.optimization_rank as u8,
        layer: nonempty(edit.layer.trim()),
        graphic,
    })
}

fn nonempty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn parse_boolean(value: &str) -> Result<bool, HostError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "false" | "no" | "0" => Ok(false),
        "true" | "yes" | "1" => Ok(true),
        _ => Err(HostError::new("attribute from suffix must be yes or no")),
    }
}

fn code_book_snapshot(
    host: &mut impl CadcraftHost,
    book: &CodeBook,
) -> Result<CodeBookManagerSnapshot, AdapterError> {
    let compiled = CompiledCodeBook::compile(book.clone()).map_err(|error| {
        AdapterError::new(
            "compile code-book preview",
            HostError::new(error.to_string()),
            None,
        )
    })?;
    let catalog =
        symbol_catalog().map_err(|error| AdapterError::new("load symbol catalog", error, None))?;
    catalog.validate_code_book(book).map_err(|error| {
        AdapterError::new(
            "validate code-book symbols",
            HostError::new(error.to_string()),
            None,
        )
    })?;
    let points = load_drawing_project(host)?
        .map(|stored| stored.project.points)
        .unwrap_or_default();
    let mut matches = BTreeMap::<String, usize>::new();
    let mut warnings = 0usize;
    let mut errors = 0usize;
    let mut diagnostic_examples = Vec::new();
    let coded = points
        .iter()
        .filter_map(|point| {
            point
                .code
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(|code| (point.number.as_str(), code))
        })
        .collect::<Vec<_>>();
    let interpretations = compiled
        .interpret_codes(coded.iter().map(|(_, code)| *code))
        .map_err(|error| {
            AdapterError::new(
                "preview point code",
                HostError::new(error.to_string()),
                None,
            )
        })?;
    let coded_points = coded.len();
    for ((point_number, _), interpretation) in coded.into_iter().zip(interpretations) {
        for matched in interpretation.matches {
            *matches.entry(matched.rule_id).or_default() += 1;
        }
        for diagnostic in interpretation.diagnostics {
            match diagnostic.severity {
                DiagnosticSeverity::Warning => warnings = warnings.saturating_add(1),
                DiagnosticSeverity::Error => errors = errors.saturating_add(1),
            }
            if diagnostic_examples.len() < 3 {
                diagnostic_examples.push(format!("{point_number}: {}", diagnostic.message));
            }
        }
    }
    let rows = book
        .rules
        .iter()
        .map(|rule| {
            let (graphic_kind, style_or_symbol, scale_or_width, attribute_from_suffix) =
                match &rule.graphic {
                    GraphicRule::None => ("none".to_owned(), String::new(), 1.0, false),
                    GraphicRule::Line {
                        style,
                        paper_width_mm,
                    } => ("line".to_owned(), style.clone(), *paper_width_mm, false),
                    GraphicRule::Symbol {
                        symbol,
                        scale,
                        attribute_from_suffix,
                    } => (
                        "symbol".to_owned(),
                        symbol.clone(),
                        *scale,
                        *attribute_from_suffix,
                    ),
                };
            let matched = matches.get(&rule.id).copied().unwrap_or_default();
            CodeRuleRow {
                record: rule.id.clone(),
                entities: Vec::new(),
                rule_id: rule.id.clone(),
                pattern: rule.pattern.clone(),
                match_mode: match rule.match_mode {
                    MatchMode::Exact => "exact",
                    MatchMode::Prefix => "prefix",
                    MatchMode::Regex => "regex",
                }
                .into(),
                description: rule.description.clone(),
                optimization_rank: rule.optimization_rank,
                layer: rule.layer.clone().unwrap_or_default(),
                graphic_kind,
                style_or_symbol,
                scale_or_width,
                attribute_from_suffix: if attribute_from_suffix { "yes" } else { "no" }.into(),
                status: format!("{matched} point{}", if matched == 1 { "" } else { "s" }),
            }
        })
        .collect();
    let mut summary = format!(
        "{} rules · {coded_points} coded points · {warnings} warnings · {errors} errors",
        book.rules.len()
    );
    if !diagnostic_examples.is_empty() {
        summary.push_str(" · ");
        summary.push_str(&diagnostic_examples.join(" | "));
    }
    if summary.len() > 480 {
        let boundary = summary.floor_char_boundary(480);
        summary.truncate(boundary);
        summary.push('…');
    }
    Ok(CodeBookManagerSnapshot { rows, summary })
}

fn drawing_layer_appearance(book: &CodeBook, layer_id: Option<&str>) -> (String, Option<String>) {
    layer_id
        .and_then(|id| book.layers.iter().find(|layer| layer.id == id))
        .map_or_else(
            || ("0".to_owned(), None),
            |layer| (layer.name.clone(), layer.color.clone()),
        )
}

fn required_position(positions: &BTreeMap<Uuid, Point3>, id: Uuid) -> Result<Point3, AdapterError> {
    positions.get(&id).copied().ok_or_else(|| {
        AdapterError::new(
            "resolve coded-feature point",
            HostError::new(format!("feature references missing point {id}")),
            None,
        )
    })
}

fn required_position_in_transaction(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    positions: &BTreeMap<Uuid, Point3>,
    id: Uuid,
) -> Result<Point3, AdapterError> {
    required_position(positions, id).map_err(|error| {
        rollback_error(
            host,
            transaction,
            "resolve coded-feature point",
            HostError::new(error.to_string()),
        )
    })
}

fn circle_request(
    first: Point3,
    second: Point3,
    third: Point3,
    layer: String,
) -> Result<CircleRequest, HostError> {
    let (center, radius) = circumcircle(first, second, third)?;
    Ok(CircleRequest {
        center,
        radius,
        layer,
    })
}

fn arc_request(
    first: Point3,
    through: Point3,
    last: Point3,
    layer: String,
) -> Result<ArcRequest, HostError> {
    let (center, radius) = circumcircle(first, through, last)?;
    let mut start = angle(center, first);
    let middle = angle(center, through);
    let mut end = angle(center, last);
    if !ccw_contains(start, middle, end) {
        std::mem::swap(&mut start, &mut end);
    }
    Ok(ArcRequest {
        center,
        radius,
        start_degrees: start.to_degrees(),
        end_degrees: end.to_degrees(),
        layer,
    })
}

fn circumcircle(first: Point3, second: Point3, third: Point3) -> Result<(Point3, f64), HostError> {
    // Work in a local frame. Squaring absolute survey coordinates loses the small differences
    // that define short-radius curves through large Easting/Northing values.
    let bx = second.x() - first.x();
    let by = second.y() - first.y();
    let cx = third.x() - first.x();
    let cy = third.y() - first.y();
    let determinant = 2.0 * bx.mul_add(cy, -(cx * by));
    let scale = bx.hypot(by).max(cx.hypot(cy)).max((bx - cx).hypot(by - cy));
    let degeneracy_threshold = 16.0 * f64::EPSILON * scale * scale;
    if !determinant.is_finite()
        || !scale.is_finite()
        || scale <= 0.0
        || determinant.abs() <= degeneracy_threshold
    {
        return Err(HostError::new("three-point curve is degenerate"));
    }
    let b2 = bx.mul_add(bx, by * by);
    let c2 = cx.mul_add(cx, cy * cy);
    let local_x = b2.mul_add(cy, -(c2 * by)) / determinant;
    let local_y = c2.mul_add(bx, -(b2 * cx)) / determinant;
    let x = first.x() + local_x;
    let y = first.y() + local_y;
    let z = (first.z() + second.z() + third.z()) / 3.0;
    let center = Point3::new(x, y, z).map_err(|error| HostError::new(error.to_string()))?;
    let radius = center.xy().distance_to(first.xy());
    if !radius.is_finite() || radius <= 0.0 {
        return Err(HostError::new("three-point curve has an invalid radius"));
    }
    Ok((center, radius))
}

fn angle(center: Point3, point: Point3) -> f64 {
    (point.y() - center.y()).atan2(point.x() - center.x())
}

fn ccw_contains(start: f64, middle: f64, end: f64) -> bool {
    let tau = std::f64::consts::TAU;
    let from_start = (middle - start).rem_euclid(tau);
    let span = (end - start).rem_euclid(tau);
    from_start <= span
}

fn create_symbol_primitive(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    primitive: &PlacedPrimitive,
    layer: &str,
    inherited_color: Option<&str>,
) -> Result<Vec<EntityId>, AdapterError> {
    match primitive {
        PlacedPrimitive::Line {
            from,
            to,
            stroke_width_mm,
            color,
        } => {
            let entity = host
                .create_line(
                    transaction,
                    &LineRequest {
                        start: *from,
                        end: *to,
                        layer: layer.to_owned(),
                    },
                )
                .map_err(|error| rollback_error(host, transaction, "create symbol line", error))?;
            apply_entity_appearance(
                host,
                transaction,
                entity,
                effective_color(color.as_deref(), inherited_color),
                Some(*stroke_width_mm),
                "style symbol line",
            )?;
            Ok(vec![entity])
        }
        PlacedPrimitive::Circle {
            center,
            radius,
            stroke_width_mm,
            filled,
            color,
        } => {
            let entity = host
                .create_circle(
                    transaction,
                    &CircleRequest {
                        center: *center,
                        radius: *radius,
                        layer: layer.to_owned(),
                    },
                )
                .map_err(|error| {
                    rollback_error(host, transaction, "create symbol circle", error)
                })?;
            let color = effective_color(color.as_deref(), inherited_color);
            apply_entity_appearance(
                host,
                transaction,
                entity,
                color.clone(),
                Some(*stroke_width_mm),
                "style symbol circle",
            )?;
            let mut entities = vec![entity];
            if *filled {
                let boundary = circle_fill_boundary(*center, *radius).map_err(|error| {
                    rollback_error(host, transaction, "prepare symbol circle fill", error)
                })?;
                let fill = host
                    .create_solid_fill(
                        transaction,
                        &SolidFillRequest {
                            boundary,
                            layer: layer.to_owned(),
                            color,
                        },
                    )
                    .map_err(|error| {
                        rollback_error(host, transaction, "create symbol circle fill", error)
                    })?;
                entities.push(fill);
            }
            Ok(entities)
        }
        PlacedPrimitive::Polyline {
            points,
            closed,
            stroke_width_mm,
            filled,
            color,
        } => {
            let pair_count = points.len().saturating_sub(1) + usize::from(*closed);
            let mut entities = Vec::with_capacity(pair_count.saturating_add(usize::from(*filled)));
            let color = effective_color(color.as_deref(), inherited_color);
            for pair in points.windows(2) {
                let Some(start) = pair.first().copied() else {
                    continue;
                };
                let Some(end) = pair.get(1).copied() else {
                    continue;
                };
                let entity = host
                    .create_line(
                        transaction,
                        &LineRequest {
                            start,
                            end,
                            layer: layer.to_owned(),
                        },
                    )
                    .map_err(|error| {
                        rollback_error(host, transaction, "create symbol polyline", error)
                    })?;
                apply_entity_appearance(
                    host,
                    transaction,
                    entity,
                    color.clone(),
                    Some(*stroke_width_mm),
                    "style symbol polyline",
                )?;
                entities.push(entity);
            }
            if *closed
                && let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
            {
                let entity = host
                    .create_line(
                        transaction,
                        &LineRequest {
                            start: last,
                            end: first,
                            layer: layer.to_owned(),
                        },
                    )
                    .map_err(|error| {
                        rollback_error(host, transaction, "close symbol polyline", error)
                    })?;
                apply_entity_appearance(
                    host,
                    transaction,
                    entity,
                    color.clone(),
                    Some(*stroke_width_mm),
                    "style closed symbol polyline",
                )?;
                entities.push(entity);
            }
            if *filled {
                let fill = host
                    .create_solid_fill(
                        transaction,
                        &SolidFillRequest {
                            boundary: points.clone(),
                            layer: layer.to_owned(),
                            color,
                        },
                    )
                    .map_err(|error| {
                        rollback_error(host, transaction, "fill symbol polyline", error)
                    })?;
                entities.push(fill);
            }
            Ok(entities)
        }
    }
}

fn effective_color(explicit: Option<&str>, inherited: Option<&str>) -> Option<String> {
    explicit.or(inherited).map(str::to_owned)
}

fn apply_entity_appearance(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    entity: EntityId,
    color: Option<String>,
    lineweight_mm: Option<f64>,
    operation: &'static str,
) -> Result<(), AdapterError> {
    host.set_entity_appearance(
        transaction,
        entity,
        &EntityAppearance {
            color,
            lineweight_mm,
        },
    )
    .map_err(|error| rollback_error(host, transaction, operation, error))
}

fn circle_fill_boundary(center: Point3, radius: f64) -> Result<Vec<Point3>, HostError> {
    const SEGMENTS: usize = 32;
    (0..SEGMENTS)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / SEGMENTS as f64;
            Point3::new(
                center.x() + radius * angle.cos(),
                center.y() + radius * angle.sin(),
                center.z(),
            )
            .map_err(|error| HostError::new(error.to_string()))
        })
        .collect()
}

fn tag_generated_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    entity: EntityId,
    feature_id: Uuid,
    rule_id: &str,
    source_ids: &[Uuid],
    role: &str,
) -> Result<(), AdapterError> {
    let sources = source_ids
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>()
        .join(",");
    for (key, value) in [
        (
            FEATURE_SET_METADATA_KEY,
            FEATURE_SET_METADATA_VALUE.to_owned(),
        ),
        (FEATURE_ID_METADATA_KEY, feature_id.to_string()),
        (RULE_ID_METADATA_KEY, rule_id.to_owned()),
        (SOURCE_POINTS_METADATA_KEY, sources),
        (ROLE_METADATA_KEY, format!("coded-{role}")),
    ] {
        host.set_entity_metadata(transaction, entity, key, &value)
            .map_err(|error| rollback_error(host, transaction, "tag coded feature", error))?;
    }
    Ok(())
}

fn projected_entity_count(
    plan: &surveycraft_coding::FeaturePlan,
    catalog: &SymbolCatalog,
    project: &SurveyProject,
) -> Result<usize, AdapterError> {
    let positions = project
        .points
        .iter()
        .map(|point| (point.id, point.position))
        .collect::<BTreeMap<_, _>>();
    let paths = plan
        .paths
        .iter()
        .try_fold(0usize, |total, path| total.checked_add(path.segments.len()));
    let Some(mut total) = paths.and_then(|value| value.checked_add(plan.circles.len())) else {
        return Err(AdapterError::new(
            "count coded features",
            HostError::new("coded entity count overflowed"),
            None,
        ));
    };
    for symbol in &plan.symbols {
        let insertion = required_position(&positions, symbol.point_id)?;
        let placed = catalog
            .place(symbol, insertion, project.linear_unit)
            .map_err(|error| {
                AdapterError::new(
                    "count symbol entities",
                    HostError::new(error.to_string()),
                    None,
                )
            })?;
        for primitive in &placed.primitives {
            let count = match primitive {
                PlacedPrimitive::Line { .. } => 1,
                PlacedPrimitive::Circle { filled, .. } => 1 + usize::from(*filled),
                PlacedPrimitive::Polyline {
                    points,
                    closed,
                    filled,
                    ..
                } => points.len().saturating_sub(1) + usize::from(*closed) + usize::from(*filled),
            };
            total = total.checked_add(count).ok_or_else(|| {
                AdapterError::new(
                    "count coded features",
                    HostError::new("coded entity count overflowed"),
                    None,
                )
            })?;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::circumcircle;
    use std::error::Error;
    use surveycraft_geometry::Point3;

    #[test]
    fn circumcircle_stays_precise_at_large_survey_coordinates() -> Result<(), Box<dyn Error>> {
        let center_x = 337_275.053;
        let center_y = 595_286.485;
        let expected_radius = 0.01;
        let point = |angle: f64| {
            Point3::new(
                center_x + expected_radius * angle.cos(),
                center_y + expected_radius * angle.sin(),
                100.0,
            )
        };
        let first = point(0.0)?;
        let second = point(std::f64::consts::TAU / 3.0)?;
        let third = point(std::f64::consts::TAU * 2.0 / 3.0)?;
        let (center, radius) = circumcircle(first, second, third)?;

        assert!((center.x() - center_x).abs() < 1.0e-8);
        assert!((center.y() - center_y).abs() < 1.0e-8);
        assert!((radius - expected_radius).abs() < 1.0e-8);
        Ok(())
    }
}
