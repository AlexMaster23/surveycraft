//! CAD-independent code-book interpretation and coded feature planning.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_core::SurveyPoint;
use surveycraft_geometry::Point3;
use uuid::Uuid;

pub const CODE_BOOK_SCHEMA_VERSION: u32 = 1;
pub const MAX_CODE_BOOK_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_LAYERS: usize = 1_024;
pub const MAX_RULES: usize = 4_096;
pub const MAX_CODE_BYTES: usize = 4_096;
pub const MAX_CODE_TOKENS: usize = 64;
pub const MAX_FEATURE_POINTS: usize = 1_000_000;
pub const MAX_MATCH_EVALUATIONS: usize = 10_000_000;
const MAX_ID_BYTES: usize = 128;
const MAX_NAME_BYTES: usize = 256;
const MAX_PATTERN_BYTES: usize = 512;
const REFERENCE_SYMBOL_SCALE: f64 = 1_000.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodingError {
    UnsupportedSchemaVersion(u32),
    TooManyLayers,
    TooManyRules,
    TooManyPoints,
    InvalidText(&'static str),
    InvalidColor(String),
    DuplicateLayerId(String),
    DuplicateRuleId(String),
    UnknownLayer { rule: String, layer: String },
    InvalidOptimizationRank { rule: String, rank: u8 },
    InvalidGraphicValue { rule: String, field: &'static str },
    InvalidRegex { rule: String, message: String },
    DuplicateDirective(String),
    CodeTooLong,
    TooManyCodeTokens,
    MatchBudgetExceeded,
    InvalidQuery,
    InvalidReplacement,
    InvalidPlanScale,
    InvalidScaleFactor,
    InvalidAngle,
}

impl Display for CodingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported code-book schema version {version}")
            }
            Self::TooManyLayers => write!(formatter, "code book exceeds {MAX_LAYERS} layers"),
            Self::TooManyRules => write!(formatter, "code book exceeds {MAX_RULES} rules"),
            Self::TooManyPoints => {
                write!(
                    formatter,
                    "feature generation exceeds {MAX_FEATURE_POINTS} points"
                )
            }
            Self::InvalidText(field) => write!(formatter, "code-book field '{field}' is invalid"),
            Self::InvalidColor(color) => write!(formatter, "invalid RGB color '{color}'"),
            Self::DuplicateLayerId(id) => write!(formatter, "duplicate layer id '{id}'"),
            Self::DuplicateRuleId(id) => write!(formatter, "duplicate rule id '{id}'"),
            Self::UnknownLayer { rule, layer } => {
                write!(
                    formatter,
                    "rule '{rule}' references unknown layer '{layer}'"
                )
            }
            Self::InvalidOptimizationRank { rule, rank } => write!(
                formatter,
                "rule '{rule}' has optimization rank {rank}; expected 1 through 9"
            ),
            Self::InvalidGraphicValue { rule, field } => {
                write!(
                    formatter,
                    "rule '{rule}' has invalid graphic field '{field}'"
                )
            }
            Self::InvalidRegex { rule, message } => {
                write!(
                    formatter,
                    "rule '{rule}' has invalid regular expression: {message}"
                )
            }
            Self::DuplicateDirective(alias) => {
                write!(
                    formatter,
                    "directive alias '{alias}' has more than one meaning"
                )
            }
            Self::CodeTooLong => write!(formatter, "point code exceeds {MAX_CODE_BYTES} bytes"),
            Self::TooManyCodeTokens => {
                write!(formatter, "point code exceeds {MAX_CODE_TOKENS} tokens")
            }
            Self::MatchBudgetExceeded => write!(
                formatter,
                "code matching exceeds the {MAX_MATCH_EVALUATIONS}-evaluation safety budget"
            ),
            Self::InvalidQuery => write!(formatter, "code query is empty or too large"),
            Self::InvalidReplacement => {
                write!(
                    formatter,
                    "replacement code must be one bounded non-directive token"
                )
            }
            Self::InvalidPlanScale => write!(formatter, "plan scale must be finite and positive"),
            Self::InvalidScaleFactor => {
                write!(formatter, "symbol scale factor must be finite and positive")
            }
            Self::InvalidAngle => write!(formatter, "rotation angle must be finite"),
        }
    }
}

impl Error for CodingError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeBook {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default = "default_token_separators")]
    pub token_separators: Vec<char>,
    #[serde(default)]
    pub directives: DirectiveAliases,
    #[serde(default)]
    pub layers: Vec<LayerDefinition>,
    pub rules: Vec<CodeRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectiveAliases {
    #[serde(default)]
    pub start: Vec<String>,
    #[serde(default)]
    pub end: Vec<String>,
    #[serde(default)]
    pub close: Vec<String>,
    #[serde(default)]
    pub arc_start: Vec<String>,
    #[serde(default)]
    pub arc_end: Vec<String>,
    #[serde(default)]
    pub on_circle: Vec<String>,
}

impl Default for DirectiveAliases {
    fn default() -> Self {
        Self {
            start: vec!["START".to_owned()],
            end: vec!["END".to_owned()],
            close: vec!["CLOSE".to_owned()],
            arc_start: vec!["ARC_START".to_owned()],
            arc_end: vec!["ARC_END".to_owned()],
            on_circle: vec!["ON_CIRCLE".to_owned()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default = "default_true")]
    pub plot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchMode {
    Exact,
    Prefix,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeRule {
    pub id: String,
    pub pattern: String,
    pub match_mode: MatchMode,
    pub description: String,
    #[serde(default = "default_optimization_rank")]
    pub optimization_rank: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    pub graphic: GraphicRule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GraphicRule {
    None,
    Line {
        style: String,
        paper_width_mm: f64,
    },
    Symbol {
        symbol: String,
        scale: f64,
        #[serde(default)]
        attribute_from_suffix: bool,
    },
}

#[derive(Debug)]
pub struct CompiledCodeBook {
    book: CodeBook,
    rules: Vec<CompiledRule>,
    rule_indexes: HashMap<String, usize>,
    exact_rule_indexes: HashMap<String, Vec<usize>>,
    non_exact_rule_indexes: Vec<usize>,
    directives: HashMap<String, JoinDirective>,
}

#[derive(Debug)]
struct CompiledRule {
    normalized_pattern: String,
    regex: Option<Regex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JoinDirective {
    Start,
    End,
    Close,
    ArcStart,
    ArcEnd,
    OnCircle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMatch {
    pub token_index: usize,
    pub token: String,
    pub rule_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodingDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeInterpretation {
    pub raw: String,
    pub tokens: Vec<String>,
    pub matches: Vec<RuleMatch>,
    pub directives: BTreeSet<JoinDirective>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub symbol_attributes: BTreeMap<String, String>,
    pub diagnostics: Vec<CodingDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerateOptions {
    pub plan_scale_denominator: f64,
    pub include_unmarked: bool,
    pub coordinate_tolerance: f64,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            plan_scale_denominator: REFERENCE_SYMBOL_SCALE,
            include_unmarked: false,
            coordinate_tolerance: 1.0e-8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturePlan {
    pub paths: Vec<PathPlan>,
    pub circles: Vec<CirclePlan>,
    pub symbols: Vec<SymbolPlan>,
    pub diagnostics: Vec<CodingDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPlan {
    pub id: Uuid,
    pub rule_id: String,
    pub layer: Option<String>,
    pub style: String,
    pub paper_width_mm: f64,
    pub closed: bool,
    pub segments: Vec<FeatureSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FeatureSegment {
    Line {
        start: Uuid,
        end: Uuid,
    },
    Arc {
        start: Uuid,
        through: Uuid,
        end: Uuid,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CirclePlan {
    pub id: Uuid,
    pub rule_id: String,
    pub layer: Option<String>,
    pub style: String,
    pub paper_width_mm: f64,
    pub points: [Uuid; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolPlan {
    pub id: Uuid,
    pub point_id: Uuid,
    pub rule_id: String,
    pub symbol: String,
    pub layer: Option<String>,
    pub scale: f64,
    pub rotation_degrees: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolSelector {
    All,
    Symbol(String),
    Rule(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearingUnit {
    Degrees,
    Gon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineStyleDecision {
    pub rule_id: String,
    pub layer: Option<String>,
    pub style: String,
    pub paper_width_mm: f64,
    pub matching_points: usize,
    pub coded_points: usize,
}

#[derive(Debug, Clone)]
struct FeaturePoint {
    id: Uuid,
    position: Point3,
    line_rule_id: Option<String>,
    directives: BTreeSet<JoinDirective>,
}

impl CompiledCodeBook {
    pub fn compile(book: CodeBook) -> Result<Self, CodingError> {
        validate_book_header(&book)?;
        let mut layer_ids = HashSet::with_capacity(book.layers.len());
        for layer in &book.layers {
            validate_id(&layer.id, "layers.id")?;
            validate_name(&layer.name, "layers.name")?;
            if let Some(color) = &layer.color
                && !is_rgb_color(color)
            {
                return Err(CodingError::InvalidColor(color.clone()));
            }
            if !layer_ids.insert(layer.id.clone()) {
                return Err(CodingError::DuplicateLayerId(layer.id.clone()));
            }
        }

        let directives = compile_directives(&book)?;
        let mut ids = HashSet::with_capacity(book.rules.len());
        let mut compiled_rules = Vec::with_capacity(book.rules.len());
        let mut rule_indexes = HashMap::with_capacity(book.rules.len());
        let mut exact_rule_indexes: HashMap<String, Vec<usize>> = HashMap::new();
        let mut non_exact_rule_indexes = Vec::new();
        for (index, rule) in book.rules.iter().enumerate() {
            validate_rule(rule, &layer_ids)?;
            if !ids.insert(rule.id.clone()) {
                return Err(CodingError::DuplicateRuleId(rule.id.clone()));
            }
            let regex = if rule.match_mode == MatchMode::Regex {
                let expression = if book.case_sensitive {
                    rule.pattern.clone()
                } else {
                    format!("(?i:{})", rule.pattern)
                };
                let anchored = format!("^(?:{expression})$");
                Some(
                    Regex::new(&anchored).map_err(|error| CodingError::InvalidRegex {
                        rule: rule.id.clone(),
                        message: error.to_string(),
                    })?,
                )
            } else {
                None
            };
            let normalized_pattern = if book.case_sensitive {
                rule.pattern.clone()
            } else {
                rule.pattern.to_lowercase()
            };
            if rule.match_mode == MatchMode::Exact {
                exact_rule_indexes
                    .entry(normalized_pattern.clone())
                    .or_default()
                    .push(index);
            } else {
                non_exact_rule_indexes.push(index);
            }
            compiled_rules.push(CompiledRule {
                normalized_pattern,
                regex,
            });
            rule_indexes.insert(rule.id.clone(), index);
        }
        Ok(Self {
            book,
            rules: compiled_rules,
            rule_indexes,
            exact_rule_indexes,
            non_exact_rule_indexes,
            directives,
        })
    }

    #[must_use]
    pub const fn book(&self) -> &CodeBook {
        &self.book
    }

    pub fn interpret_code(&self, raw: &str) -> Result<CodeInterpretation, CodingError> {
        let mut remaining_evaluations = MAX_MATCH_EVALUATIONS;
        self.interpret_code_with_budget(raw, &mut remaining_evaluations)
    }

    pub fn interpret_codes<'a>(
        &self,
        codes: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<CodeInterpretation>, CodingError> {
        let mut remaining_evaluations = MAX_MATCH_EVALUATIONS;
        codes
            .into_iter()
            .map(|code| self.interpret_code_with_budget(code, &mut remaining_evaluations))
            .collect()
    }

    fn interpret_code_with_budget(
        &self,
        raw: &str,
        remaining_evaluations: &mut usize,
    ) -> Result<CodeInterpretation, CodingError> {
        if raw.len() > MAX_CODE_BYTES {
            return Err(CodingError::CodeTooLong);
        }
        let tokens = self.tokenize(raw)?;
        let mut matches = Vec::new();
        let mut directives = BTreeSet::new();
        let mut diagnostics = Vec::new();
        let mut primary_rule_id = None;
        let mut symbol_attributes = BTreeMap::new();
        let mut pending_attribute_rule: Option<String> = None;

        for (token_index, token) in tokens.iter().enumerate() {
            let normalized = self.normalize(token);
            if let Some(directive) = self.directives.get(&normalized) {
                directives.insert(*directive);
                continue;
            }
            if let Some(rule_id) = pending_attribute_rule.take() {
                symbol_attributes.insert(rule_id, token.clone());
                continue;
            }
            let candidates = self.matching_rule_indexes(token, remaining_evaluations)?;
            let Some(selected) = candidates.first().copied() else {
                diagnostics.push(CodingDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "unknown-code".to_owned(),
                    message: format!("no code-book rule matches '{token}'"),
                    point_id: None,
                });
                continue;
            };
            if candidates.len() > 1 {
                let ids = candidates
                    .iter()
                    .map(|candidate| self.book.rules[*candidate].id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(CodingDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "ambiguous-code".to_owned(),
                    message: format!(
                        "'{token}' matches {ids}; deterministic precedence selected one"
                    ),
                    point_id: None,
                });
            }
            let rule = &self.book.rules[selected];
            if primary_rule_id.is_none() {
                primary_rule_id = Some(rule.id.clone());
            }
            matches.push(RuleMatch {
                token_index,
                token: token.clone(),
                rule_id: rule.id.clone(),
            });
            if matches!(
                rule.graphic,
                GraphicRule::Symbol {
                    attribute_from_suffix: true,
                    ..
                }
            ) {
                pending_attribute_rule = Some(rule.id.clone());
            }
        }
        if let Some(rule_id) = pending_attribute_rule {
            diagnostics.push(CodingDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "missing-symbol-attribute".to_owned(),
                message: format!("symbol rule '{rule_id}' requires a following attribute token"),
                point_id: None,
            });
        }
        Ok(CodeInterpretation {
            raw: raw.to_owned(),
            tokens,
            matches,
            directives,
            primary_rule_id,
            symbol_attributes,
            diagnostics,
        })
    }

    pub fn select_points(
        &self,
        points: &[SurveyPoint],
        query: &str,
    ) -> Result<Vec<Uuid>, CodingError> {
        if query.trim().is_empty() || query.len() > MAX_CODE_BYTES {
            return Err(CodingError::InvalidQuery);
        }
        let query = self.normalize(query.trim());
        let mut selected = Vec::new();
        for point in points {
            let Some(code) = point.code.as_deref() else {
                continue;
            };
            let normalized_code = self.normalize(code);
            let tokens = self.tokenize(code)?;
            if wildcard_matches(&query, &normalized_code)
                || tokens
                    .iter()
                    .any(|token| wildcard_matches(&query, &self.normalize(token)))
            {
                selected.push(point.id);
            }
        }
        Ok(selected)
    }

    pub fn change_first_code(
        &self,
        points: &mut [SurveyPoint],
        selected: &BTreeSet<Uuid>,
        replacement: &str,
    ) -> Result<usize, CodingError> {
        let replacement = replacement.trim();
        if replacement.is_empty()
            || replacement.len() > MAX_PATTERN_BYTES
            || replacement.chars().any(char::is_control)
            || replacement
                .chars()
                .any(|character| self.book.token_separators.contains(&character))
            || self.directives.contains_key(&self.normalize(replacement))
        {
            return Err(CodingError::InvalidReplacement);
        }
        let mut changed = 0usize;
        for point in points {
            if !selected.contains(&point.id) {
                continue;
            }
            point.code = Some(replace_first_token(
                point.code.as_deref().map_or("", |value| value),
                replacement,
                &self.book.token_separators,
            ));
            changed += 1;
        }
        Ok(changed)
    }

    pub fn generate_features(
        &self,
        project_id: Uuid,
        points: &[SurveyPoint],
        options: GenerateOptions,
    ) -> Result<FeaturePlan, CodingError> {
        if points.len() > MAX_FEATURE_POINTS {
            return Err(CodingError::TooManyPoints);
        }
        if !options.plan_scale_denominator.is_finite() || options.plan_scale_denominator <= 0.0 {
            return Err(CodingError::InvalidPlanScale);
        }
        if !options.coordinate_tolerance.is_finite() || options.coordinate_tolerance <= 0.0 {
            return Err(CodingError::InvalidGraphicValue {
                rule: "generation-options".to_owned(),
                field: "coordinateTolerance",
            });
        }

        let mut ordered: Vec<&SurveyPoint> = points.iter().collect();
        ordered.sort_by(|left, right| compare_point_numbers(&left.number, &right.number));
        let mut feature_points = Vec::with_capacity(ordered.len());
        let mut symbols = Vec::new();
        let mut diagnostics = Vec::new();
        let mut circle_groups: BTreeMap<String, Vec<FeaturePoint>> = BTreeMap::new();
        let mut planned_symbols: HashSet<(Uuid, String)> = HashSet::new();
        let mut remaining_evaluations = MAX_MATCH_EVALUATIONS;

        for point in ordered {
            let interpretation = self.interpret_code_with_budget(
                point.code.as_deref().map_or("", |value| value),
                &mut remaining_evaluations,
            )?;
            diagnostics.extend(
                interpretation
                    .diagnostics
                    .iter()
                    .cloned()
                    .map(|mut diagnostic| {
                        diagnostic.point_id = Some(point.id);
                        diagnostic
                    }),
            );
            let line_rule_id = interpretation
                .matches
                .iter()
                .find(|rule_match| self.is_line_rule(&rule_match.rule_id))
                .map(|rule_match| rule_match.rule_id.clone());
            let feature_point = FeaturePoint {
                id: point.id,
                position: point.position,
                line_rule_id: line_rule_id.clone(),
                directives: interpretation.directives.clone(),
            };
            if interpretation.directives.contains(&JoinDirective::OnCircle)
                && let Some(rule_id) = &line_rule_id
            {
                circle_groups
                    .entry(rule_id.clone())
                    .or_default()
                    .push(feature_point.clone());
            }
            for rule_match in &interpretation.matches {
                let Some(rule) = self.rule(&rule_match.rule_id) else {
                    continue;
                };
                let GraphicRule::Symbol {
                    symbol,
                    scale,
                    attribute_from_suffix: _,
                } = &rule.graphic
                else {
                    continue;
                };
                if !planned_symbols.insert((point.id, symbol.clone())) {
                    continue;
                }
                let effective_scale =
                    options.plan_scale_denominator / REFERENCE_SYMBOL_SCALE * scale;
                if !effective_scale.is_finite() || effective_scale <= 0.0 {
                    return Err(CodingError::InvalidScaleFactor);
                }
                symbols.push(SymbolPlan {
                    id: stable_id(project_id, &format!("symbol:{}:{}", point.id, rule.id)),
                    point_id: point.id,
                    rule_id: rule.id.clone(),
                    symbol: symbol.clone(),
                    layer: rule.layer.clone(),
                    scale: effective_scale,
                    rotation_degrees: 0.0,
                    attribute: interpretation.symbol_attributes.get(&rule.id).cloned(),
                });
            }
            feature_points.push(feature_point);
        }

        let circles = self.build_circles(
            project_id,
            circle_groups,
            options.coordinate_tolerance,
            &mut diagnostics,
        );
        let paths = self.build_paths(
            project_id,
            &feature_points,
            options.include_unmarked,
            options.coordinate_tolerance,
        );
        Ok(FeaturePlan {
            paths,
            circles,
            symbols,
            diagnostics,
        })
    }

    pub fn choose_line_style(
        &self,
        codes: &[&str],
    ) -> Result<Option<LineStyleDecision>, CodingError> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut coded_points = 0usize;
        let mut remaining_evaluations = MAX_MATCH_EVALUATIONS;
        for code in codes {
            let interpretation =
                self.interpret_code_with_budget(code, &mut remaining_evaluations)?;
            if let Some(rule_id) = interpretation
                .matches
                .iter()
                .find(|rule_match| self.is_line_rule(&rule_match.rule_id))
                .map(|rule_match| rule_match.rule_id.clone())
            {
                *counts.entry(rule_id).or_default() += 1;
                coded_points += 1;
            }
        }
        let selected = counts.iter().max_by(|left, right| {
            left.1.cmp(right.1).then_with(|| {
                let left_rule = self.rule(left.0);
                let right_rule = self.rule(right.0);
                match (left_rule, right_rule) {
                    (Some(left_rule), Some(right_rule)) => right_rule
                        .optimization_rank
                        .cmp(&left_rule.optimization_rank)
                        .then_with(|| right.0.cmp(left.0)),
                    _ => Ordering::Equal,
                }
            })
        });
        let Some((rule_id, matching_points)) = selected else {
            return Ok(None);
        };
        let Some(rule) = self.rule(rule_id) else {
            return Ok(None);
        };
        let GraphicRule::Line {
            style,
            paper_width_mm,
        } = &rule.graphic
        else {
            return Ok(None);
        };
        Ok(Some(LineStyleDecision {
            rule_id: rule.id.clone(),
            layer: rule.layer.clone(),
            style: style.clone(),
            paper_width_mm: *paper_width_mm,
            matching_points: *matching_points,
            coded_points,
        }))
    }

    fn tokenize(&self, raw: &str) -> Result<Vec<String>, CodingError> {
        let tokens: Vec<String> = raw
            .split(|character| self.book.token_separators.contains(&character))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
            .collect();
        if tokens.len() > MAX_CODE_TOKENS {
            return Err(CodingError::TooManyCodeTokens);
        }
        Ok(tokens)
    }

    fn normalize(&self, value: &str) -> String {
        if self.book.case_sensitive {
            value.to_owned()
        } else {
            value.to_lowercase()
        }
    }

    fn matching_rule_indexes(
        &self,
        token: &str,
        remaining_evaluations: &mut usize,
    ) -> Result<Vec<usize>, CodingError> {
        let evaluation_count = self.non_exact_rule_indexes.len().saturating_add(1);
        if *remaining_evaluations < evaluation_count {
            return Err(CodingError::MatchBudgetExceeded);
        }
        *remaining_evaluations -= evaluation_count;
        let normalized_token = self.normalize(token);
        let mut indexes = self
            .exact_rule_indexes
            .get(&normalized_token)
            .cloned()
            .unwrap_or_default();
        indexes.extend(self.non_exact_rule_indexes.iter().filter_map(|index| {
            let rule = &self.book.rules[*index];
            let normalized_pattern = &self.rules[*index].normalized_pattern;
            let matched = match rule.match_mode {
                MatchMode::Exact => false,
                MatchMode::Prefix => normalized_token.starts_with(normalized_pattern),
                MatchMode::Regex => self
                    .rules
                    .get(*index)
                    .and_then(|compiled| compiled.regex.as_ref())
                    .is_some_and(|regex| regex.is_match(token)),
            };
            matched.then_some(*index)
        }));
        indexes.sort_by(|left, right| {
            compare_rules(&self.book.rules[*left], &self.book.rules[*right])
        });
        Ok(indexes)
    }

    fn rule(&self, id: &str) -> Option<&CodeRule> {
        self.rule_indexes
            .get(id)
            .and_then(|index| self.book.rules.get(*index))
    }

    fn is_line_rule(&self, id: &str) -> bool {
        self.rule(id)
            .is_some_and(|rule| matches!(rule.graphic, GraphicRule::Line { .. }))
    }

    fn build_circles(
        &self,
        project_id: Uuid,
        groups: BTreeMap<String, Vec<FeaturePoint>>,
        tolerance: f64,
        diagnostics: &mut Vec<CodingDiagnostic>,
    ) -> Vec<CirclePlan> {
        let mut circles = Vec::new();
        for (rule_id, points) in groups {
            for chunk in points.chunks(3) {
                if chunk.len() != 3 {
                    diagnostics.push(CodingDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        code: "incomplete-circle".to_owned(),
                        message: format!(
                            "rule '{rule_id}' has {} unused circle point(s)",
                            chunk.len()
                        ),
                        point_id: chunk.first().map(|point| point.id),
                    });
                    continue;
                }
                if circle_is_degenerate(
                    chunk[0].position,
                    chunk[1].position,
                    chunk[2].position,
                    tolerance,
                ) {
                    diagnostics.push(CodingDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        code: "degenerate-circle".to_owned(),
                        message: format!(
                            "rule '{rule_id}' has collinear or repeated circle points"
                        ),
                        point_id: Some(chunk[0].id),
                    });
                    continue;
                }
                let Some(rule) = self.rule(&rule_id) else {
                    continue;
                };
                let GraphicRule::Line {
                    style,
                    paper_width_mm,
                } = &rule.graphic
                else {
                    continue;
                };
                let point_ids = [chunk[0].id, chunk[1].id, chunk[2].id];
                circles.push(CirclePlan {
                    id: stable_id(
                        project_id,
                        &format!(
                            "circle:{rule_id}:{}:{}:{}",
                            point_ids[0], point_ids[1], point_ids[2]
                        ),
                    ),
                    rule_id: rule_id.clone(),
                    layer: rule.layer.clone(),
                    style: style.clone(),
                    paper_width_mm: *paper_width_mm,
                    points: point_ids,
                });
            }
        }
        circles
    }

    fn build_paths(
        &self,
        project_id: Uuid,
        points: &[FeaturePoint],
        include_unmarked: bool,
        coordinate_tolerance: f64,
    ) -> Vec<PathPlan> {
        let mut by_rule: BTreeMap<String, Vec<&FeaturePoint>> = BTreeMap::new();
        for point in points {
            if let Some(rule_id) = &point.line_rule_id {
                by_rule.entry(rule_id.clone()).or_default().push(point);
            }
        }
        let mut paths = Vec::new();
        let mut used = HashSet::new();
        for (rule_id, rule_points) in &by_rule {
            let mut active = Vec::new();
            for point in rule_points {
                if point.directives.contains(&JoinDirective::Start) {
                    self.finish_path(
                        project_id,
                        rule_id,
                        &active,
                        coordinate_tolerance,
                        &mut paths,
                        &mut used,
                    );
                    active.clear();
                    active.push(*point);
                } else if !active.is_empty() {
                    active.push(*point);
                }
                if !active.is_empty()
                    && (point.directives.contains(&JoinDirective::End)
                        || point.directives.contains(&JoinDirective::Close))
                {
                    self.finish_path(
                        project_id,
                        rule_id,
                        &active,
                        coordinate_tolerance,
                        &mut paths,
                        &mut used,
                    );
                    active.clear();
                }
            }
            self.finish_path(
                project_id,
                rule_id,
                &active,
                coordinate_tolerance,
                &mut paths,
                &mut used,
            );
        }

        if include_unmarked {
            let mut run: Vec<&FeaturePoint> = Vec::new();
            let mut run_rule: Option<&str> = None;
            for point in points {
                let candidate_rule = point
                    .line_rule_id
                    .as_deref()
                    .filter(|_| !used.contains(&point.id));
                if candidate_rule != run_rule {
                    if let Some(rule_id) = run_rule {
                        self.finish_path(
                            project_id,
                            rule_id,
                            &run,
                            coordinate_tolerance,
                            &mut paths,
                            &mut used,
                        );
                    }
                    run.clear();
                    run_rule = candidate_rule;
                }
                if candidate_rule.is_some() {
                    run.push(point);
                }
                if point.directives.contains(&JoinDirective::End)
                    || point.directives.contains(&JoinDirective::Close)
                {
                    if let Some(rule_id) = run_rule {
                        self.finish_path(
                            project_id,
                            rule_id,
                            &run,
                            coordinate_tolerance,
                            &mut paths,
                            &mut used,
                        );
                    }
                    run.clear();
                    run_rule = None;
                }
            }
            if let Some(rule_id) = run_rule {
                self.finish_path(
                    project_id,
                    rule_id,
                    &run,
                    coordinate_tolerance,
                    &mut paths,
                    &mut used,
                );
            }
        }
        paths
    }

    fn finish_path(
        &self,
        project_id: Uuid,
        rule_id: &str,
        points: &[&FeaturePoint],
        coordinate_tolerance: f64,
        paths: &mut Vec<PathPlan>,
        used: &mut HashSet<Uuid>,
    ) {
        if points.len() < 2 {
            return;
        }
        let Some(rule) = self.rule(rule_id) else {
            return;
        };
        let GraphicRule::Line {
            style,
            paper_width_mm,
        } = &rule.graphic
        else {
            return;
        };
        let closed = points
            .iter()
            .any(|point| point.directives.contains(&JoinDirective::Close));
        let segments = build_segments(points, closed, coordinate_tolerance);
        if segments.is_empty() {
            return;
        }
        let point_identity = points
            .iter()
            .map(|point| point.id.to_string())
            .collect::<Vec<_>>()
            .join(":");
        for point in points {
            used.insert(point.id);
        }
        paths.push(PathPlan {
            id: stable_id(
                project_id,
                &format!("path:{rule_id}:{closed}:{point_identity}"),
            ),
            rule_id: rule_id.to_owned(),
            layer: rule.layer.clone(),
            style: style.clone(),
            paper_width_mm: *paper_width_mm,
            closed,
            segments,
        });
    }
}

pub fn rescale_symbols(
    symbols: &mut [SymbolPlan],
    selector: &SymbolSelector,
    factor: f64,
) -> Result<usize, CodingError> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err(CodingError::InvalidScaleFactor);
    }
    if symbols.iter().any(|symbol| {
        let scaled = symbol.scale * factor;
        symbol_selected(symbol, selector) && (!scaled.is_finite() || scaled <= 0.0)
    }) {
        return Err(CodingError::InvalidScaleFactor);
    }
    let mut changed = 0usize;
    for symbol in symbols {
        if symbol_selected(symbol, selector) {
            symbol.scale *= factor;
            changed += 1;
        }
    }
    Ok(changed)
}

pub fn rotate_symbols(
    symbols: &mut [SymbolPlan],
    selector: &SymbolSelector,
    bearing: f64,
    unit: BearingUnit,
) -> Result<usize, CodingError> {
    let rotation = topographic_bearing_to_cad_degrees(bearing, unit)?;
    let mut changed = 0usize;
    for symbol in symbols {
        if symbol_selected(symbol, selector) {
            symbol.rotation_degrees = rotation;
            changed += 1;
        }
    }
    Ok(changed)
}

pub fn delete_symbols(symbols: &mut Vec<SymbolPlan>, selector: &SymbolSelector) -> usize {
    let before = symbols.len();
    symbols.retain(|symbol| !symbol_selected(symbol, selector));
    before.saturating_sub(symbols.len())
}

pub fn topographic_bearing_to_cad_degrees(
    bearing: f64,
    unit: BearingUnit,
) -> Result<f64, CodingError> {
    if !bearing.is_finite() {
        return Err(CodingError::InvalidAngle);
    }
    let degrees = match unit {
        BearingUnit::Degrees => bearing,
        BearingUnit::Gon => bearing * 0.9,
    };
    Ok((90.0 - degrees).rem_euclid(360.0))
}

fn validate_book_header(book: &CodeBook) -> Result<(), CodingError> {
    if book.schema_version != CODE_BOOK_SCHEMA_VERSION {
        return Err(CodingError::UnsupportedSchemaVersion(book.schema_version));
    }
    validate_name(&book.name, "name")?;
    if book.layers.len() > MAX_LAYERS {
        return Err(CodingError::TooManyLayers);
    }
    if book.rules.len() > MAX_RULES {
        return Err(CodingError::TooManyRules);
    }
    if book.token_separators.is_empty()
        || book.token_separators.len() > 8
        || book
            .token_separators
            .iter()
            .any(|separator| separator.is_alphanumeric() || separator.is_control())
    {
        return Err(CodingError::InvalidText("tokenSeparators"));
    }
    let unique: HashSet<char> = book.token_separators.iter().copied().collect();
    if unique.len() != book.token_separators.len() {
        return Err(CodingError::InvalidText("tokenSeparators"));
    }
    Ok(())
}

fn validate_rule(rule: &CodeRule, layers: &HashSet<String>) -> Result<(), CodingError> {
    validate_id(&rule.id, "rules.id")?;
    validate_name(&rule.description, "rules.description")?;
    if rule.pattern.trim().is_empty()
        || rule.pattern.len() > MAX_PATTERN_BYTES
        || rule.pattern.chars().any(char::is_control)
    {
        return Err(CodingError::InvalidText("rules.pattern"));
    }
    if !(1..=9).contains(&rule.optimization_rank) {
        return Err(CodingError::InvalidOptimizationRank {
            rule: rule.id.clone(),
            rank: rule.optimization_rank,
        });
    }
    if let Some(layer) = &rule.layer
        && !layers.contains(layer)
    {
        return Err(CodingError::UnknownLayer {
            rule: rule.id.clone(),
            layer: layer.clone(),
        });
    }
    match &rule.graphic {
        GraphicRule::None => {}
        GraphicRule::Line {
            style,
            paper_width_mm,
        } => {
            validate_name(style, "rules.graphic.style")?;
            validate_positive(rule, "paperWidthMm", *paper_width_mm)?;
        }
        GraphicRule::Symbol { symbol, scale, .. } => {
            validate_id(symbol, "rules.graphic.symbol")?;
            validate_positive(rule, "scale", *scale)?;
        }
    }
    Ok(())
}

fn validate_positive(rule: &CodeRule, field: &'static str, value: f64) -> Result<(), CodingError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(CodingError::InvalidGraphicValue {
            rule: rule.id.clone(),
            field,
        })
    }
}

fn validate_id(value: &str, field: &'static str) -> Result<(), CodingError> {
    if value.is_empty()
        || value.len() > MAX_ID_BYTES
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        Err(CodingError::InvalidText(field))
    } else {
        Ok(())
    }
}

fn validate_name(value: &str, field: &'static str) -> Result<(), CodingError> {
    if value.trim().is_empty()
        || value.len() > MAX_NAME_BYTES
        || value.chars().any(char::is_control)
    {
        Err(CodingError::InvalidText(field))
    } else {
        Ok(())
    }
}

fn is_rgb_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit())
}

fn compile_directives(book: &CodeBook) -> Result<HashMap<String, JoinDirective>, CodingError> {
    let groups = [
        (&book.directives.start, JoinDirective::Start),
        (&book.directives.end, JoinDirective::End),
        (&book.directives.close, JoinDirective::Close),
        (&book.directives.arc_start, JoinDirective::ArcStart),
        (&book.directives.arc_end, JoinDirective::ArcEnd),
        (&book.directives.on_circle, JoinDirective::OnCircle),
    ];
    let mut compiled = HashMap::new();
    for (aliases, directive) in groups {
        for alias in aliases {
            if alias.trim().is_empty()
                || alias.len() > MAX_PATTERN_BYTES
                || alias.chars().any(char::is_control)
            {
                return Err(CodingError::InvalidText("directives"));
            }
            let normalized = if book.case_sensitive {
                alias.clone()
            } else {
                alias.to_lowercase()
            };
            if compiled.insert(normalized.clone(), directive).is_some() {
                return Err(CodingError::DuplicateDirective(alias.clone()));
            }
        }
    }
    Ok(compiled)
}

fn compare_rules(left: &CodeRule, right: &CodeRule) -> Ordering {
    match_rank(left.match_mode)
        .cmp(&match_rank(right.match_mode))
        .then_with(|| left.optimization_rank.cmp(&right.optimization_rank))
        .then_with(|| left.id.cmp(&right.id))
}

const fn match_rank(mode: MatchMode) -> u8 {
    match mode {
        MatchMode::Exact => 0,
        MatchMode::Prefix => 1,
        MatchMode::Regex => 2,
    }
}

fn compare_point_numbers(left: &str, right: &str) -> Ordering {
    match (left.parse::<u128>(), right.parse::<u128>()) {
        (Ok(left_number), Ok(right_number)) => {
            left_number.cmp(&right_number).then_with(|| left.cmp(right))
        }
        _ => left.cmp(right),
    }
}

fn build_segments(
    points: &[&FeaturePoint],
    closed: bool,
    coordinate_tolerance: f64,
) -> Vec<FeatureSegment> {
    let mut segments = Vec::new();
    let mut index = 0usize;
    while index + 1 < points.len() {
        if points[index].directives.contains(&JoinDirective::ArcStart)
            && let Some(end_offset) = points[index + 1..]
                .iter()
                .position(|point| point.directives.contains(&JoinDirective::ArcEnd))
        {
            let end_index = index + 1 + end_offset;
            if end_index >= index + 2 {
                if circle_is_degenerate(
                    points[index].position,
                    points[index + 1].position,
                    points[end_index].position,
                    coordinate_tolerance,
                ) {
                    segments.push(FeatureSegment::Line {
                        start: points[index].id,
                        end: points[end_index].id,
                    });
                } else {
                    segments.push(FeatureSegment::Arc {
                        start: points[index].id,
                        through: points[index + 1].id,
                        end: points[end_index].id,
                    });
                }
                index = end_index;
                continue;
            }
        }
        segments.push(FeatureSegment::Line {
            start: points[index].id,
            end: points[index + 1].id,
        });
        index += 1;
    }
    if closed
        && let (Some(first), Some(last)) = (points.first(), points.last())
        && first.id != last.id
    {
        segments.push(FeatureSegment::Line {
            start: last.id,
            end: first.id,
        });
    }
    segments
}

fn circle_is_degenerate(first: Point3, second: Point3, third: Point3, tolerance: f64) -> bool {
    let baseline_x = third.x() - first.x();
    let baseline_y = third.y() - first.y();
    let baseline_length = baseline_x.hypot(baseline_y);
    if baseline_length <= tolerance {
        return true;
    }
    let twice_area = (second.x() - first.x()) * baseline_y - (second.y() - first.y()) * baseline_x;
    twice_area.abs() / baseline_length <= tolerance
}

fn stable_id(project_id: Uuid, identity: &str) -> Uuid {
    Uuid::new_v5(&project_id, identity.as_bytes())
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let mut pattern_index = 0usize;
    let mut value_index = 0usize;
    let mut star_index = None;
    let mut retry_value_index = 0usize;
    while value_index < value.len() {
        if pattern.get(pattern_index) == value.get(value_index) {
            pattern_index += 1;
            value_index += 1;
        } else if pattern.get(pattern_index) == Some(&'*') {
            star_index = Some(pattern_index);
            pattern_index += 1;
            retry_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            retry_value_index += 1;
            value_index = retry_value_index;
        } else {
            return false;
        }
    }
    while pattern.get(pattern_index) == Some(&'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn replace_first_token(raw: &str, replacement: &str, separators: &[char]) -> String {
    let mut start = None;
    let mut end = raw.len();
    for (index, character) in raw.char_indices() {
        if start.is_none() {
            if !character.is_whitespace() && !separators.contains(&character) {
                start = Some(index);
            }
        } else if separators.contains(&character) {
            end = raw[..index].trim_end().len();
            break;
        }
    }
    let Some(start) = start else {
        return replacement.to_owned();
    };
    format!("{}{}{}", &raw[..start], replacement, &raw[end..])
}

fn symbol_selected(symbol: &SymbolPlan, selector: &SymbolSelector) -> bool {
    match selector {
        SymbolSelector::All => true,
        SymbolSelector::Symbol(id) => symbol.symbol == *id,
        SymbolSelector::Rule(id) => symbol.rule_id == *id,
    }
}

const fn default_optimization_rank() -> u8 {
    9
}

fn default_token_separators() -> Vec<char> {
    vec!['-']
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{
        BearingUnit, CodeBook, CodeRule, CompiledCodeBook, DirectiveAliases, FeatureSegment,
        GenerateOptions, GraphicRule, LayerDefinition, MatchMode, SymbolSelector, delete_symbols,
        rescale_symbols, rotate_symbols, topographic_bearing_to_cad_degrees,
    };
    use std::collections::BTreeSet;
    use std::error::Error;
    use surveycraft_core::{SourceRef, SurveyPoint};
    use surveycraft_geometry::Point3;
    use uuid::Uuid;

    fn book() -> Result<CompiledCodeBook, super::CodingError> {
        CompiledCodeBook::compile(CodeBook {
            schema_version: 1,
            name: "Synthetic field codes".to_owned(),
            case_sensitive: false,
            token_separators: vec!['-'],
            directives: DirectiveAliases {
                start: vec!["START".to_owned(), "STA".to_owned()],
                end: vec!["END".to_owned()],
                close: vec!["CLOSE".to_owned()],
                arc_start: vec!["ARC_START".to_owned()],
                arc_end: vec!["ARC_END".to_owned()],
                on_circle: vec!["ON_CIRCLE".to_owned()],
            },
            layers: vec![
                LayerDefinition {
                    id: "edge".to_owned(),
                    name: "SC-FEATURE-EDGE".to_owned(),
                    color: Some("#F47A20".to_owned()),
                    plot: true,
                },
                LayerDefinition {
                    id: "symbols".to_owned(),
                    name: "SC-SYMBOLS".to_owned(),
                    color: None,
                    plot: true,
                },
            ],
            rules: vec![
                CodeRule {
                    id: "edge".to_owned(),
                    pattern: "EP".to_owned(),
                    match_mode: MatchMode::Exact,
                    description: "Edge line".to_owned(),
                    optimization_rank: 1,
                    layer: Some("edge".to_owned()),
                    graphic: GraphicRule::Line {
                        style: "continuous".to_owned(),
                        paper_width_mm: 0.25,
                    },
                },
                CodeRule {
                    id: "boundary".to_owned(),
                    pattern: "B(D|Y)".to_owned(),
                    match_mode: MatchMode::Regex,
                    description: "Boundary line".to_owned(),
                    optimization_rank: 2,
                    layer: Some("edge".to_owned()),
                    graphic: GraphicRule::Line {
                        style: "dashed".to_owned(),
                        paper_width_mm: 0.18,
                    },
                },
                CodeRule {
                    id: "tree".to_owned(),
                    pattern: "TREE".to_owned(),
                    match_mode: MatchMode::Prefix,
                    description: "Original tree marker".to_owned(),
                    optimization_rank: 3,
                    layer: Some("symbols".to_owned()),
                    graphic: GraphicRule::Symbol {
                        symbol: "surveycraft.tree".to_owned(),
                        scale: 1.25,
                        attribute_from_suffix: false,
                    },
                },
                CodeRule {
                    id: "post".to_owned(),
                    pattern: "POST".to_owned(),
                    match_mode: MatchMode::Exact,
                    description: "Numbered post marker".to_owned(),
                    optimization_rank: 2,
                    layer: Some("symbols".to_owned()),
                    graphic: GraphicRule::Symbol {
                        symbol: "surveycraft.post".to_owned(),
                        scale: 1.0,
                        attribute_from_suffix: true,
                    },
                },
            ],
        })
    }

    fn point(number: &str, x: f64, y: f64, code: &str) -> Result<SurveyPoint, Box<dyn Error>> {
        Ok(SurveyPoint::new(
            Uuid::new_v5(&Uuid::nil(), number.as_bytes()),
            number,
            Point3::new(x, y, 0.0)?,
            Some(code.to_owned()),
            None,
            SourceRef {
                source: "synthetic".to_owned(),
                row: 1,
            },
        )?)
    }

    #[test]
    fn rejects_code_matching_before_the_shared_budget_is_exhausted() -> Result<(), Box<dyn Error>> {
        let compiled = book()?;
        let mut remaining_evaluations = 2;
        let result = compiled.interpret_code_with_budget("EP", &mut remaining_evaluations);
        assert!(matches!(
            result,
            Err(super::CodingError::MatchBudgetExceeded)
        ));
        Ok(())
    }

    #[test]
    fn interprets_multi_codes_directives_and_attributes() -> Result<(), Box<dyn Error>> {
        let book = book()?;
        let interpreted = book.interpret_code("EP-START")?;
        assert_eq!(interpreted.primary_rule_id.as_deref(), Some("edge"));
        assert!(
            interpreted
                .directives
                .contains(&super::JoinDirective::Start)
        );

        let post = book.interpret_code("POST-24")?;
        assert_eq!(
            post.symbol_attributes.get("post").map(String::as_str),
            Some("24")
        );
        assert!(post.diagnostics.is_empty());

        let regex = book.interpret_code("by")?;
        assert_eq!(regex.primary_rule_id.as_deref(), Some("boundary"));
        Ok(())
    }

    #[test]
    fn selects_wildcards_and_replaces_only_the_first_code() -> Result<(), Box<dyn Error>> {
        let book = book()?;
        let mut points = vec![
            point("1", 0.0, 0.0, "TREE-OAK")?,
            point("2", 1.0, 0.0, "EP-START")?,
        ];
        let selected = book.select_points(&points, "TREE*")?;
        assert_eq!(selected, [points[0].id]);
        let selected_set: BTreeSet<Uuid> = selected.into_iter().collect();
        assert_eq!(
            book.change_first_code(&mut points, &selected_set, "POST")?,
            1
        );
        assert_eq!(points[0].code.as_deref(), Some("POST-OAK"));
        assert_eq!(points[1].code.as_deref(), Some("EP-START"));
        assert!(
            book.change_first_code(&mut points, &selected_set, "START")
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn generates_ordered_closed_paths_arcs_and_circles() -> Result<(), Box<dyn Error>> {
        let book = book()?;
        let points = vec![
            point("4", 0.0, 1.0, "EP-ARC_END-CLOSE")?,
            point("2", 1.0, 0.0, "EP-ARC_START")?,
            point("3", 1.0, 1.0, "EP")?,
            point("1", 0.0, 0.0, "EP-START")?,
            point("10", 5.0, 0.0, "EP-ON_CIRCLE")?,
            point("11", 6.0, 1.0, "EP-ON_CIRCLE")?,
            point("12", 7.0, 0.0, "EP-ON_CIRCLE")?,
        ];
        let plan = book.generate_features(Uuid::nil(), &points, GenerateOptions::default())?;
        assert_eq!(plan.paths.len(), 1);
        assert!(plan.paths[0].closed);
        assert_eq!(plan.paths[0].segments.len(), 3);
        assert!(matches!(
            plan.paths[0].segments[1],
            FeatureSegment::Arc { .. }
        ));
        assert_eq!(plan.circles.len(), 1);
        Ok(())
    }

    #[test]
    fn degrades_collinear_arcs_to_lines() -> Result<(), Box<dyn Error>> {
        let book = book()?;
        let points = vec![
            point("1", 0.0, 0.0, "EP-START-ARC_START")?,
            point("2", 1.0, 0.0, "EP")?,
            point("3", 2.0, 0.0, "EP-ARC_END-END")?,
        ];
        let plan = book.generate_features(Uuid::nil(), &points, GenerateOptions::default())?;
        assert!(matches!(
            plan.paths[0].segments[0],
            FeatureSegment::Line { .. }
        ));
        Ok(())
    }

    #[test]
    fn symbol_plans_are_scaled_rotated_and_deleted_deterministically() -> Result<(), Box<dyn Error>>
    {
        let book = book()?;
        let points = vec![
            point("1", 0.0, 0.0, "TREE-TREE")?,
            point("2", 1.0, 0.0, "POST-7")?,
        ];
        let mut plan = book.generate_features(
            Uuid::nil(),
            &points,
            GenerateOptions {
                plan_scale_denominator: 2_000.0,
                ..GenerateOptions::default()
            },
        )?;
        assert_eq!(plan.symbols.len(), 2);
        assert!((plan.symbols[0].scale - 2.5).abs() < f64::EPSILON);
        assert_eq!(
            rescale_symbols(&mut plan.symbols, &SymbolSelector::All, 2.0)?,
            2
        );
        assert_eq!(
            rotate_symbols(
                &mut plan.symbols,
                &SymbolSelector::Rule("tree".to_owned()),
                100.0,
                BearingUnit::Gon,
            )?,
            1
        );
        assert!(plan.symbols[0].rotation_degrees.abs() < f64::EPSILON);
        assert_eq!(
            delete_symbols(
                &mut plan.symbols,
                &SymbolSelector::Symbol("surveycraft.post".to_owned())
            ),
            1
        );
        assert_eq!(plan.symbols.len(), 1);
        Ok(())
    }

    #[test]
    fn rejects_symbol_scale_overflow_without_partial_mutation() -> Result<(), Box<dyn Error>> {
        let base = book()?;
        let mut raw_book = base.book().clone();
        let tree_rule = raw_book
            .rules
            .iter_mut()
            .find(|rule| rule.id == "tree")
            .ok_or("missing tree rule")?;
        let GraphicRule::Symbol { scale, .. } = &mut tree_rule.graphic else {
            return Err("tree rule is not a symbol".into());
        };
        *scale = f64::MAX;
        let book = CompiledCodeBook::compile(raw_book)?;
        let points = vec![point("1", 0.0, 0.0, "TREE")?];
        assert!(
            book.generate_features(
                Uuid::nil(),
                &points,
                GenerateOptions {
                    plan_scale_denominator: f64::MAX,
                    ..GenerateOptions::default()
                },
            )
            .is_err()
        );

        let mut symbols = vec![super::SymbolPlan {
            id: Uuid::nil(),
            point_id: Uuid::nil(),
            rule_id: "tree".to_owned(),
            symbol: "surveycraft.tree".to_owned(),
            layer: None,
            scale: f64::MAX,
            rotation_degrees: 0.0,
            attribute: None,
        }];
        let before = symbols.clone();
        assert!(rescale_symbols(&mut symbols, &SymbolSelector::All, 2.0).is_err());
        assert_eq!(symbols, before);
        Ok(())
    }

    #[test]
    fn line_style_uses_majority_then_importance() -> Result<(), Box<dyn Error>> {
        let book = book()?;
        let majority = book
            .choose_line_style(&["TREE", "EP", "EP"])?
            .ok_or("missing majority decision")?;
        assert_eq!(majority.rule_id, "edge");
        assert_eq!(majority.matching_points, 2);
        let tie = book
            .choose_line_style(&["BD", "EP"])?
            .ok_or("missing tie decision")?;
        assert_eq!(tie.rule_id, "edge");
        Ok(())
    }

    #[test]
    fn converts_topographic_bearings_to_world_cad_angles() -> Result<(), Box<dyn Error>> {
        assert!(
            (topographic_bearing_to_cad_degrees(0.0, BearingUnit::Gon)? - 90.0).abs()
                < f64::EPSILON
        );
        assert!(topographic_bearing_to_cad_degrees(100.0, BearingUnit::Gon)?.abs() < f64::EPSILON);
        assert!(
            (topographic_bearing_to_cad_degrees(200.0, BearingUnit::Gon)? - 270.0).abs()
                < f64::EPSILON
        );
        Ok(())
    }
}
