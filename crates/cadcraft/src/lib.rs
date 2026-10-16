//! Narrow, testable boundary between SurveyCraft commands and a CAD host.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_core::{
    Crs, LinearUnit, NumericPolicy, QualityFlag, SourceRef, SurveyPoint, SurveyProject,
    migrate_project,
};
use surveycraft_geometry::Point3;
use uuid::Uuid;

mod coding;
pub use coding::*;

const DRAWING_PROJECT_SCHEMA_VERSION: u32 = 1;
const PROJECT_METADATA_KEY: &str = "survey-project";
const POINT_ID_METADATA_KEY: &str = "point-id";
const PROJECT_ID_METADATA_KEY: &str = "project-id";
const ROLE_METADATA_KEY: &str = "role";
const MAX_INSERT_POINTS: usize = 2_000;
const MAX_COORDINATE: f64 = 1.0e15;
const MAX_POINT_NUMBER_BYTES: usize = 128;
const MAX_CODE_BYTES: usize = 256;
const MAX_DESCRIPTION_BYTES: usize = 1_024;
const MAX_MANAGER_MUTATION_POINTS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransactionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineRequest {
    pub start: Point3,
    pub end: Point3,
    pub layer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcRequest {
    pub center: Point3,
    pub radius: f64,
    pub start_degrees: f64,
    pub end_degrees: f64,
    pub layer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircleRequest {
    pub center: Point3,
    pub radius: f64,
    pub layer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointRequest {
    pub position: Point3,
    pub layer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRequest {
    pub position: Point3,
    pub value: String,
    pub height: f64,
    pub rotation_degrees: f64,
    pub layer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntityKind {
    Line,
    Arc,
    Circle,
    Point,
    Text,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityView {
    pub id: EntityId,
    pub kind: EntityKind,
    pub layer: String,
    pub position: Option<Point3>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointRow {
    pub number: String,
    #[serde(alias = "easting")]
    pub x: f64,
    #[serde(alias = "northing")]
    pub y: f64,
    pub elevation: f64,
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoordinateConvention {
    #[default]
    CadXy,
    SurveyXy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPointsRequest {
    pub rows: Vec<PointRow>,
    #[serde(default)]
    pub convention: CoordinateConvention,
    #[serde(default = "default_label_height")]
    pub label_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPointsResult {
    pub inserted: usize,
    pub entities: usize,
    pub total_points: usize,
    pub project_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PointEntityRole {
    Point,
    NumberLabel,
    ElevationLabel,
    CodeLabel,
}

impl PointEntityRole {
    const fn metadata_value(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::NumberLabel => "number-label",
            Self::ElevationLabel => "elevation-label",
            Self::CodeLabel => "code-label",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointEntityLinks {
    pub point_id: Uuid,
    pub point: EntityId,
    pub number_label: EntityId,
    pub elevation_label: EntityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_label: Option<EntityId>,
}

impl PointEntityLinks {
    fn roles(&self) -> [(PointEntityRole, Option<EntityId>); 4] {
        [
            (PointEntityRole::Point, Some(self.point)),
            (PointEntityRole::NumberLabel, Some(self.number_label)),
            (PointEntityRole::ElevationLabel, Some(self.elevation_label)),
            (PointEntityRole::CodeLabel, self.code_label),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawingSurveyProject {
    pub schema_version: u32,
    pub project: SurveyProject,
    pub entity_links: Vec<PointEntityLinks>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReconciliationIssueKind {
    MissingEntity,
    WrongEntityKind,
    GeometryChanged,
    TextChanged,
    LayerChanged,
    MetadataChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationIssue {
    pub point_id: Uuid,
    pub role: PointEntityRole,
    pub entity: EntityId,
    pub kind: ReconciliationIssueKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationReport {
    pub project_id: Option<Uuid>,
    pub points: usize,
    pub healthy_points: usize,
    pub missing_entities: usize,
    pub modified_entities: usize,
    pub orphaned_entities: usize,
    pub issues: Vec<ReconciliationIssue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PointManagerRequest {
    List,
    Update {
        rows: Vec<PointManagerEdit>,
    },
    Delete {
        ids: Vec<Uuid>,
    },
    Synchronize {
        ids: Vec<Uuid>,
    },
    Renumber {
        ids: Vec<Uuid>,
        #[serde(default)]
        prefix: String,
        start: u64,
        step: u64,
    },
    Export {
        #[serde(default)]
        ids: Vec<Uuid>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointManagerEdit {
    pub id: Uuid,
    pub number: String,
    pub x: f64,
    pub y: f64,
    pub elevation: f64,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointManagerRow {
    pub id: Uuid,
    pub entities: Vec<u64>,
    pub number: String,
    pub x: f64,
    pub y: f64,
    pub elevation: f64,
    pub code: String,
    pub description: String,
    pub quality: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointManagerSnapshot {
    pub rows: Vec<PointManagerRow>,
    pub summary: String,
    pub project_id: Option<Uuid>,
    pub points: usize,
    pub healthy_points: usize,
    pub missing_entities: usize,
    pub modified_entities: usize,
    pub orphaned_entities: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointManagerMutation {
    pub message: String,
    pub affected: usize,
    pub total_points: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointManagerExport {
    pub file_name: String,
    pub content: String,
    pub points: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointManagerResult {
    Snapshot(PointManagerSnapshot),
    Mutation(PointManagerMutation),
    Export(PointManagerExport),
}

fn default_label_height() -> f64 {
    0.25
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HostError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    operation: &'static str,
    source: HostError,
    rollback_error: Option<HostError>,
}

impl AdapterError {
    fn new(operation: &'static str, source: HostError, rollback_error: Option<HostError>) -> Self {
        Self {
            operation,
            source,
            rollback_error,
        }
    }
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "CAD host failed to {}: {}",
            self.operation, self.source
        )?;
        if let Some(rollback_error) = &self.rollback_error {
            write!(formatter, "; rollback also failed: {rollback_error}")?;
        }
        Ok(())
    }
}

impl Error for AdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Portable host surface used by SurveyCraft commands. Metadata is implicitly scoped to the
/// running extension by CADCraft; implementations must make every transaction undoable.
pub trait CadcraftHost {
    fn begin_transaction(&mut self, label: &str) -> Result<TransactionId, HostError>;
    fn create_line(
        &mut self,
        transaction: TransactionId,
        request: &LineRequest,
    ) -> Result<EntityId, HostError>;
    fn create_arc(
        &mut self,
        transaction: TransactionId,
        request: &ArcRequest,
    ) -> Result<EntityId, HostError>;
    fn create_circle(
        &mut self,
        transaction: TransactionId,
        request: &CircleRequest,
    ) -> Result<EntityId, HostError>;
    fn create_point(
        &mut self,
        transaction: TransactionId,
        request: &PointRequest,
    ) -> Result<EntityId, HostError>;
    fn create_text(
        &mut self,
        transaction: TransactionId,
        request: &TextRequest,
    ) -> Result<EntityId, HostError>;
    fn get_drawing_metadata(&mut self, key: &str) -> Result<Option<String>, HostError>;
    fn set_drawing_metadata(
        &mut self,
        transaction: TransactionId,
        key: &str,
        value: &str,
    ) -> Result<(), HostError>;
    fn remove_drawing_metadata(
        &mut self,
        transaction: TransactionId,
        key: &str,
    ) -> Result<(), HostError>;
    fn get_entity(&mut self, entity: EntityId) -> Result<Option<EntityView>, HostError>;
    fn get_entity_metadata(
        &mut self,
        entity: EntityId,
        key: &str,
    ) -> Result<Option<String>, HostError>;
    fn find_entities_by_metadata(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<Vec<EntityView>, HostError>;
    fn set_entity_metadata(
        &mut self,
        transaction: TransactionId,
        entity: EntityId,
        key: &str,
        value: &str,
    ) -> Result<(), HostError>;
    fn remove_entity_metadata(
        &mut self,
        transaction: TransactionId,
        entity: EntityId,
        key: &str,
    ) -> Result<(), HostError>;
    fn update_point(
        &mut self,
        transaction: TransactionId,
        entity: EntityId,
        request: &PointRequest,
    ) -> Result<(), HostError>;
    fn update_text(
        &mut self,
        transaction: TransactionId,
        entity: EntityId,
        request: &TextRequest,
    ) -> Result<(), HostError>;
    fn delete_entity(
        &mut self,
        transaction: TransactionId,
        entity: EntityId,
    ) -> Result<(), HostError>;
    fn commit_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError>;
    fn rollback_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError>;
}

/// Insert survey points and persist stable point/entity links in one undoable transaction.
pub fn insert_points(
    host: &mut impl CadcraftHost,
    request: &InsertPointsRequest,
) -> Result<InsertPointsResult, AdapterError> {
    validate_insert_request(request)
        .map_err(|error| AdapterError::new("validate points", error, None))?;
    let existing = load_drawing_project(host)?;
    let mut stored = match existing {
        Some(stored) => stored,
        None => new_drawing_project(request)
            .map_err(|error| AdapterError::new("create survey project", error, None))?,
    };
    reject_existing_numbers(&stored.project, request)
        .map_err(|error| AdapterError::new("validate points", error, None))?;

    let transaction = host
        .begin_transaction("Insert SurveyCraft points")
        .map_err(|error| AdapterError::new("begin point transaction", error, None))?;
    let original_points = stored.project.points.len();
    let mut entities = 0usize;
    for (index, row) in request.rows.iter().enumerate() {
        let (cad_x, cad_y) = cad_xy(row, request.convention);
        let position = point(cad_x, cad_y, row.elevation)
            .map_err(|error| rollback_error(host, transaction, "prepare point", error))?;
        let point_id = drawing_point_id(stored.project.id, &row.number);
        let point_entity = create_point_entity(
            host,
            transaction,
            &PointRequest {
                position,
                layer: stored.project.styles.point.layer.clone(),
            },
        )?;
        tag_entity(
            host,
            transaction,
            point_entity,
            stored.project.id,
            point_id,
            PointEntityRole::Point,
        )?;
        entities += 1;

        let labels = &stored.project.styles.labels;
        let number_position = point(
            cad_x + labels.offset_x,
            cad_y + labels.offset_y,
            row.elevation,
        )
        .map_err(|error| rollback_error(host, transaction, "prepare point number", error))?;
        let number_label = create_text_entity(
            host,
            transaction,
            &TextRequest {
                position: number_position,
                value: row.number.trim().to_owned(),
                height: labels.height,
                rotation_degrees: 0.0,
                layer: labels.number_layer.clone(),
            },
        )?;
        tag_entity(
            host,
            transaction,
            number_label,
            stored.project.id,
            point_id,
            PointEntityRole::NumberLabel,
        )?;
        entities += 1;

        let elevation_position = point(
            cad_x + labels.offset_x,
            cad_y - labels.offset_y,
            row.elevation,
        )
        .map_err(|error| rollback_error(host, transaction, "prepare elevation label", error))?;
        let elevation_label = create_text_entity(
            host,
            transaction,
            &TextRequest {
                position: elevation_position,
                value: format!(
                    "{:.*}",
                    usize::from(labels.elevation_decimals),
                    row.elevation
                ),
                height: labels.height,
                rotation_degrees: 0.0,
                layer: labels.elevation_layer.clone(),
            },
        )?;
        tag_entity(
            host,
            transaction,
            elevation_label,
            stored.project.id,
            point_id,
            PointEntityRole::ElevationLabel,
        )?;
        entities += 1;

        let code = row.code.trim();
        let code_label = if code.is_empty() {
            None
        } else {
            let code_position = point(
                cad_x + labels.offset_x,
                cad_y - labels.offset_y * 2.4,
                row.elevation,
            )
            .map_err(|error| rollback_error(host, transaction, "prepare point code", error))?;
            let entity = create_text_entity(
                host,
                transaction,
                &TextRequest {
                    position: code_position,
                    value: code.to_owned(),
                    height: labels.height,
                    rotation_degrees: 0.0,
                    layer: labels.code_layer.clone(),
                },
            )?;
            tag_entity(
                host,
                transaction,
                entity,
                stored.project.id,
                point_id,
                PointEntityRole::CodeLabel,
            )?;
            entities += 1;
            Some(entity)
        };

        let source_row = original_points.saturating_add(index).saturating_add(1);
        let survey_point = SurveyPoint::new(
            point_id,
            row.number.clone(),
            position,
            (!code.is_empty()).then(|| code.to_owned()),
            None,
            SourceRef {
                source: "drawing-import".to_owned(),
                row: source_row,
            },
        )
        .map_err(|error| {
            rollback_error(
                host,
                transaction,
                "record survey point",
                HostError::new(error.to_string()),
            )
        })?;
        stored.project.add_point(survey_point);
        stored.entity_links.push(PointEntityLinks {
            point_id,
            point: point_entity,
            number_label,
            elevation_label,
            code_label,
        });
    }

    stored.project.validate().map_err(|error| {
        rollback_error(
            host,
            transaction,
            "validate survey project",
            HostError::new(error.to_string()),
        )
    })?;
    validate_drawing_project(&stored)
        .map_err(|error| rollback_error(host, transaction, "validate entity links", error))?;
    let metadata = serde_json::to_string(&stored).map_err(|error| {
        rollback_error(
            host,
            transaction,
            "encode survey project",
            HostError::new(error.to_string()),
        )
    })?;
    host.set_drawing_metadata(transaction, PROJECT_METADATA_KEY, &metadata)
        .map_err(|error| rollback_error(host, transaction, "persist survey project", error))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(
            "commit point transaction",
            error,
            rollback_error,
        ));
    }
    Ok(InsertPointsResult {
        inserted: request.rows.len(),
        entities,
        total_points: stored.project.points.len(),
        project_id: stored.project.id,
    })
}

/// Report deleted or externally edited linked entities without changing the drawing.
pub fn inspect_project(host: &mut impl CadcraftHost) -> Result<ReconciliationReport, AdapterError> {
    let stored = load_drawing_project(host)?;
    let report = match stored {
        Some(stored) => reconcile(host, &stored)?,
        None => ReconciliationReport {
            project_id: None,
            points: 0,
            healthy_points: 0,
            missing_entities: 0,
            modified_entities: 0,
            orphaned_entities: 0,
            issues: Vec::new(),
        },
    };
    let transaction = host
        .begin_transaction("Inspect SurveyCraft point links")
        .map_err(|error| AdapterError::new("begin inspection transaction", error, None))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(
            "commit inspection transaction",
            error,
            rollback_error,
        ));
    }
    Ok(report)
}

/// Load or mutate the persistent point model through the native CADCraft data-manager protocol.
pub fn manage_points(
    host: &mut impl CadcraftHost,
    request: &PointManagerRequest,
) -> Result<PointManagerResult, AdapterError> {
    match request {
        PointManagerRequest::List => list_managed_points(host).map(PointManagerResult::Snapshot),
        PointManagerRequest::Update { rows } => {
            update_managed_points(host, rows).map(PointManagerResult::Mutation)
        }
        PointManagerRequest::Delete { ids } => {
            delete_managed_points(host, ids).map(PointManagerResult::Mutation)
        }
        PointManagerRequest::Synchronize { ids } => {
            synchronize_managed_points(host, ids).map(PointManagerResult::Mutation)
        }
        PointManagerRequest::Renumber {
            ids,
            prefix,
            start,
            step,
        } => renumber_managed_points(host, ids, prefix, *start, *step)
            .map(PointManagerResult::Mutation),
        PointManagerRequest::Export { ids } => {
            export_managed_points(host, ids).map(PointManagerResult::Export)
        }
    }
}

fn mutation_message(action: &str, count: usize) -> String {
    format!(
        "{action} {count} survey {}",
        if count == 1 { "point" } else { "points" }
    )
}

fn list_managed_points(host: &mut impl CadcraftHost) -> Result<PointManagerSnapshot, AdapterError> {
    let snapshot = point_manager_snapshot(host)?;
    commit_read_only(host, "List SurveyCraft points", "complete point listing")?;
    Ok(snapshot)
}

fn point_manager_snapshot(
    host: &mut impl CadcraftHost,
) -> Result<PointManagerSnapshot, AdapterError> {
    let Some(stored) = load_drawing_project(host)? else {
        return Ok(PointManagerSnapshot {
            rows: Vec::new(),
            summary: "No SurveyCraft point project is stored in this drawing".into(),
            project_id: None,
            points: 0,
            healthy_points: 0,
            missing_entities: 0,
            modified_entities: 0,
            orphaned_entities: 0,
        });
    };
    let report = reconcile(host, &stored)?;
    let mut issues: BTreeMap<Uuid, Vec<ReconciliationIssueKind>> = BTreeMap::new();
    for issue in &report.issues {
        issues.entry(issue.point_id).or_default().push(issue.kind);
    }
    let links: BTreeMap<Uuid, &PointEntityLinks> = stored
        .entity_links
        .iter()
        .map(|links| (links.point_id, links))
        .collect();
    let mut rows = stored
        .project
        .points
        .iter()
        .filter_map(|point| {
            let linked = links.get(&point.id).copied()?;
            let status = match issues.get(&point.id) {
                None => "Healthy",
                Some(kinds)
                    if kinds.iter().any(|kind| {
                        matches!(
                            kind,
                            ReconciliationIssueKind::MissingEntity
                                | ReconciliationIssueKind::WrongEntityKind
                        )
                    }) =>
                {
                    "Missing"
                }
                Some(_) => "Modified",
            };
            let quality = if point.quality_flags.is_empty() {
                "OK".to_owned()
            } else {
                point
                    .quality_flags
                    .iter()
                    .map(|flag| match flag {
                        QualityFlag::DuplicateNumber => "Duplicate number",
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            Some(PointManagerRow {
                id: point.id,
                entities: linked
                    .roles()
                    .into_iter()
                    .filter_map(|(_, entity)| entity.map(|entity| entity.0))
                    .collect(),
                number: point.number.clone(),
                x: point.position.x(),
                y: point.position.y(),
                elevation: point.position.z(),
                code: point.code.clone().unwrap_or_default(),
                description: point.description.clone().unwrap_or_default(),
                quality,
                status: status.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        compare_point_numbers(&left.number, &right.number).then_with(|| left.id.cmp(&right.id))
    });
    let summary = format!(
        "{} {} · {} healthy · {} missing · {} modified · {} orphaned entities",
        report.points,
        if report.points == 1 {
            "point"
        } else {
            "points"
        },
        report.healthy_points,
        report.missing_entities,
        report.modified_entities,
        report.orphaned_entities
    );
    Ok(PointManagerSnapshot {
        rows,
        summary,
        project_id: report.project_id,
        points: report.points,
        healthy_points: report.healthy_points,
        missing_entities: report.missing_entities,
        modified_entities: report.modified_entities,
        orphaned_entities: report.orphaned_entities,
    })
}

fn update_managed_points(
    host: &mut impl CadcraftHost,
    rows: &[PointManagerEdit],
) -> Result<PointManagerMutation, AdapterError> {
    if rows.is_empty() || rows.len() > MAX_MANAGER_MUTATION_POINTS {
        return Err(AdapterError::new(
            "validate point edits",
            HostError::new(format!(
                "one edit may contain 1 to {MAX_MANAGER_MUTATION_POINTS} points"
            )),
            None,
        ));
    }
    let mut stored = required_drawing_project(host)?;
    let mut edited = BTreeSet::new();
    for row in rows {
        if !edited.insert(row.id) {
            return Err(AdapterError::new(
                "validate point edits",
                HostError::new("one point id occurs more than once"),
                None,
            ));
        }
        let Some(existing) = stored
            .project
            .points
            .iter_mut()
            .find(|point| point.id == row.id)
        else {
            return Err(AdapterError::new(
                "validate point edits",
                HostError::new(format!("point id {} is not in this drawing", row.id)),
                None,
            ));
        };
        let position = point(row.x, row.y, row.elevation)
            .map_err(|error| AdapterError::new("validate point edits", error, None))?;
        let code = optional_manager_text(&row.code, MAX_CODE_BYTES, "code")
            .map_err(|error| AdapterError::new("validate point edits", error, None))?;
        let description =
            optional_manager_text(&row.description, MAX_DESCRIPTION_BYTES, "description")
                .map_err(|error| AdapterError::new("validate point edits", error, None))?;
        let mut replacement = SurveyPoint::new(
            existing.id,
            row.number.clone(),
            position,
            code,
            description,
            existing.source.clone(),
        )
        .map_err(|error| {
            AdapterError::new(
                "validate point edits",
                HostError::new(error.to_string()),
                None,
            )
        })?;
        replacement.quality_flags = existing.quality_flags.clone();
        *existing = replacement;
    }
    validate_unique_numbers(&stored.project)
        .map_err(|error| AdapterError::new("validate point edits", error, None))?;
    mutate_linked_points(host, &mut stored, &edited, "Edit SurveyCraft points")?;
    Ok(PointManagerMutation {
        message: mutation_message("Updated", edited.len()),
        affected: edited.len(),
        total_points: stored.project.points.len(),
    })
}

fn delete_managed_points(
    host: &mut impl CadcraftHost,
    ids: &[Uuid],
) -> Result<PointManagerMutation, AdapterError> {
    let mut stored = required_drawing_project(host)?;
    let ids = validated_ids(&stored, ids, MAX_INSERT_POINTS, "delete points")?;
    let transaction = host
        .begin_transaction("Delete SurveyCraft points")
        .map_err(|error| AdapterError::new("begin point deletion", error, None))?;
    for links in stored
        .entity_links
        .iter()
        .filter(|links| ids.contains(&links.point_id))
    {
        for (_, entity) in links.roles() {
            let Some(entity) = entity else { continue };
            let exists = host
                .get_entity(entity)
                .map_err(|error| rollback_error(host, transaction, "inspect linked entity", error))?
                .is_some();
            if exists {
                host.delete_entity(transaction, entity).map_err(|error| {
                    rollback_error(host, transaction, "delete linked entity", error)
                })?;
            }
        }
    }
    stored
        .project
        .points
        .retain(|point| !ids.contains(&point.id));
    stored
        .entity_links
        .retain(|links| !ids.contains(&links.point_id));
    persist_and_commit(host, transaction, &stored, "delete points")?;
    Ok(PointManagerMutation {
        message: mutation_message("Deleted", ids.len()),
        affected: ids.len(),
        total_points: stored.project.points.len(),
    })
}

fn synchronize_managed_points(
    host: &mut impl CadcraftHost,
    ids: &[Uuid],
) -> Result<PointManagerMutation, AdapterError> {
    let mut stored = required_drawing_project(host)?;
    let ids = validated_ids(
        &stored,
        ids,
        MAX_MANAGER_MUTATION_POINTS,
        "synchronize points",
    )?;
    mutate_linked_points(host, &mut stored, &ids, "Synchronize SurveyCraft points")?;
    Ok(PointManagerMutation {
        message: mutation_message("Synchronized", ids.len()),
        affected: ids.len(),
        total_points: stored.project.points.len(),
    })
}

fn renumber_managed_points(
    host: &mut impl CadcraftHost,
    ids: &[Uuid],
    prefix: &str,
    start: u64,
    step: u64,
) -> Result<PointManagerMutation, AdapterError> {
    let mut stored = required_drawing_project(host)?;
    let ids = validated_ids(&stored, ids, MAX_MANAGER_MUTATION_POINTS, "renumber points")?;
    let prefix = prefix.trim();
    if prefix.len() > 64 || prefix.chars().any(char::is_control) || step == 0 {
        return Err(AdapterError::new(
            "validate point renumbering",
            HostError::new("prefix must be bounded printable text and step must be positive"),
            None,
        ));
    }
    let mut ordered_ids = stored
        .project
        .points
        .iter()
        .filter(|point| ids.contains(&point.id))
        .map(|point| (point.number.clone(), point.id))
        .collect::<Vec<_>>();
    ordered_ids.sort_by(|left, right| {
        compare_point_numbers(&left.0, &right.0).then_with(|| left.1.cmp(&right.1))
    });
    for (index, (_, id)) in ordered_ids.iter().enumerate() {
        let offset = step
            .checked_mul(u64::try_from(index).map_err(|_| {
                AdapterError::new(
                    "validate point renumbering",
                    HostError::new("renumber index is too large"),
                    None,
                )
            })?)
            .and_then(|offset| start.checked_add(offset))
            .ok_or_else(|| {
                AdapterError::new(
                    "validate point renumbering",
                    HostError::new("renumber sequence overflows"),
                    None,
                )
            })?;
        let number = format!("{prefix}{offset}");
        if number.len() > MAX_POINT_NUMBER_BYTES {
            return Err(AdapterError::new(
                "validate point renumbering",
                HostError::new("a generated point number is too long"),
                None,
            ));
        }
        let Some(point) = stored
            .project
            .points
            .iter_mut()
            .find(|point| point.id == *id)
        else {
            return Err(AdapterError::new(
                "validate point renumbering",
                HostError::new("point id is not in this drawing"),
                None,
            ));
        };
        point.number = number;
    }
    validate_unique_numbers(&stored.project)
        .map_err(|error| AdapterError::new("validate point renumbering", error, None))?;
    mutate_linked_points(host, &mut stored, &ids, "Renumber SurveyCraft points")?;
    Ok(PointManagerMutation {
        message: mutation_message("Renumbered", ids.len()),
        affected: ids.len(),
        total_points: stored.project.points.len(),
    })
}

fn export_managed_points(
    host: &mut impl CadcraftHost,
    ids: &[Uuid],
) -> Result<PointManagerExport, AdapterError> {
    let stored = required_drawing_project(host)?;
    let selected = if ids.is_empty() {
        BTreeSet::new()
    } else {
        validated_ids(&stored, ids, MAX_INSERT_POINTS, "export points")?
    };
    let bytes = surveycraft_io::export_point_csv(&stored.project, &selected).map_err(|error| {
        AdapterError::new("export points", HostError::new(error.to_string()), None)
    })?;
    let content = String::from_utf8(bytes).map_err(|error| {
        AdapterError::new("export points", HostError::new(error.to_string()), None)
    })?;
    let points = if selected.is_empty() {
        stored.project.points.len()
    } else {
        selected.len()
    };
    commit_read_only(host, "Export SurveyCraft points", "complete point export")?;
    Ok(PointManagerExport {
        file_name: "surveycraft-points.csv".into(),
        content,
        points,
    })
}

fn commit_read_only(
    host: &mut impl CadcraftHost,
    label: &str,
    operation: &'static str,
) -> Result<(), AdapterError> {
    let transaction = host
        .begin_transaction(label)
        .map_err(|error| AdapterError::new("begin read-only transaction", error, None))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(operation, error, rollback_error));
    }
    Ok(())
}

fn required_drawing_project(
    host: &mut impl CadcraftHost,
) -> Result<DrawingSurveyProject, AdapterError> {
    load_drawing_project(host)?.ok_or_else(|| {
        AdapterError::new(
            "load survey project",
            HostError::new("this drawing has no SurveyCraft point project"),
            None,
        )
    })
}

fn validated_ids(
    stored: &DrawingSurveyProject,
    ids: &[Uuid],
    maximum: usize,
    operation: &'static str,
) -> Result<BTreeSet<Uuid>, AdapterError> {
    if ids.is_empty() || ids.len() > maximum {
        return Err(AdapterError::new(
            operation,
            HostError::new(format!("the request must contain 1 to {maximum} point ids")),
            None,
        ));
    }
    let selected = ids.iter().copied().collect::<BTreeSet<_>>();
    if selected.len() != ids.len() {
        return Err(AdapterError::new(
            operation,
            HostError::new("one point id occurs more than once"),
            None,
        ));
    }
    if let Some(id) = selected
        .iter()
        .find(|id| !stored.project.points.iter().any(|point| point.id == **id))
    {
        return Err(AdapterError::new(
            operation,
            HostError::new(format!("point id {id} is not in this drawing")),
            None,
        ));
    }
    Ok(selected)
}

fn optional_manager_text(
    value: &str,
    maximum: usize,
    field: &str,
) -> Result<Option<String>, HostError> {
    let trimmed = value.trim();
    if trimmed.len() > maximum || trimmed.chars().any(char::is_control) {
        Err(HostError::new(format!(
            "{field} must contain at most {maximum} printable bytes"
        )))
    } else if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_owned()))
    }
}

fn validate_unique_numbers(project: &SurveyProject) -> Result<(), HostError> {
    let mut numbers = BTreeSet::new();
    for point in &project.points {
        let folded = point.number.trim().to_lowercase();
        if folded.is_empty() || !numbers.insert(folded) {
            return Err(HostError::new(format!(
                "point number '{}' is empty or duplicated",
                point.number
            )));
        }
    }
    Ok(())
}

fn compare_point_numbers(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.trim().parse::<i128>(), right.trim().parse::<i128>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.to_lowercase().cmp(&right.to_lowercase()),
    }
}

fn mutate_linked_points(
    host: &mut impl CadcraftHost,
    stored: &mut DrawingSurveyProject,
    ids: &BTreeSet<Uuid>,
    label: &str,
) -> Result<(), AdapterError> {
    let transaction = host
        .begin_transaction(label)
        .map_err(|error| AdapterError::new("begin point update", error, None))?;
    for id in ids {
        synchronize_linked_point(host, transaction, stored, *id)?;
    }
    validate_drawing_project(stored)
        .map_err(|error| rollback_error(host, transaction, "validate updated project", error))?;
    persist_and_commit(host, transaction, stored, "update points")
}

fn synchronize_linked_point(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    stored: &mut DrawingSurveyProject,
    point_id: Uuid,
) -> Result<(), AdapterError> {
    let point = stored
        .project
        .points
        .iter()
        .find(|point| point.id == point_id)
        .cloned()
        .ok_or_else(|| {
            rollback_error(
                host,
                transaction,
                "find survey point",
                HostError::new("point id is not in this drawing"),
            )
        })?;
    let link_index = stored
        .entity_links
        .iter()
        .position(|links| links.point_id == point_id)
        .ok_or_else(|| {
            rollback_error(
                host,
                transaction,
                "find point entity links",
                HostError::new("point has no entity-link record"),
            )
        })?;
    let links = stored
        .entity_links
        .get(link_index)
        .cloned()
        .ok_or_else(|| {
            rollback_error(
                host,
                transaction,
                "find point entity links",
                HostError::new("point link index is invalid"),
            )
        })?;
    let project_id = stored.project.id;
    let point_request = PointRequest {
        position: point.position,
        layer: stored.project.styles.point.layer.clone(),
    };
    let point_entity = ensure_point_entity(
        host,
        transaction,
        Some(links.point),
        &point_request,
        project_id,
        point_id,
    )?;
    let number_request = label_request(&point, PointEntityRole::NumberLabel, &stored.project)
        .map_err(|error| rollback_error(host, transaction, "prepare point number", error))?;
    let number_label = ensure_text_entity(
        host,
        transaction,
        Some(links.number_label),
        &number_request,
        project_id,
        point_id,
        PointEntityRole::NumberLabel,
    )?;
    let elevation_request = label_request(&point, PointEntityRole::ElevationLabel, &stored.project)
        .map_err(|error| rollback_error(host, transaction, "prepare point elevation", error))?;
    let elevation_label = ensure_text_entity(
        host,
        transaction,
        Some(links.elevation_label),
        &elevation_request,
        project_id,
        point_id,
        PointEntityRole::ElevationLabel,
    )?;
    let code_label = if point
        .code
        .as_deref()
        .is_some_and(|code| !code.trim().is_empty())
    {
        let code_request = label_request(&point, PointEntityRole::CodeLabel, &stored.project)
            .map_err(|error| rollback_error(host, transaction, "prepare point code", error))?;
        Some(ensure_text_entity(
            host,
            transaction,
            links.code_label,
            &code_request,
            project_id,
            point_id,
            PointEntityRole::CodeLabel,
        )?)
    } else {
        if let Some(entity) = links.code_label {
            delete_if_present(host, transaction, entity)?;
        }
        None
    };
    let Some(target) = stored.entity_links.get_mut(link_index) else {
        return Err(rollback_error(
            host,
            transaction,
            "store point entity links",
            HostError::new("point link index is invalid"),
        ));
    };
    *target = PointEntityLinks {
        point_id,
        point: point_entity,
        number_label,
        elevation_label,
        code_label,
    };
    Ok(())
}

fn ensure_point_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    existing: Option<EntityId>,
    request: &PointRequest,
    project_id: Uuid,
    point_id: Uuid,
) -> Result<EntityId, AdapterError> {
    let entity = match existing {
        Some(entity) => match host
            .get_entity(entity)
            .map_err(|error| rollback_error(host, transaction, "inspect linked point", error))?
        {
            Some(view) if view.kind == EntityKind::Point => {
                host.update_point(transaction, entity, request)
                    .map_err(|error| {
                        rollback_error(host, transaction, "update linked point", error)
                    })?;
                entity
            }
            Some(_) => {
                host.delete_entity(transaction, entity).map_err(|error| {
                    rollback_error(host, transaction, "replace linked point", error)
                })?;
                create_point_entity(host, transaction, request)?
            }
            None => create_point_entity(host, transaction, request)?,
        },
        None => create_point_entity(host, transaction, request)?,
    };
    tag_entity_if_needed(
        host,
        transaction,
        entity,
        project_id,
        point_id,
        PointEntityRole::Point,
    )?;
    Ok(entity)
}

fn ensure_text_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    existing: Option<EntityId>,
    request: &TextRequest,
    project_id: Uuid,
    point_id: Uuid,
    role: PointEntityRole,
) -> Result<EntityId, AdapterError> {
    let entity = match existing {
        Some(entity) => match host
            .get_entity(entity)
            .map_err(|error| rollback_error(host, transaction, "inspect linked label", error))?
        {
            Some(view) if view.kind == EntityKind::Text => {
                host.update_text(transaction, entity, request)
                    .map_err(|error| {
                        rollback_error(host, transaction, "update linked label", error)
                    })?;
                entity
            }
            Some(_) => {
                host.delete_entity(transaction, entity).map_err(|error| {
                    rollback_error(host, transaction, "replace linked label", error)
                })?;
                create_text_entity(host, transaction, request)?
            }
            None => create_text_entity(host, transaction, request)?,
        },
        None => create_text_entity(host, transaction, request)?,
    };
    tag_entity_if_needed(host, transaction, entity, project_id, point_id, role)?;
    Ok(entity)
}

fn tag_entity_if_needed(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    entity: EntityId,
    project_id: Uuid,
    point_id: Uuid,
    role: PointEntityRole,
) -> Result<(), AdapterError> {
    for (key, value) in [
        (PROJECT_ID_METADATA_KEY, project_id.to_string()),
        (POINT_ID_METADATA_KEY, point_id.to_string()),
        (ROLE_METADATA_KEY, role.metadata_value().to_owned()),
    ] {
        let current = host.get_entity_metadata(entity, key).map_err(|error| {
            rollback_error(host, transaction, "inspect survey entity metadata", error)
        })?;
        if current.as_deref() != Some(value.as_str()) {
            host.set_entity_metadata(transaction, entity, key, &value)
                .map_err(|error| {
                    rollback_error(host, transaction, "repair survey entity metadata", error)
                })?;
        }
    }
    Ok(())
}

fn delete_if_present(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    entity: EntityId,
) -> Result<(), AdapterError> {
    let exists = host
        .get_entity(entity)
        .map_err(|error| rollback_error(host, transaction, "inspect linked entity", error))?
        .is_some();
    if exists {
        host.delete_entity(transaction, entity)
            .map_err(|error| rollback_error(host, transaction, "delete linked entity", error))?;
    }
    Ok(())
}

fn label_request(
    survey_point: &SurveyPoint,
    role: PointEntityRole,
    project: &SurveyProject,
) -> Result<TextRequest, HostError> {
    let labels = &project.styles.labels;
    let (position, value, layer) = match role {
        PointEntityRole::NumberLabel => (
            point(
                survey_point.position.x() + labels.offset_x,
                survey_point.position.y() + labels.offset_y,
                survey_point.position.z(),
            ),
            survey_point.number.clone(),
            labels.number_layer.clone(),
        ),
        PointEntityRole::ElevationLabel => (
            point(
                survey_point.position.x() + labels.offset_x,
                survey_point.position.y() - labels.offset_y,
                survey_point.position.z(),
            ),
            format!(
                "{:.*}",
                usize::from(labels.elevation_decimals),
                survey_point.position.z()
            ),
            labels.elevation_layer.clone(),
        ),
        PointEntityRole::CodeLabel => (
            point(
                survey_point.position.x() + labels.offset_x,
                survey_point.position.y() - labels.offset_y * 2.4,
                survey_point.position.z(),
            ),
            survey_point.code.clone().unwrap_or_default(),
            labels.code_layer.clone(),
        ),
        PointEntityRole::Point => {
            return Err(HostError::new("point geometry is not a text label"));
        }
    };
    Ok(TextRequest {
        position: position?,
        value,
        height: labels.height,
        rotation_degrees: 0.0,
        layer,
    })
}

fn persist_and_commit(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    stored: &DrawingSurveyProject,
    operation: &'static str,
) -> Result<(), AdapterError> {
    validate_drawing_project(stored)
        .map_err(|error| rollback_error(host, transaction, "validate survey project", error))?;
    let metadata = serde_json::to_string(stored).map_err(|error| {
        rollback_error(
            host,
            transaction,
            "encode survey project",
            HostError::new(error.to_string()),
        )
    })?;
    host.set_drawing_metadata(transaction, PROJECT_METADATA_KEY, &metadata)
        .map_err(|error| rollback_error(host, transaction, "persist survey project", error))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(operation, error, rollback_error));
    }
    Ok(())
}

fn load_drawing_project(
    host: &mut impl CadcraftHost,
) -> Result<Option<DrawingSurveyProject>, AdapterError> {
    let Some(json) = host
        .get_drawing_metadata(PROJECT_METADATA_KEY)
        .map_err(|error| AdapterError::new("read survey project", error, None))?
    else {
        return Ok(None);
    };
    let mut stored: DrawingSurveyProject = serde_json::from_str(&json).map_err(|error| {
        AdapterError::new(
            "decode survey project",
            HostError::new(error.to_string()),
            None,
        )
    })?;
    if stored.schema_version != DRAWING_PROJECT_SCHEMA_VERSION {
        return Err(AdapterError::new(
            "decode survey project",
            HostError::new(format!(
                "unsupported drawing metadata schema {}",
                stored.schema_version
            )),
            None,
        ));
    }
    stored.project = migrate_project(stored.project).map_err(|error| {
        AdapterError::new(
            "migrate survey project",
            HostError::new(error.to_string()),
            None,
        )
    })?;
    validate_drawing_project(&stored)
        .map_err(|error| AdapterError::new("validate entity links", error, None))?;
    Ok(Some(stored))
}

fn new_drawing_project(request: &InsertPointsRequest) -> Result<DrawingSurveyProject, HostError> {
    let project_id = initial_project_id(request);
    let mut project = SurveyProject::new(
        project_id,
        "SurveyCraft drawing",
        Crs::local_engineering(),
        LinearUnit::Metre,
        NumericPolicy::default(),
    )
    .map_err(|error| HostError::new(error.to_string()))?;
    project.styles.labels.height = request.label_height;
    project.styles.labels.offset_x = request.label_height * 1.4;
    project.styles.labels.offset_y = request.label_height * 1.4;
    Ok(DrawingSurveyProject {
        schema_version: DRAWING_PROJECT_SCHEMA_VERSION,
        project,
        entity_links: Vec::new(),
    })
}

fn initial_project_id(request: &InsertPointsRequest) -> Uuid {
    let mut seed = String::from("surveycraft-drawing-v2");
    for row in &request.rows {
        seed.push('|');
        seed.push_str(&row.number.trim().to_lowercase());
        seed.push(':');
        seed.push_str(&format!(
            "{:016x}:{:016x}:{:016x}",
            row.x.to_bits(),
            row.y.to_bits(),
            row.elevation.to_bits()
        ));
    }
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes())
}

fn drawing_point_id(project_id: Uuid, number: &str) -> Uuid {
    Uuid::new_v5(
        &project_id,
        format!("drawing-point:{}", number.trim().to_lowercase()).as_bytes(),
    )
}

fn reject_existing_numbers(
    project: &SurveyProject,
    request: &InsertPointsRequest,
) -> Result<(), HostError> {
    let existing: BTreeSet<String> = project
        .points
        .iter()
        .map(|point| point.number.to_lowercase())
        .collect();
    if let Some(number) = request
        .rows
        .iter()
        .map(|row| row.number.trim())
        .find(|number| existing.contains(&number.to_lowercase()))
    {
        Err(HostError::new(format!(
            "point number '{number}' already exists in this drawing"
        )))
    } else {
        Ok(())
    }
}

fn validate_drawing_project(stored: &DrawingSurveyProject) -> Result<(), HostError> {
    if stored.schema_version != DRAWING_PROJECT_SCHEMA_VERSION {
        return Err(HostError::new(format!(
            "unsupported drawing metadata schema {}",
            stored.schema_version
        )));
    }
    stored
        .project
        .validate()
        .map_err(|error| HostError::new(error.to_string()))?;
    if stored.project.points.len() != stored.entity_links.len() {
        return Err(HostError::new(
            "every survey point must have exactly one entity-link record",
        ));
    }
    let point_ids: HashSet<Uuid> = stored.project.points.iter().map(|point| point.id).collect();
    let mut linked_points = HashSet::new();
    let mut entities = BTreeSet::new();
    for links in &stored.entity_links {
        if !point_ids.contains(&links.point_id) || !linked_points.insert(links.point_id) {
            return Err(HostError::new(
                "entity links contain an unknown or duplicate point id",
            ));
        }
        for (_, entity) in links.roles() {
            if let Some(entity) = entity
                && !entities.insert(entity)
            {
                return Err(HostError::new(
                    "one CAD entity is linked to more than one point role",
                ));
            }
        }
    }
    Ok(())
}

fn tag_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    entity: EntityId,
    project_id: Uuid,
    point_id: Uuid,
    role: PointEntityRole,
) -> Result<(), AdapterError> {
    for (key, value) in [
        (PROJECT_ID_METADATA_KEY, project_id.to_string()),
        (POINT_ID_METADATA_KEY, point_id.to_string()),
        (ROLE_METADATA_KEY, role.metadata_value().to_owned()),
    ] {
        host.set_entity_metadata(transaction, entity, key, &value)
            .map_err(|error| rollback_error(host, transaction, "tag survey entity", error))?;
    }
    Ok(())
}

fn reconcile(
    host: &mut impl CadcraftHost,
    stored: &DrawingSurveyProject,
) -> Result<ReconciliationReport, AdapterError> {
    let by_id: BTreeMap<Uuid, &SurveyPoint> = stored
        .project
        .points
        .iter()
        .map(|point| (point.id, point))
        .collect();
    let linked_entities: BTreeSet<EntityId> = stored
        .entity_links
        .iter()
        .flat_map(|links| links.roles().into_iter().filter_map(|(_, id)| id))
        .collect();
    let mut issues = Vec::new();
    let mut unhealthy = HashSet::new();
    let mut missing_entities = 0usize;
    let mut modified_entities = 0usize;
    for links in &stored.entity_links {
        let Some(point) = by_id.get(&links.point_id).copied() else {
            continue;
        };
        for (role, entity_id) in links.roles() {
            let Some(entity_id) = entity_id else { continue };
            let actual = host
                .get_entity(entity_id)
                .map_err(|error| AdapterError::new("read linked entity", error, None))?;
            let Some(actual) = actual else {
                missing_entities += 1;
                unhealthy.insert(point.id);
                issues.push(ReconciliationIssue {
                    point_id: point.id,
                    role,
                    entity: entity_id,
                    kind: ReconciliationIssueKind::MissingEntity,
                });
                continue;
            };
            let expected = expected_entity(point, role, &stored.project);
            let before = issues.len();
            compare_entity(
                point.id,
                role,
                entity_id,
                &actual,
                &expected,
                stored.project.numeric_policy.coordinate_tolerance,
                &mut issues,
            );
            let point_id_value = point.id.to_string();
            let point_metadata = host
                .get_entity_metadata(entity_id, POINT_ID_METADATA_KEY)
                .map_err(|error| AdapterError::new("read linked entity metadata", error, None))?;
            let role_value = host
                .get_entity_metadata(entity_id, ROLE_METADATA_KEY)
                .map_err(|error| AdapterError::new("read linked entity metadata", error, None))?;
            if point_metadata.as_deref() != Some(point_id_value.as_str())
                || role_value.as_deref() != Some(role.metadata_value())
            {
                issues.push(ReconciliationIssue {
                    point_id: point.id,
                    role,
                    entity: entity_id,
                    kind: ReconciliationIssueKind::MetadataChanged,
                });
            }
            if issues.len() > before {
                modified_entities += 1;
                unhealthy.insert(point.id);
            }
        }
    }
    let project_id = stored.project.id.to_string();
    let orphaned_entities = host
        .find_entities_by_metadata(PROJECT_ID_METADATA_KEY, &project_id)
        .map_err(|error| AdapterError::new("find survey entities", error, None))?
        .into_iter()
        .filter(|entity| !linked_entities.contains(&entity.id))
        .count();
    Ok(ReconciliationReport {
        project_id: Some(stored.project.id),
        points: stored.project.points.len(),
        healthy_points: stored.project.points.len().saturating_sub(unhealthy.len()),
        missing_entities,
        modified_entities,
        orphaned_entities,
        issues,
    })
}

fn expected_entity(
    point: &SurveyPoint,
    role: PointEntityRole,
    project: &SurveyProject,
) -> EntityView {
    let labels = &project.styles.labels;
    let position = point.position;
    match role {
        PointEntityRole::Point => EntityView {
            id: EntityId(0),
            kind: EntityKind::Point,
            layer: project.styles.point.layer.clone(),
            position: Some(position),
            value: None,
        },
        PointEntityRole::NumberLabel => EntityView {
            id: EntityId(0),
            kind: EntityKind::Text,
            layer: labels.number_layer.clone(),
            position: offset_point(position, labels.offset_x, labels.offset_y),
            value: Some(point.number.clone()),
        },
        PointEntityRole::ElevationLabel => EntityView {
            id: EntityId(0),
            kind: EntityKind::Text,
            layer: labels.elevation_layer.clone(),
            position: offset_point(position, labels.offset_x, -labels.offset_y),
            value: Some(format!(
                "{:.*}",
                usize::from(labels.elevation_decimals),
                position.z()
            )),
        },
        PointEntityRole::CodeLabel => EntityView {
            id: EntityId(0),
            kind: EntityKind::Text,
            layer: labels.code_layer.clone(),
            position: offset_point(position, labels.offset_x, -labels.offset_y * 2.4),
            value: point.code.clone(),
        },
    }
}

fn offset_point(position: Point3, dx: f64, dy: f64) -> Option<Point3> {
    Point3::new(position.x() + dx, position.y() + dy, position.z()).ok()
}

fn compare_entity(
    point_id: Uuid,
    role: PointEntityRole,
    entity_id: EntityId,
    actual: &EntityView,
    expected: &EntityView,
    tolerance: f64,
    issues: &mut Vec<ReconciliationIssue>,
) {
    let mut note = |kind| {
        issues.push(ReconciliationIssue {
            point_id,
            role,
            entity: entity_id,
            kind,
        })
    };
    if actual.kind != expected.kind {
        note(ReconciliationIssueKind::WrongEntityKind);
        return;
    }
    if !actual.layer.eq_ignore_ascii_case(&expected.layer) {
        note(ReconciliationIssueKind::LayerChanged);
    }
    if let (Some(actual), Some(expected)) = (actual.position, expected.position) {
        let changed = (actual.x() - expected.x()).abs() > tolerance
            || (actual.y() - expected.y()).abs() > tolerance
            || (actual.z() - expected.z()).abs() > tolerance;
        if changed {
            note(ReconciliationIssueKind::GeometryChanged);
        }
    }
    if actual.value != expected.value {
        note(ReconciliationIssueKind::TextChanged);
    }
}

fn cad_xy(row: &PointRow, convention: CoordinateConvention) -> (f64, f64) {
    match convention {
        CoordinateConvention::CadXy => (row.x, row.y),
        CoordinateConvention::SurveyXy => (row.y, row.x),
    }
}

fn validate_insert_request(request: &InsertPointsRequest) -> Result<(), HostError> {
    if request.rows.is_empty() || request.rows.len() > MAX_INSERT_POINTS {
        return Err(HostError::new(format!(
            "point insertion requires 1 to {MAX_INSERT_POINTS} rows"
        )));
    }
    if !request.label_height.is_finite()
        || request.label_height <= 0.0
        || request.label_height > MAX_COORDINATE
    {
        return Err(HostError::new("label height must be finite and positive"));
    }
    let mut numbers = BTreeSet::new();
    for (index, row) in request.rows.iter().enumerate() {
        let number = row.number.trim();
        if number.is_empty()
            || number.len() > MAX_POINT_NUMBER_BYTES
            || number.chars().any(char::is_control)
        {
            return Err(HostError::new(format!(
                "row {} has an invalid point number",
                index + 1
            )));
        }
        if !numbers.insert(number.to_lowercase()) {
            return Err(HostError::new(format!(
                "point number '{number}' occurs more than once"
            )));
        }
        if row.code.len() > MAX_CODE_BYTES || row.code.chars().any(char::is_control) {
            return Err(HostError::new(format!(
                "row {} has an invalid code",
                index + 1
            )));
        }
        let (cad_x, cad_y) = cad_xy(row, request.convention);
        point(cad_x, cad_y, row.elevation)?;
    }
    Ok(())
}

fn point(cad_x: f64, cad_y: f64, elevation: f64) -> Result<Point3, HostError> {
    if [cad_x, cad_y, elevation]
        .iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE)
    {
        return Err(HostError::new(
            "coordinates must be finite and within the supported drawing range",
        ));
    }
    Point3::new(cad_x, cad_y, elevation).map_err(|error| HostError::new(error.to_string()))
}

fn create_point_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    request: &PointRequest,
) -> Result<EntityId, AdapterError> {
    host.create_point(transaction, request)
        .map_err(|error| rollback_error(host, transaction, "create point", error))
}

fn create_text_entity(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    request: &TextRequest,
) -> Result<EntityId, AdapterError> {
    host.create_text(transaction, request)
        .map_err(|error| rollback_error(host, transaction, "create point label", error))
}

fn rollback_error(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    operation: &'static str,
    error: HostError,
) -> AdapterError {
    let rollback_error = host.rollback_transaction(transaction).err();
    AdapterError::new(operation, error, rollback_error)
}

/// Proves one entity can cross the host boundary in a guarded transaction.
pub fn run_reference_extension(
    host: &mut impl CadcraftHost,
    request: &LineRequest,
) -> Result<EntityId, AdapterError> {
    let transaction = host
        .begin_transaction("SurveyCraft reference line")
        .map_err(|error| AdapterError::new("begin transaction", error, None))?;
    let entity = host
        .create_line(transaction, request)
        .map_err(|error| rollback_error(host, transaction, "create line", error))?;
    if let Err(error) = host.commit_transaction(transaction) {
        let rollback_error = host.rollback_transaction(transaction).err();
        return Err(AdapterError::new(
            "commit transaction",
            error,
            rollback_error,
        ));
    }
    Ok(entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct MockHost {
        events: Vec<&'static str>,
        entities: BTreeMap<EntityId, EntityView>,
        entity_metadata: BTreeMap<EntityId, BTreeMap<String, String>>,
        drawing_metadata: BTreeMap<String, String>,
        next_id: u64,
        fail_create: bool,
    }

    impl MockHost {
        fn allocate(
            &mut self,
            kind: EntityKind,
            layer: &str,
            position: Option<Point3>,
            value: Option<String>,
        ) -> EntityId {
            self.next_id = self.next_id.max(100);
            let id = EntityId(self.next_id);
            self.next_id = self.next_id.saturating_add(1);
            self.entities.insert(
                id,
                EntityView {
                    id,
                    kind,
                    layer: layer.to_owned(),
                    position,
                    value,
                },
            );
            id
        }
    }

    impl CadcraftHost for MockHost {
        fn begin_transaction(&mut self, _label: &str) -> Result<TransactionId, HostError> {
            self.events.push("begin");
            Ok(TransactionId(1))
        }

        fn create_line(
            &mut self,
            _transaction: TransactionId,
            request: &LineRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(self.allocate(EntityKind::Line, &request.layer, None, None))
            }
        }

        fn create_arc(
            &mut self,
            _transaction: TransactionId,
            request: &ArcRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-arc");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(self.allocate(EntityKind::Arc, &request.layer, Some(request.center), None))
            }
        }

        fn create_circle(
            &mut self,
            _transaction: TransactionId,
            request: &CircleRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-circle");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(self.allocate(
                    EntityKind::Circle,
                    &request.layer,
                    Some(request.center),
                    None,
                ))
            }
        }

        fn create_point(
            &mut self,
            _transaction: TransactionId,
            request: &PointRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-point");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(self.allocate(
                    EntityKind::Point,
                    &request.layer,
                    Some(request.position),
                    None,
                ))
            }
        }

        fn create_text(
            &mut self,
            _transaction: TransactionId,
            request: &TextRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-text");
            Ok(self.allocate(
                EntityKind::Text,
                &request.layer,
                Some(request.position),
                Some(request.value.clone()),
            ))
        }

        fn get_drawing_metadata(&mut self, key: &str) -> Result<Option<String>, HostError> {
            Ok(self.drawing_metadata.get(key).cloned())
        }

        fn set_drawing_metadata(
            &mut self,
            _transaction: TransactionId,
            key: &str,
            value: &str,
        ) -> Result<(), HostError> {
            self.drawing_metadata
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        fn remove_drawing_metadata(
            &mut self,
            _transaction: TransactionId,
            key: &str,
        ) -> Result<(), HostError> {
            self.drawing_metadata.remove(key);
            Ok(())
        }

        fn get_entity(&mut self, entity: EntityId) -> Result<Option<EntityView>, HostError> {
            Ok(self.entities.get(&entity).cloned())
        }

        fn get_entity_metadata(
            &mut self,
            entity: EntityId,
            key: &str,
        ) -> Result<Option<String>, HostError> {
            Ok(self
                .entity_metadata
                .get(&entity)
                .and_then(|data| data.get(key))
                .cloned())
        }

        fn find_entities_by_metadata(
            &mut self,
            key: &str,
            value: &str,
        ) -> Result<Vec<EntityView>, HostError> {
            Ok(self
                .entity_metadata
                .iter()
                .filter(|(_, data)| data.get(key).is_some_and(|stored| stored == value))
                .filter_map(|(id, _)| self.entities.get(id).cloned())
                .collect())
        }

        fn set_entity_metadata(
            &mut self,
            _transaction: TransactionId,
            entity: EntityId,
            key: &str,
            value: &str,
        ) -> Result<(), HostError> {
            self.entity_metadata
                .entry(entity)
                .or_default()
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        fn remove_entity_metadata(
            &mut self,
            _transaction: TransactionId,
            entity: EntityId,
            key: &str,
        ) -> Result<(), HostError> {
            if let Some(data) = self.entity_metadata.get_mut(&entity) {
                data.remove(key);
            }
            Ok(())
        }

        fn update_point(
            &mut self,
            _transaction: TransactionId,
            entity: EntityId,
            request: &PointRequest,
        ) -> Result<(), HostError> {
            let Some(view) = self.entities.get_mut(&entity) else {
                return Err(HostError::new("missing entity"));
            };
            view.kind = EntityKind::Point;
            view.layer = request.layer.clone();
            view.position = Some(request.position);
            Ok(())
        }

        fn update_text(
            &mut self,
            _transaction: TransactionId,
            entity: EntityId,
            request: &TextRequest,
        ) -> Result<(), HostError> {
            let Some(view) = self.entities.get_mut(&entity) else {
                return Err(HostError::new("missing entity"));
            };
            view.kind = EntityKind::Text;
            view.layer = request.layer.clone();
            view.position = Some(request.position);
            view.value = Some(request.value.clone());
            Ok(())
        }

        fn delete_entity(
            &mut self,
            _transaction: TransactionId,
            entity: EntityId,
        ) -> Result<(), HostError> {
            self.entities.remove(&entity);
            Ok(())
        }

        fn commit_transaction(&mut self, _transaction: TransactionId) -> Result<(), HostError> {
            self.events.push("commit");
            Ok(())
        }

        fn rollback_transaction(&mut self, _transaction: TransactionId) -> Result<(), HostError> {
            self.events.push("rollback");
            Ok(())
        }
    }

    fn request() -> InsertPointsRequest {
        InsertPointsRequest {
            rows: vec![PointRow {
                number: "101".into(),
                x: 500.0,
                y: 200.0,
                elevation: 12.5,
                code: "STN".into(),
            }],
            convention: CoordinateConvention::CadXy,
            label_height: 0.25,
        }
    }

    #[test]
    fn inserts_and_persists_point_identity() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let result = insert_points(&mut host, &request())?;
        assert_eq!(result.inserted, 1);
        assert_eq!(result.entities, 4);
        assert_eq!(result.total_points, 1);
        assert!(host.drawing_metadata.contains_key(PROJECT_METADATA_KEY));

        let report = inspect_project(&mut host)?;
        assert_eq!(report.points, 1);
        assert_eq!(report.healthy_points, 1);
        assert!(report.issues.is_empty());
        Ok(())
    }

    #[test]
    fn reconciliation_reports_deleted_and_edited_entities() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        insert_points(&mut host, &request())?;
        let metadata = host
            .drawing_metadata
            .get(PROJECT_METADATA_KEY)
            .cloned()
            .ok_or("missing project metadata")?;
        let stored: DrawingSurveyProject = serde_json::from_str(&metadata)?;
        let links = stored.entity_links.first().ok_or("missing links")?;
        host.entities.remove(&links.number_label);
        if let Some(point) = host.entities.get_mut(&links.point) {
            point.position = Some(Point3::new(501.0, 200.0, 12.5)?);
        }

        let report = inspect_project(&mut host)?;
        assert_eq!(report.healthy_points, 0);
        assert_eq!(report.missing_entities, 1);
        assert!(report.modified_entities >= 1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == ReconciliationIssueKind::MissingEntity)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.kind == ReconciliationIssueKind::GeometryChanged)
        );
        Ok(())
    }

    #[test]
    fn rejects_existing_point_numbers_without_creating_entities() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        insert_points(&mut host, &request())?;
        let before = host.entities.len();
        let result = insert_points(&mut host, &request());
        assert!(result.is_err());
        assert_eq!(host.entities.len(), before);
        Ok(())
    }

    #[test]
    fn survey_y_becomes_cad_x() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let mut input = request();
        input.convention = CoordinateConvention::SurveyXy;
        insert_points(&mut host, &input)?;
        let point = host
            .entities
            .values()
            .find(|entity| entity.kind == EntityKind::Point)
            .and_then(|entity| entity.position);
        assert_eq!(point, Some(Point3::new(200.0, 500.0, 12.5)?));
        Ok(())
    }

    #[test]
    fn point_manager_edits_geometry_labels_and_exports() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        insert_points(&mut host, &request())?;
        let listed = manage_points(&mut host, &PointManagerRequest::List)?;
        let PointManagerResult::Snapshot(snapshot) = listed else {
            return Err("point manager did not return a snapshot".into());
        };
        let Some(row) = snapshot.rows.first() else {
            return Err("point manager returned no rows".into());
        };
        let id = row.id;

        let result = manage_points(
            &mut host,
            &PointManagerRequest::Update {
                rows: vec![PointManagerEdit {
                    id,
                    number: "201".into(),
                    x: 700.0,
                    y: 300.0,
                    elevation: 15.25,
                    code: String::new(),
                    description: "Edited station".into(),
                }],
            },
        )?;
        let PointManagerResult::Mutation(mutation) = result else {
            return Err("point manager did not return a mutation summary".into());
        };
        assert_eq!(mutation.affected, 1);

        let listed = manage_points(&mut host, &PointManagerRequest::List)?;
        let PointManagerResult::Snapshot(snapshot) = listed else {
            return Err("point manager did not return an updated snapshot".into());
        };
        let Some(row) = snapshot.rows.first() else {
            return Err("updated point manager returned no rows".into());
        };
        assert_eq!(row.number, "201");
        assert_eq!((row.x, row.y, row.elevation), (700.0, 300.0, 15.25));
        assert_eq!(row.description, "Edited station");
        assert!(row.code.is_empty());
        assert_eq!(row.status, "Healthy");
        assert_eq!(row.entities.len(), 3);

        let exported = manage_points(&mut host, &PointManagerRequest::Export { ids: vec![id] })?;
        let PointManagerResult::Export(export) = exported else {
            return Err("point manager did not return an export".into());
        };
        assert!(export.content.contains("201,700,300,15.25,,Edited station"));
        Ok(())
    }

    #[test]
    fn point_manager_renumbers_repairs_and_deletes() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let mut input = request();
        input.rows.push(PointRow {
            number: "102".into(),
            x: 510.0,
            y: 210.0,
            elevation: 13.5,
            code: "EDGE".into(),
        });
        insert_points(&mut host, &input)?;
        let metadata = host
            .drawing_metadata
            .get(PROJECT_METADATA_KEY)
            .cloned()
            .ok_or("missing project metadata")?;
        let stored: DrawingSurveyProject = serde_json::from_str(&metadata)?;
        let first = stored
            .entity_links
            .first()
            .ok_or("missing first point links")?;
        let old_number_label = first.number_label;
        host.entities.remove(&old_number_label);

        manage_points(
            &mut host,
            &PointManagerRequest::Synchronize {
                ids: vec![first.point_id],
            },
        )?;
        let metadata = host
            .drawing_metadata
            .get(PROJECT_METADATA_KEY)
            .cloned()
            .ok_or("missing repaired metadata")?;
        let repaired: DrawingSurveyProject = serde_json::from_str(&metadata)?;
        let repaired_first = repaired
            .entity_links
            .first()
            .ok_or("missing repaired links")?;
        assert_ne!(repaired_first.number_label, old_number_label);
        assert_eq!(inspect_project(&mut host)?.healthy_points, 2);

        let ids = repaired
            .project
            .points
            .iter()
            .map(|point| point.id)
            .collect::<Vec<_>>();
        manage_points(
            &mut host,
            &PointManagerRequest::Renumber {
                ids: ids.clone(),
                prefix: "P-".into(),
                start: 10,
                step: 10,
            },
        )?;
        let listed = manage_points(&mut host, &PointManagerRequest::List)?;
        let PointManagerResult::Snapshot(snapshot) = listed else {
            return Err("point manager did not return a renumbered snapshot".into());
        };
        assert_eq!(
            snapshot
                .rows
                .iter()
                .map(|row| row.number.as_str())
                .collect::<Vec<_>>(),
            ["P-10", "P-20"]
        );

        let delete_id = *ids.first().ok_or("missing point id")?;
        manage_points(
            &mut host,
            &PointManagerRequest::Delete {
                ids: vec![delete_id],
            },
        )?;
        let listed = manage_points(&mut host, &PointManagerRequest::List)?;
        let PointManagerResult::Snapshot(snapshot) = listed else {
            return Err("point manager did not return a post-delete snapshot".into());
        };
        assert_eq!(snapshot.points, 1);
        assert_eq!(snapshot.healthy_points, 1);
        Ok(())
    }

    #[test]
    fn point_manager_rejects_duplicate_numbers_before_mutation() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let mut input = request();
        input.rows.push(PointRow {
            number: "102".into(),
            x: 510.0,
            y: 210.0,
            elevation: 13.5,
            code: String::new(),
        });
        insert_points(&mut host, &input)?;
        let metadata = host
            .drawing_metadata
            .get(PROJECT_METADATA_KEY)
            .cloned()
            .ok_or("missing project metadata")?;
        let stored: DrawingSurveyProject = serde_json::from_str(&metadata)?;
        let second = stored.project.points.get(1).ok_or("missing second point")?;
        let result = manage_points(
            &mut host,
            &PointManagerRequest::Update {
                rows: vec![PointManagerEdit {
                    id: second.id,
                    number: "101".into(),
                    x: second.position.x(),
                    y: second.position.y(),
                    elevation: second.position.z(),
                    code: String::new(),
                    description: String::new(),
                }],
            },
        );
        assert!(result.is_err());
        assert_eq!(
            host.events
                .iter()
                .filter(|event| **event == "begin")
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn code_book_manager_previews_and_persists_rule_edits() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        insert_points(&mut host, &request())?;
        let listed = manage_code_book(&mut host, &CodeBookManagerRequest::List)?;
        let CodeBookManagerResult::Snapshot(snapshot) = listed else {
            return Err("code-book manager did not return rows".into());
        };
        assert_eq!(snapshot.rows.len(), 8);
        assert!(snapshot.summary.contains("1 coded points"));
        let row = snapshot
            .rows
            .iter()
            .find(|row| row.rule_id == "station")
            .ok_or("missing station rule")?;
        assert_eq!(row.status, "1 point");
        let mut edit = CodeRuleEdit {
            record: row.record.clone(),
            rule_id: row.rule_id.clone(),
            pattern: row.pattern.clone(),
            match_mode: row.match_mode.clone(),
            description: "Edited station rule".into(),
            optimization_rank: f64::from(row.optimization_rank),
            layer: row.layer.clone(),
            graphic_kind: row.graphic_kind.clone(),
            style_or_symbol: row.style_or_symbol.clone(),
            scale_or_width: row.scale_or_width,
            attribute_from_suffix: row.attribute_from_suffix.clone(),
        };
        let preview = manage_code_book(
            &mut host,
            &CodeBookManagerRequest::Preview {
                rows: snapshot
                    .rows
                    .iter()
                    .map(|row| CodeRuleEdit {
                        record: row.record.clone(),
                        rule_id: row.rule_id.clone(),
                        pattern: row.pattern.clone(),
                        match_mode: row.match_mode.clone(),
                        description: if row.rule_id == "station" {
                            edit.description.clone()
                        } else {
                            row.description.clone()
                        },
                        optimization_rank: f64::from(row.optimization_rank),
                        layer: row.layer.clone(),
                        graphic_kind: row.graphic_kind.clone(),
                        style_or_symbol: row.style_or_symbol.clone(),
                        scale_or_width: row.scale_or_width,
                        attribute_from_suffix: row.attribute_from_suffix.clone(),
                    })
                    .collect(),
            },
        )?;
        let CodeBookManagerResult::Snapshot(preview) = preview else {
            return Err("code-book preview did not return rows".into());
        };
        assert!(
            preview
                .rows
                .iter()
                .any(|row| row.description == edit.description)
        );
        edit.pattern = "STATION".into();
        manage_code_book(
            &mut host,
            &CodeBookManagerRequest::Update { rows: vec![edit] },
        )?;
        let mut reopened = host.clone();
        let listed = manage_code_book(&mut reopened, &CodeBookManagerRequest::List)?;
        let CodeBookManagerResult::Snapshot(snapshot) = listed else {
            return Err("reopened code-book manager did not return rows".into());
        };
        assert!(
            snapshot
                .rows
                .iter()
                .any(|row| row.rule_id == "station" && row.pattern == "STATION")
        );
        Ok(())
    }

    #[test]
    fn coded_features_are_native_linked_and_replaced_transactionally() -> Result<(), Box<dyn Error>>
    {
        let mut host = MockHost::default();
        let rows = vec![
            PointRow {
                number: "1".into(),
                x: 0.0,
                y: 0.0,
                elevation: 0.0,
                code: "EDG-START".into(),
            },
            PointRow {
                number: "2".into(),
                x: 10.0,
                y: 0.0,
                elevation: 0.0,
                code: "EDG-ARC_START".into(),
            },
            PointRow {
                number: "3".into(),
                x: 10.0,
                y: 10.0,
                elevation: 0.0,
                code: "EDG".into(),
            },
            PointRow {
                number: "4".into(),
                x: 0.0,
                y: 10.0,
                elevation: 0.0,
                code: "EDG-ARC_END-CLOSE".into(),
            },
            PointRow {
                number: "10".into(),
                x: 20.0,
                y: 0.0,
                elevation: 0.0,
                code: "EDG-ON_CIRCLE".into(),
            },
            PointRow {
                number: "11".into(),
                x: 25.0,
                y: 5.0,
                elevation: 0.0,
                code: "EDG-ON_CIRCLE".into(),
            },
            PointRow {
                number: "12".into(),
                x: 30.0,
                y: 0.0,
                elevation: 0.0,
                code: "EDG-ON_CIRCLE".into(),
            },
            PointRow {
                number: "20".into(),
                x: 5.0,
                y: 5.0,
                elevation: 0.0,
                code: "TREE".into(),
            },
        ];
        insert_points(
            &mut host,
            &InsertPointsRequest {
                rows,
                convention: CoordinateConvention::CadXy,
                label_height: 0.25,
            },
        )?;
        let first = generate_coded_features(&mut host, GenerateCodedFeaturesRequest::default())?;
        assert_eq!((first.paths, first.circles, first.symbols), (1, 1, 1));
        assert!(first.entities >= 10);
        assert_eq!(first.replaced_entities, 0);
        let linked =
            host.find_entities_by_metadata(FEATURE_SET_METADATA_KEY, FEATURE_SET_METADATA_VALUE)?;
        assert_eq!(linked.len(), first.entities);
        assert!(linked.iter().any(|entity| entity.kind == EntityKind::Arc));
        assert!(
            linked
                .iter()
                .any(|entity| entity.kind == EntityKind::Circle)
        );

        let second = generate_coded_features(&mut host, GenerateCodedFeaturesRequest::default())?;
        assert_eq!(second.replaced_entities, first.entities);
        let linked =
            host.find_entities_by_metadata(FEATURE_SET_METADATA_KEY, FEATURE_SET_METADATA_VALUE)?;
        assert_eq!(linked.len(), second.entities);
        assert_eq!(host.events.last(), Some(&"commit"));
        Ok(())
    }

    #[test]
    fn reference_extension_rolls_back_on_create_failure() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost {
            fail_create: true,
            ..MockHost::default()
        };
        let line = LineRequest {
            start: Point3::new(0.0, 0.0, 0.0)?,
            end: Point3::new(1.0, 1.0, 0.0)?,
            layer: "0".into(),
        };
        assert!(run_reference_extension(&mut host, &line).is_err());
        assert_eq!(host.events, ["begin", "create", "rollback"]);
        Ok(())
    }
}
