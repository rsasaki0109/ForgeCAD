use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{OpenCadError, Result};

/// Length stored internally in SI meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Length {
    pub value_si: f64,
}

/// Angle stored internally in SI radians.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Angle {
    pub value_si: f64,
}

/// Mass stored internally in SI kilograms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mass {
    pub value_si: f64,
}

/// Density stored internally in SI kg/m³.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Density {
    pub value_si: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LengthUnit {
    #[serde(rename = "m")]
    Meter,
    #[serde(rename = "mm")]
    Millimeter,
    #[serde(rename = "cm")]
    Centimeter,
    #[serde(rename = "in")]
    Inch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MassUnit {
    #[serde(rename = "kg")]
    Kilogram,
    #[serde(rename = "g")]
    Gram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityUnit {
    #[serde(rename = "kg/m^3")]
    KgPerCubicMeter,
    #[serde(rename = "g/cm^3")]
    GPerCubicCentimeter,
}

impl Length {
    pub const METER: Self = Self { value_si: 1.0 };
    pub const MILLIMETER: Self = Self { value_si: 0.001 };

    pub fn from_meters(value: f64) -> Self {
        Self { value_si: value }
    }

    pub fn from_unit(value: f64, unit: LengthUnit) -> Self {
        Self {
            value_si: value * unit.to_meters_factor(),
        }
    }

    pub fn as_unit(&self, unit: LengthUnit) -> f64 {
        self.value_si / unit.to_meters_factor()
    }

    pub fn meters(&self) -> f64 {
        self.value_si
    }

    pub fn millimeters(&self) -> f64 {
        self.as_unit(LengthUnit::Millimeter)
    }
}

impl Angle {
    pub fn from_radians(value: f64) -> Self {
        Self { value_si: value }
    }

    pub fn from_degrees(value: f64) -> Self {
        Self {
            value_si: value.to_radians(),
        }
    }

    pub fn radians(&self) -> f64 {
        self.value_si
    }

    pub fn degrees(&self) -> f64 {
        self.value_si.to_degrees()
    }
}

impl Mass {
    pub fn from_kilograms(value: f64) -> Self {
        Self { value_si: value }
    }

    pub fn from_unit(value: f64, unit: MassUnit) -> Self {
        Self {
            value_si: value * unit.to_kg_factor(),
        }
    }

    pub fn kilograms(&self) -> f64 {
        self.value_si
    }

    pub fn grams(&self) -> f64 {
        self.as_unit(MassUnit::Gram)
    }

    pub fn as_unit(&self, unit: MassUnit) -> f64 {
        self.value_si / unit.to_kg_factor()
    }
}

impl Density {
    pub fn from_kg_per_cubic_meter(value: f64) -> Self {
        Self { value_si: value }
    }

    pub fn from_unit(value: f64, unit: DensityUnit) -> Self {
        Self {
            value_si: value * unit.to_si_factor(),
        }
    }

    pub fn kg_per_cubic_meter(&self) -> f64 {
        self.value_si
    }
}

impl LengthUnit {
    pub fn to_meters_factor(self) -> f64 {
        match self {
            Self::Meter => 1.0,
            Self::Millimeter => 0.001,
            Self::Centimeter => 0.01,
            Self::Inch => 0.0254,
        }
    }
}

impl MassUnit {
    pub fn to_kg_factor(self) -> f64 {
        match self {
            Self::Kilogram => 1.0,
            Self::Gram => 0.001,
        }
    }
}

impl DensityUnit {
    pub fn to_si_factor(self) -> f64 {
        match self {
            Self::KgPerCubicMeter => 1.0,
            Self::GPerCubicCentimeter => 1000.0,
        }
    }
}

/// Unit-aware expression string (e.g. `"80 mm"`, `"thickness + 3 mm"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Expression(String);

impl Expression {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(OpenCadError::InvalidExpression(
                "expression must not be empty".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Expression {
    fn from(value: &str) -> Self {
        Self::new(value).expect("static expression must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_unit_conversion() {
        let len = Length::from_unit(80.0, LengthUnit::Millimeter);
        assert!((len.meters() - 0.08).abs() < 1e-12);
        assert!((len.millimeters() - 80.0).abs() < 1e-9);
    }

    #[test]
    fn angle_conversion() {
        let angle = Angle::from_degrees(90.0);
        assert!((angle.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn mass_conversion() {
        let mass = Mass::from_unit(128.0, MassUnit::Gram);
        assert!((mass.kilograms() - 0.128).abs() < 1e-12);
    }

    #[test]
    fn expression_rejects_empty() {
        assert!(Expression::new("").is_err());
    }
}
