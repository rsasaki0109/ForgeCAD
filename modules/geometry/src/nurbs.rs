use serde::{Deserialize, Serialize};

/// Serializable NURBS surface for `.ocad` storage (MVP: no editing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NurbsSurface {
    pub degree_u: u32,
    pub degree_v: u32,
    pub knots_u: Vec<f64>,
    pub knots_v: Vec<f64>,
    pub weights: Vec<Vec<f64>>,
    pub control_points: Vec<Vec<[f64; 3]>>,
    pub rational: bool,
}
