//! B-Rep separates geometry curves/surfaces from topology connectivity.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Surface {
    Plane { origin: [f64; 3], normal: [f64; 3] },
    Cylinder { axis: [f64; 3], radius: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Curve {
    Line {
        start: [f64; 3],
        end: [f64; 3],
    },
    Circle {
        center: [f64; 3],
        radius: f64,
        normal: [f64; 3],
    },
}
