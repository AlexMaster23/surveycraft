//! Toolkit-independent state exchanged between SurveyCraft commands and UI hosts.

use serde::{Deserialize, Serialize};

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
    use super::TaskProgress;

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
}
