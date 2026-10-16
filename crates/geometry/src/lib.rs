//! Small, deterministic geometry primitives shared by SurveyCraft engines.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Error returned when a coordinate is not a usable finite number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinateError {
    axis: &'static str,
}

impl CoordinateError {
    #[must_use]
    pub const fn axis(self) -> &'static str {
        self.axis
    }
}

impl Display for CoordinateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} coordinate must be finite", self.axis)
    }
}

impl Error for CoordinateError {}

/// A validated two-dimensional point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    x: f64,
    y: f64,
}

impl Point2 {
    pub fn new(x: f64, y: f64) -> Result<Self, CoordinateError> {
        ensure_finite("x", x)?;
        ensure_finite("y", y)?;
        Ok(Self { x, y })
    }

    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }

    #[must_use]
    pub fn distance_to(self, other: Self) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// A validated three-dimensional point in the project's linear unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Point3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, CoordinateError> {
        ensure_finite("x", x)?;
        ensure_finite("y", y)?;
        ensure_finite("z", z)?;
        Ok(Self { x, y, z })
    }

    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }

    #[must_use]
    pub const fn z(self) -> f64 {
        self.z
    }

    #[must_use]
    pub const fn xy(self) -> Point2 {
        Point2 {
            x: self.x,
            y: self.y,
        }
    }

    #[must_use]
    pub fn distance_3d_to(self, other: Self) -> f64 {
        let horizontal = self.xy().distance_to(other.xy());
        horizontal.hypot(other.z - self.z)
    }
}

fn ensure_finite(axis: &'static str, value: f64) -> Result<(), CoordinateError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CoordinateError { axis })
    }
}

#[cfg(test)]
mod tests {
    use super::{Point2, Point3};
    use std::error::Error;

    #[test]
    fn computes_distances() -> Result<(), Box<dyn Error>> {
        let first = Point2::new(0.0, 0.0)?;
        let second = Point2::new(3.0, 4.0)?;
        assert!((first.distance_to(second) - 5.0).abs() < f64::EPSILON);

        let low = Point3::new(0.0, 0.0, 0.0)?;
        let high = Point3::new(0.0, 4.0, 3.0)?;
        assert!((low.distance_3d_to(high) - 5.0).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn rejects_non_finite_coordinates() {
        let error = Point3::new(f64::NAN, 0.0, 0.0);
        assert!(error.is_err());
    }
}
