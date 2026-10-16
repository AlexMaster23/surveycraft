//! CAD-independent SurveyCraft domain model.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use surveycraft_geometry::Point3;
use uuid::Uuid;

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
const MAX_PROJECT_NAME_BYTES: usize = 256;
const MAX_POINT_NUMBER_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidProjectName,
    InvalidPointNumber,
    InvalidTolerance(&'static str),
    InvalidDisplayDecimals,
    UnsupportedSchemaVersion(u32),
    DuplicatePointId(Uuid),
}

impl Display for CoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProjectName => write!(formatter, "project name is empty or invalid"),
            Self::InvalidPointNumber => write!(formatter, "point number is empty or invalid"),
            Self::InvalidTolerance(field) => {
                write!(
                    formatter,
                    "numeric policy field '{field}' must be finite and positive"
                )
            }
            Self::InvalidDisplayDecimals => {
                write!(formatter, "display decimals must be between 0 and 12")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported project schema version {version}")
            }
            Self::DuplicatePointId(id) => write!(formatter, "duplicate point id {id}"),
        }
    }
}

impl Error for CoreError {}

/// Unit used by all coordinates and tolerances in one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinearUnit {
    Metre,
    Millimetre,
    InternationalFoot,
    UsSurveyFoot,
}

impl LinearUnit {
    #[must_use]
    pub const fn metres_per_unit(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Millimetre => 0.001,
            Self::InternationalFoot => 0.3048,
            Self::UsSurveyFoot => 1200.0 / 3937.0,
        }
    }
}

/// Coordinate-reference metadata. No implicit coordinate transformation occurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crs {
    pub authority: Option<String>,
    pub code: Option<String>,
    pub name: String,
}

impl Crs {
    #[must_use]
    pub fn local_engineering() -> Self {
        Self {
            authority: None,
            code: None,
            name: "Local engineering grid".to_owned(),
        }
    }
}

/// Explicit thresholds used instead of hidden floating-point guesses.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumericPolicy {
    pub coordinate_tolerance: f64,
    pub distance_tolerance: f64,
    pub area_tolerance: f64,
    pub display_decimals: u8,
}

impl Default for NumericPolicy {
    fn default() -> Self {
        Self {
            coordinate_tolerance: 1.0e-8,
            distance_tolerance: 1.0e-6,
            area_tolerance: 1.0e-6,
            display_decimals: 3,
        }
    }
}

impl NumericPolicy {
    pub fn validate(self) -> Result<(), CoreError> {
        validate_positive("coordinateTolerance", self.coordinate_tolerance)?;
        validate_positive("distanceTolerance", self.distance_tolerance)?;
        validate_positive("areaTolerance", self.area_tolerance)?;
        if self.display_decimals > 12 {
            return Err(CoreError::InvalidDisplayDecimals);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualityFlag {
    DuplicateNumber,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub source: String,
    pub row: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurveyPoint {
    pub id: Uuid,
    pub number: String,
    pub position: Point3,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: SourceRef,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub quality_flags: BTreeSet<QualityFlag>,
}

impl SurveyPoint {
    pub fn new(
        id: Uuid,
        number: impl Into<String>,
        position: Point3,
        code: Option<String>,
        description: Option<String>,
        source: SourceRef,
    ) -> Result<Self, CoreError> {
        let number = number.into().trim().to_owned();
        validate_text(&number, MAX_POINT_NUMBER_BYTES)
            .map_err(|_| CoreError::InvalidPointNumber)?;
        Ok(Self {
            id,
            number,
            position,
            code: clean_optional_text(code),
            description: clean_optional_text(description),
            source,
            quality_flags: BTreeSet::new(),
        })
    }

    pub fn add_quality_flag(&mut self, flag: QualityFlag) {
        self.quality_flags.insert(flag);
    }
}

/// Stable point identity for repeatable imports of the same project and row.
#[must_use]
pub fn deterministic_point_id(project_id: Uuid, row: usize, number: &str) -> Uuid {
    let name = format!("point:{row}:{}", number.trim());
    Uuid::new_v5(&project_id, name.as_bytes())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurveyProject {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub crs: Crs,
    pub linear_unit: LinearUnit,
    pub numeric_policy: NumericPolicy,
    pub points: Vec<SurveyPoint>,
}

impl SurveyProject {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        crs: Crs,
        linear_unit: LinearUnit,
        numeric_policy: NumericPolicy,
    ) -> Result<Self, CoreError> {
        let name = name.into().trim().to_owned();
        validate_text(&name, MAX_PROJECT_NAME_BYTES).map_err(|_| CoreError::InvalidProjectName)?;
        numeric_policy.validate()?;
        Ok(Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            id,
            name,
            crs,
            linear_unit,
            numeric_policy,
            points: Vec::new(),
        })
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(self.schema_version));
        }
        validate_text(&self.name, MAX_PROJECT_NAME_BYTES)
            .map_err(|_| CoreError::InvalidProjectName)?;
        self.numeric_policy.validate()?;
        let mut ids = HashSet::with_capacity(self.points.len());
        for point in &self.points {
            validate_text(&point.number, MAX_POINT_NUMBER_BYTES)
                .map_err(|_| CoreError::InvalidPointNumber)?;
            if !ids.insert(point.id) {
                return Err(CoreError::DuplicatePointId(point.id));
            }
        }
        Ok(())
    }

    pub fn add_point(&mut self, point: SurveyPoint) {
        self.points.push(point);
    }
}

fn validate_positive(field: &'static str, value: f64) -> Result<(), CoreError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(CoreError::InvalidTolerance(field))
    }
}

fn validate_text(value: &str, max_bytes: usize) -> Result<(), ()> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(())
    } else {
        Ok(())
    }
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Crs, LinearUnit, NumericPolicy, SourceRef, SurveyPoint, SurveyProject,
        deterministic_point_id,
    };
    use std::error::Error;
    use surveycraft_geometry::Point3;
    use uuid::Uuid;

    #[test]
    fn point_ids_are_stable() {
        let project_id = Uuid::nil();
        assert_eq!(
            deterministic_point_id(project_id, 2, "101"),
            deterministic_point_id(project_id, 2, "101")
        );
        assert_ne!(
            deterministic_point_id(project_id, 2, "101"),
            deterministic_point_id(project_id, 3, "101")
        );
    }

    #[test]
    fn validates_a_project() -> Result<(), Box<dyn Error>> {
        let project_id = Uuid::nil();
        let mut project = SurveyProject::new(
            project_id,
            "Synthetic survey",
            Crs::local_engineering(),
            LinearUnit::Metre,
            NumericPolicy::default(),
        )?;
        project.add_point(SurveyPoint::new(
            deterministic_point_id(project_id, 2, "101"),
            "101",
            Point3::new(500.0, 200.0, 12.5)?,
            Some("STN".to_owned()),
            None,
            SourceRef {
                source: "synthetic.csv".to_owned(),
                row: 2,
            },
        )?);
        project.validate()?;
        Ok(())
    }
}
