// Shared golden fixture helpers for integration tests.

use opencad_ai::DesignPatch;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GoldenFixture {
    pub density_kg_per_m3: f64,
    pub cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize)]
pub struct GoldenCase {
    pub id: String,
    pub volume_m3: f64,
    pub mass_kg: f64,
    pub volume_tol: f64,
    pub mass_tol: f64,
    pub patch: Option<DesignPatch>,
    pub min_volume_delta_m3: Option<f64>,
}

pub fn load_fixture(json: &str) -> GoldenFixture {
    serde_json::from_str(json).expect("golden fixture JSON")
}

pub fn assert_near(actual: f64, expected: f64, tol: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{label}: actual={actual} expected={expected} tol={tol}"
    );
}
