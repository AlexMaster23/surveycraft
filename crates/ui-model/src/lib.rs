//! Toolkit-independent state exchanged between SurveyCraft commands and UI hosts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_coding::{
    CodeBook, CodingDiagnostic, CompiledCodeBook, DiagnosticSeverity, GraphicRule, JoinDirective,
    MatchMode,
};
use surveycraft_symbols::{SymbolCatalog, SymbolPoint, SymbolPrimitive};

pub const MAX_CODE_PREVIEW_ROWS: usize = 1_000;
pub const MAX_CODE_PREVIEW_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Module {
    Points,
    Coding,
    Parcels,
    Terrain,
    Profiles,
    Volumes,
    Coordinates,
    Raster,
    Plans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSummary {
    pub warnings: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointManagerState {
    pub total_points: usize,
    pub selected_points: usize,
    pub filter: String,
    pub diagnostics: DiagnosticSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeBookEditorError {
    TooManyPreviewRows,
    PreviewCodeTooLong(usize),
}

impl Display for CodeBookEditorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyPreviewRows => write!(
                formatter,
                "code preview exceeds {MAX_CODE_PREVIEW_ROWS} rows"
            ),
            Self::PreviewCodeTooLong(index) => write!(
                formatter,
                "preview code at row {} exceeds {MAX_CODE_PREVIEW_BYTES} bytes",
                index + 1
            ),
        }
    }
}

impl Error for CodeBookEditorError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDiagnostic {
    pub severity: EditorDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_row: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookLayerRow {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub plot: bool,
    pub used_by_rules: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditorGraphicKind {
    None,
    Line,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookRuleRow {
    pub id: String,
    pub pattern: String,
    pub match_mode: MatchMode,
    pub description: String,
    pub optimization_rank: u8,
    pub layer: Option<String>,
    pub graphic_kind: EditorGraphicKind,
    pub style_or_symbol: Option<String>,
    pub scale_or_width: Option<f64>,
    pub missing_symbol: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodePreviewRow {
    pub input: String,
    pub primary_rule_id: Option<String>,
    pub matched_rule_ids: Vec<String>,
    pub directives: Vec<JoinDirective>,
    pub diagnostics: Vec<CodingDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolOption {
    pub id: String,
    pub name: String,
    pub pack_id: String,
    pub pack_name: String,
    pub spdx: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolVectorPreview {
    pub id: String,
    pub name: String,
    pub anchor: SymbolPoint,
    pub primitives: Vec<SymbolPrimitive>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBookEditorState {
    pub book_name: String,
    pub valid: bool,
    pub layers: Vec<CodeBookLayerRow>,
    pub rules: Vec<CodeBookRuleRow>,
    pub previews: Vec<CodePreviewRow>,
    pub symbols: Vec<SymbolOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_preview: Option<SymbolVectorPreview>,
    pub diagnostics: Vec<EditorDiagnostic>,
    pub summary: DiagnosticSummary,
}

pub fn build_code_book_editor_state(
    book: &CodeBook,
    preview_codes: &[String],
    catalog: &SymbolCatalog,
    selected_symbol_id: Option<&str>,
) -> Result<CodeBookEditorState, CodeBookEditorError> {
    validate_preview_inputs(preview_codes)?;
    let usage = layer_usage(book);
    let layers = book
        .layers
        .iter()
        .map(|layer| CodeBookLayerRow {
            id: layer.id.clone(),
            name: layer.name.clone(),
            color: layer.color.clone(),
            plot: layer.plot,
            used_by_rules: usage.get(&layer.id).copied().unwrap_or_default(),
        })
        .collect();
    let rules = book
        .rules
        .iter()
        .map(|rule| {
            let (graphic_kind, style_or_symbol, scale_or_width, missing_symbol) =
                match &rule.graphic {
                    GraphicRule::None => (EditorGraphicKind::None, None, None, false),
                    GraphicRule::Line {
                        style,
                        paper_width_mm,
                    } => (
                        EditorGraphicKind::Line,
                        Some(style.clone()),
                        Some(*paper_width_mm),
                        false,
                    ),
                    GraphicRule::Symbol { symbol, scale, .. } => (
                        EditorGraphicKind::Symbol,
                        Some(symbol.clone()),
                        Some(*scale),
                        !catalog.contains(symbol),
                    ),
                };
            CodeBookRuleRow {
                id: rule.id.clone(),
                pattern: rule.pattern.clone(),
                match_mode: rule.match_mode,
                description: rule.description.clone(),
                optimization_rank: rule.optimization_rank,
                layer: rule.layer.clone(),
                graphic_kind,
                style_or_symbol,
                scale_or_width,
                missing_symbol,
            }
        })
        .collect();
    let symbols = catalog
        .symbol_ids()
        .into_iter()
        .filter_map(|id| {
            let definition = catalog.definition(id)?;
            let attribution = catalog.attribution(id)?;
            Some(SymbolOption {
                id: definition.id.clone(),
                name: definition.name.clone(),
                pack_id: attribution.pack_id,
                pack_name: attribution.pack_name,
                spdx: attribution.spdx,
            })
        })
        .collect::<Vec<_>>();
    let fallback_symbol_id = book
        .rules
        .iter()
        .find_map(|rule| match &rule.graphic {
            GraphicRule::Symbol { symbol, .. } if catalog.contains(symbol) => Some(symbol.as_str()),
            _ => None,
        })
        .or_else(|| symbols.first().map(|symbol| symbol.id.as_str()));
    let preview_symbol_id = selected_symbol_id.or(fallback_symbol_id);
    let symbol_preview =
        preview_symbol_id
            .and_then(|id| catalog.definition(id))
            .map(|definition| SymbolVectorPreview {
                id: definition.id.clone(),
                name: definition.name.clone(),
                anchor: definition.anchor,
                primitives: definition.primitives.clone(),
            });
    let mut diagnostics = missing_symbol_diagnostics(book, catalog);
    if let Some(selected) = selected_symbol_id
        && symbol_preview.is_none()
    {
        diagnostics.push(EditorDiagnostic {
            severity: EditorDiagnosticSeverity::Warning,
            code: "unknown-preview-symbol".to_owned(),
            message: format!("symbol '{selected}' is not available for preview"),
            preview_row: None,
        });
    }
    let mut previews = Vec::new();
    match CompiledCodeBook::compile(book.clone()) {
        Ok(compiled) => {
            for (index, code) in preview_codes.iter().enumerate() {
                match compiled.interpret_code(code) {
                    Ok(preview) => {
                        diagnostics.extend(preview.diagnostics.iter().map(|diagnostic| {
                            EditorDiagnostic {
                                severity: match diagnostic.severity {
                                    DiagnosticSeverity::Warning => {
                                        EditorDiagnosticSeverity::Warning
                                    }
                                    DiagnosticSeverity::Error => EditorDiagnosticSeverity::Error,
                                },
                                code: diagnostic.code.clone(),
                                message: diagnostic.message.clone(),
                                preview_row: Some(index),
                            }
                        }));
                        previews.push(CodePreviewRow {
                            input: code.clone(),
                            primary_rule_id: preview.primary_rule_id,
                            matched_rule_ids: preview
                                .matches
                                .into_iter()
                                .map(|rule_match| rule_match.rule_id)
                                .collect(),
                            directives: preview.directives.into_iter().collect(),
                            diagnostics: preview.diagnostics,
                        });
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let diagnostic = CodingDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            code: "invalid-preview-code".to_owned(),
                            message: message.clone(),
                            point_id: None,
                        };
                        diagnostics.push(EditorDiagnostic {
                            severity: EditorDiagnosticSeverity::Error,
                            code: diagnostic.code.clone(),
                            message,
                            preview_row: Some(index),
                        });
                        previews.push(CodePreviewRow {
                            input: code.clone(),
                            primary_rule_id: None,
                            matched_rule_ids: Vec::new(),
                            directives: Vec::new(),
                            diagnostics: vec![diagnostic],
                        });
                    }
                }
            }
        }
        Err(error) => diagnostics.push(EditorDiagnostic {
            severity: EditorDiagnosticSeverity::Error,
            code: "invalid-code-book".to_owned(),
            message: error.to_string(),
            preview_row: None,
        }),
    }
    diagnostics.sort_by(|left, right| {
        left.preview_row
            .cmp(&right.preview_row)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.message.cmp(&right.message))
    });
    let summary = DiagnosticSummary {
        warnings: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == EditorDiagnosticSeverity::Warning)
            .count(),
        errors: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == EditorDiagnosticSeverity::Error)
            .count(),
    };
    Ok(CodeBookEditorState {
        book_name: book.name.clone(),
        valid: summary.errors == 0,
        layers,
        rules,
        previews,
        symbols,
        symbol_preview,
        diagnostics,
        summary,
    })
}

fn validate_preview_inputs(preview_codes: &[String]) -> Result<(), CodeBookEditorError> {
    if preview_codes.len() > MAX_CODE_PREVIEW_ROWS {
        return Err(CodeBookEditorError::TooManyPreviewRows);
    }
    if let Some((index, _)) = preview_codes
        .iter()
        .enumerate()
        .find(|(_, code)| code.len() > MAX_CODE_PREVIEW_BYTES)
    {
        return Err(CodeBookEditorError::PreviewCodeTooLong(index));
    }
    Ok(())
}

fn layer_usage(book: &CodeBook) -> BTreeMap<String, usize> {
    let mut usage = BTreeMap::new();
    for layer in book.rules.iter().filter_map(|rule| rule.layer.as_ref()) {
        *usage.entry(layer.clone()).or_default() += 1;
    }
    usage
}

fn missing_symbol_diagnostics(book: &CodeBook, catalog: &SymbolCatalog) -> Vec<EditorDiagnostic> {
    book.rules
        .iter()
        .filter_map(|rule| match &rule.graphic {
            GraphicRule::Symbol { symbol, .. } if !catalog.contains(symbol) => {
                Some(EditorDiagnostic {
                    severity: EditorDiagnosticSeverity::Error,
                    code: "missing-symbol".to_owned(),
                    message: format!(
                        "rule '{}' requires unavailable symbol '{}'",
                        rule.id, symbol
                    ),
                    preview_row: None,
                })
            }
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub command_id: String,
    pub completed: u64,
    pub total: u64,
    pub message: String,
}

impl TaskProgress {
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed.min(self.total) as f64) / (self.total as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskProgress, build_code_book_editor_state};
    use std::error::Error;
    use surveycraft_coding::{
        CodeBook, CodeRule, DirectiveAliases, GraphicRule, LayerDefinition, MatchMode,
    };
    use surveycraft_symbols::{
        SymbolCatalog, SymbolDefinition, SymbolLicense, SymbolPack, SymbolPoint, SymbolPrimitive,
    };

    #[test]
    fn progress_is_bounded() {
        let progress = TaskProgress {
            command_id: "surveycraft.points.import".to_owned(),
            completed: 12,
            total: 10,
            message: "Done".to_owned(),
        };
        assert!((progress.fraction() - 1.0).abs() < f64::EPSILON);
    }

    fn book(symbol: &str) -> CodeBook {
        CodeBook {
            schema_version: 1,
            name: "Editor test".to_owned(),
            case_sensitive: false,
            token_separators: vec!['-'],
            directives: DirectiveAliases::default(),
            layers: vec![LayerDefinition {
                id: "symbols".to_owned(),
                name: "SC-SYMBOLS".to_owned(),
                color: None,
                plot: true,
            }],
            rules: vec![CodeRule {
                id: "tree".to_owned(),
                pattern: "TREE".to_owned(),
                match_mode: MatchMode::Exact,
                description: "Tree".to_owned(),
                optimization_rank: 1,
                layer: Some("symbols".to_owned()),
                graphic: GraphicRule::Symbol {
                    symbol: symbol.to_owned(),
                    scale: 1.0,
                    attribute_from_suffix: false,
                },
            }],
        }
    }

    fn catalog() -> Result<SymbolCatalog, surveycraft_symbols::SymbolError> {
        SymbolCatalog::compile(vec![SymbolPack {
            schema_version: 1,
            id: "surveycraft.test".to_owned(),
            name: "Test".to_owned(),
            version: "1.0.0".to_owned(),
            license: SymbolLicense {
                spdx: "MIT".to_owned(),
                notice: "Original test symbol.".to_owned(),
            },
            authors: vec!["SurveyCraft tests".to_owned()],
            homepage: None,
            symbols: vec![SymbolDefinition {
                id: "surveycraft.tree".to_owned(),
                name: "Tree".to_owned(),
                anchor: SymbolPoint::default(),
                primitives: vec![SymbolPrimitive::Circle {
                    center: SymbolPoint::default(),
                    radius: 1.0,
                    stroke_width_mm: 0.2,
                    filled: false,
                    color: None,
                }],
            }],
        }])
    }

    #[test]
    fn code_book_editor_builds_preview_and_symbol_options() -> Result<(), Box<dyn Error>> {
        let state = build_code_book_editor_state(
            &book("surveycraft.tree"),
            &["tree-start".to_owned()],
            &catalog()?,
            Some("surveycraft.tree"),
        )?;
        assert!(state.valid);
        assert_eq!(state.layers[0].used_by_rules, 1);
        assert_eq!(state.previews[0].primary_rule_id.as_deref(), Some("tree"));
        assert_eq!(state.symbols.len(), 1);
        assert_eq!(
            state
                .symbol_preview
                .as_ref()
                .map(|preview| preview.id.as_str()),
            Some("surveycraft.tree")
        );
        assert_eq!(state.summary.errors, 0);
        Ok(())
    }

    #[test]
    fn code_book_editor_reports_missing_symbols() -> Result<(), Box<dyn Error>> {
        let state = build_code_book_editor_state(
            &book("surveycraft.missing"),
            &["TREE".to_owned()],
            &catalog()?,
            None,
        )?;
        assert!(!state.valid);
        assert!(state.rules[0].missing_symbol);
        assert_eq!(state.summary.errors, 1);
        Ok(())
    }
}
