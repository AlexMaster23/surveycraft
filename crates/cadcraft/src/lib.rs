//! Narrow, testable boundary between SurveyCraft commands and a CAD host.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_geometry::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointRow {
    pub number: String,
    pub easting: f64,
    pub northing: f64,
    pub elevation: f64,
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPointsRequest {
    pub rows: Vec<PointRow>,
    #[serde(default = "default_label_height")]
    pub label_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPointsResult {
    pub inserted: usize,
    pub entities: usize,
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

/// Portable host surface used by the current SurveyCraft commands.
/// Implementations must make transactions undoable.
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
    fn commit_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError>;
    fn rollback_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError>;
}

const MAX_INSERT_POINTS: usize = 2_000;
const MAX_COORDINATE: f64 = 1.0e15;
const MAX_POINT_NUMBER_BYTES: usize = 128;
const MAX_CODE_BYTES: usize = 256;

/// Validate and insert survey points, point numbers, elevations and optional codes in one undoable
/// CAD transaction.
pub fn insert_points(
    host: &mut impl CadcraftHost,
    request: &InsertPointsRequest,
) -> Result<InsertPointsResult, AdapterError> {
    validate_insert_request(request)
        .map_err(|error| AdapterError::new("validate points", error, None))?;
    let transaction = host
        .begin_transaction("Insert SurveyCraft points")
        .map_err(|error| AdapterError::new("begin point transaction", error, None))?;
    let mut entities = 0usize;
    for row in &request.rows {
        let position = point(row.easting, row.northing, row.elevation)
            .map_err(|error| rollback_error(host, transaction, "prepare point", error))?;
        create_point(
            host,
            transaction,
            &PointRequest {
                position,
                layer: "SC-POINTS".into(),
            },
        )?;
        entities += 1;

        let offset = request.label_height * 1.4;
        let labels = [
            (
                row.number.trim().to_owned(),
                offset,
                offset,
                "SC-POINT-NUMBERS",
            ),
            (
                format!("{:.3}", row.elevation),
                offset,
                -offset,
                "SC-POINT-ELEVATIONS",
            ),
        ];
        for (value, dx, dy, layer) in labels {
            let label_position = point(row.easting + dx, row.northing + dy, row.elevation)
                .map_err(|error| rollback_error(host, transaction, "prepare point label", error))?;
            create_text(
                host,
                transaction,
                &TextRequest {
                    position: label_position,
                    value,
                    height: request.label_height,
                    rotation_degrees: 0.0,
                    layer: layer.into(),
                },
            )?;
            entities += 1;
        }
        if !row.code.trim().is_empty() {
            let code_position = point(
                row.easting + offset,
                row.northing - offset * 2.4,
                row.elevation,
            )
            .map_err(|error| rollback_error(host, transaction, "prepare point code", error))?;
            create_text(
                host,
                transaction,
                &TextRequest {
                    position: code_position,
                    value: row.code.trim().to_owned(),
                    height: request.label_height,
                    rotation_degrees: 0.0,
                    layer: "SC-POINT-CODES".into(),
                },
            )?;
            entities += 1;
        }
    }
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
    })
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
        point(row.easting, row.northing, row.elevation).map_err(|_| {
            HostError::new(format!(
                "row {} has coordinates outside the supported drawing range",
                index + 1
            ))
        })?;
    }
    Ok(())
}

fn point(easting: f64, northing: f64, elevation: f64) -> Result<Point3, HostError> {
    let values = [easting, northing, elevation];
    if values
        .iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE)
    {
        return Err(HostError::new(
            "coordinates must be finite and within the supported drawing range",
        ));
    }
    Point3::new(easting, northing, elevation).map_err(|error| HostError::new(error.to_string()))
}

fn create_point(
    host: &mut impl CadcraftHost,
    transaction: TransactionId,
    request: &PointRequest,
) -> Result<EntityId, AdapterError> {
    host.create_point(transaction, request)
        .map_err(|error| rollback_error(host, transaction, "create point", error))
}

fn create_text(
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

/// Proves one entity can cross the proposed host boundary in a guarded transaction.
pub fn run_reference_extension(
    host: &mut impl CadcraftHost,
    request: &LineRequest,
) -> Result<EntityId, AdapterError> {
    let transaction = host
        .begin_transaction("SurveyCraft reference line")
        .map_err(|error| AdapterError::new("begin transaction", error, None))?;

    let entity = match host.create_line(transaction, request) {
        Ok(entity) => entity,
        Err(error) => {
            let rollback_error = host.rollback_transaction(transaction).err();
            return Err(AdapterError::new("create line", error, rollback_error));
        }
    };

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
    use super::{
        CadcraftHost, EntityId, HostError, LineRequest, TransactionId, run_reference_extension,
    };
    use std::error::Error;
    use surveycraft_geometry::Point3;

    #[derive(Default)]
    struct MockHost {
        events: Vec<&'static str>,
        fail_create: bool,
    }

    impl CadcraftHost for MockHost {
        fn begin_transaction(&mut self, _label: &str) -> Result<TransactionId, HostError> {
            self.events.push("begin");
            Ok(TransactionId(1))
        }

        fn create_line(
            &mut self,
            _transaction: TransactionId,
            _request: &LineRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(EntityId(42))
            }
        }

        fn create_point(
            &mut self,
            _transaction: TransactionId,
            _request: &super::PointRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-point");
            if self.fail_create {
                Err(HostError::new("synthetic create failure"))
            } else {
                Ok(EntityId(43))
            }
        }

        fn create_text(
            &mut self,
            _transaction: TransactionId,
            _request: &super::TextRequest,
        ) -> Result<EntityId, HostError> {
            self.events.push("create-text");
            Ok(EntityId(44))
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

    fn request() -> Result<LineRequest, Box<dyn Error>> {
        Ok(LineRequest {
            start: Point3::new(0.0, 0.0, 0.0)?,
            end: Point3::new(10.0, 5.0, 0.0)?,
            layer: "SURVEY_REFERENCE".to_owned(),
        })
    }

    #[test]
    fn commits_a_successful_entity_creation() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let entity = run_reference_extension(&mut host, &request()?)?;
        assert_eq!(entity, EntityId(42));
        assert_eq!(host.events, ["begin", "create", "commit"]);
        Ok(())
    }

    #[test]
    fn rolls_back_a_failed_entity_creation() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost {
            fail_create: true,
            ..MockHost::default()
        };
        let result = run_reference_extension(&mut host, &request()?);
        assert!(result.is_err());
        assert_eq!(host.events, ["begin", "create", "rollback"]);
        Ok(())
    }

    #[test]
    fn inserts_point_geometry_and_labels_in_one_transaction() -> Result<(), Box<dyn Error>> {
        let mut host = MockHost::default();
        let request = super::InsertPointsRequest {
            rows: vec![super::PointRow {
                number: "101".into(),
                easting: 500.0,
                northing: 200.0,
                elevation: 12.5,
                code: "STN".into(),
            }],
            label_height: 0.25,
        };
        let result = super::insert_points(&mut host, &request)?;
        assert_eq!(
            result,
            super::InsertPointsResult {
                inserted: 1,
                entities: 4
            }
        );
        assert_eq!(
            host.events,
            [
                "begin",
                "create-point",
                "create-text",
                "create-text",
                "create-text",
                "commit"
            ]
        );
        Ok(())
    }

    #[test]
    fn rejects_duplicate_point_numbers_before_opening_a_transaction() {
        let mut host = MockHost::default();
        let request = super::InsertPointsRequest {
            rows: vec![
                super::PointRow {
                    number: "P-1".into(),
                    easting: 500.0,
                    northing: 200.0,
                    elevation: 12.5,
                    code: String::new(),
                },
                super::PointRow {
                    number: "p-1".into(),
                    easting: 501.0,
                    northing: 201.0,
                    elevation: 12.6,
                    code: String::new(),
                },
            ],
            label_height: 0.25,
        };

        let result = super::insert_points(&mut host, &request);
        assert!(result.is_err());
        assert!(host.events.is_empty());
    }
}
