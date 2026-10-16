//! Narrow, testable boundary between SurveyCraft commands and a CAD host.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_core::{
    Crs, LinearUnit, NumericPolicy, SourceRef, SurveyPoint, SurveyProject, migrate_project,
};
use surveycraft_geometry::Point3;
use uuid::Uuid;

const DRAWING_PROJECT_SCHEMA_VERSION: u32 = 1;
const PROJECT_METADATA_KEY: &str = "survey-project";
const POINT_ID_METADATA_KEY: &str = "point-id";
const PROJECT_ID_METADATA_KEY: &str = "project-id";
const ROLE_METADATA_KEY: &str = "role";
const MAX_INSERT_POINTS: usize = 2_000;
const MAX_COORDINATE: f64 = 1.0e15;
const MAX_POINT_NUMBER_BYTES: usize = 128;
const MAX_CODE_BYTES: usize = 256;

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
