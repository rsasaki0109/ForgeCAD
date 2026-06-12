// Shared golden fixture helpers for feature integration tests.

use opencad_ai::DesignPatch;
use opencad_feature::{FeatureRegistry, PartModel};
use opencad_geometry::GeometryKernel;
use opencad_graph::ParamGraph;
use opencad_kernel_occt::OcctGeometryKernel;
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

pub fn regen_volume_mass(
    kernel: &OcctGeometryKernel,
    model: &mut PartModel,
    params: &ParamGraph,
    density_kg_per_m3: f64,
) -> (f64, f64) {
    let registry = FeatureRegistry::with_defaults();
    model
        .regenerate(kernel, &registry, Some(params), None)
        .expect("regen");
    let body = model.active_body().expect("body");
    let mass = kernel
        .mass_properties(body, density_kg_per_m3)
        .expect("mass");
    (mass.volume_m3, mass.mass_kg)
}

#[allow(dead_code)]
pub fn apply_patch(
    model: &mut PartModel,
    params: &mut ParamGraph,
    patch: &DesignPatch,
) {
    patch.apply_to_parameters(params).expect("param patch");
    let mut nodes: Vec<opencad_feature::FeatureNode> =
        model.nodes.clone().into_values().collect();
    patch.apply_to_features(&mut nodes).expect("feature patch");
    for node in nodes {
        model.nodes.insert(node.id.clone(), node);
    }
}
