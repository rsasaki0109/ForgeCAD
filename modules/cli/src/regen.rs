//! `opencad regen` command (Task-123+).

use opencad_feature::{FeatureRegistry, PartModel, RegenReport};
use opencad_file::{read_ocad, write_expanded_dir, OcadDocument};
use opencad_geometry::GeometryKernel;
use opencad_graph::{FeatureGraph, ParamGraph};
use opencad_sketch::Sketch;
use serde::{Deserialize, Serialize};

#[cfg(feature = "occt")]
use opencad_kernel_occt::OcctGeometryKernel;

use opencad_core::Result;

const DEFAULT_DENSITY_KG_PER_M3: f64 = 2700.0;

/// Summary printed by `opencad regen`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegenSummary {
    pub kernel: String,
    pub report: RegenReport,
    pub volume_m3: Option<f64>,
    pub mass_kg: Option<f64>,
}

/// Serializable regeneration result for Agent API responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegenResult {
    pub kernel: String,
    pub regenerated: Vec<String>,
    pub skipped_suppressed: Vec<String>,
    pub volume_m3: Option<f64>,
    pub mass_kg: Option<f64>,
    pub density_kg_per_m3: f64,
}

/// Design body required for in-memory regeneration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegenBodyParams {
    pub parameters: ParamGraph,
    pub sketches: Vec<Sketch>,
    pub feature_graph: FeatureGraph,
    pub feature_nodes: Vec<opencad_feature::FeatureNode>,
}

impl From<RegenSummary> for RegenResult {
    fn from(summary: RegenSummary) -> Self {
        Self {
            kernel: summary.kernel,
            regenerated: summary.report.regenerated,
            skipped_suppressed: summary.report.skipped_suppressed,
            volume_m3: summary.volume_m3,
            mass_kg: summary.mass_kg,
            density_kg_per_m3: DEFAULT_DENSITY_KG_PER_M3,
        }
    }
}

pub fn regen_document(path: &str, sync_topo_refs: bool) -> Result<RegenSummary> {
    if sync_topo_refs {
        let mut doc = read_ocad(path)?;
        let summary = regen_ocad_document(&doc)?;
        crate::topo_sync::sync_document_topo_refs(&mut doc)?;
        write_expanded_dir(path, &doc)?;
        return Ok(summary);
    }
    let doc = read_ocad(path)?;
    regen_ocad_document(&doc)
}

pub fn regen_ocad_document(doc: &OcadDocument) -> Result<RegenSummary> {
    let parameters = doc.parameters.clone();
    let mut model = doc.clone().into_part_model();
    regenerate_part(&mut model, Some(&parameters), Some(&doc.semantic_refs))
}

pub fn regen_body(body: &RegenBodyParams) -> Result<RegenSummary> {
    let mut model = PartModel::new();
    model.graph = body.feature_graph.clone();
    for sketch in &body.sketches {
        model
            .sketches
            .insert(sketch.id.as_str().to_string(), sketch.clone());
    }
    for node in &body.feature_nodes {
        model.nodes.insert(node.id.clone(), node.clone());
    }
    regenerate_part(&mut model, Some(&body.parameters), None)
}

pub fn regenerate_part(
    model: &mut PartModel,
    parameters: Option<&ParamGraph>,
    semantic_refs: Option<&[opencad_geometry::TopoRef]>,
) -> Result<RegenSummary> {
    let registry = FeatureRegistry::with_defaults();

    #[cfg(feature = "occt")]
    {
        let kernel = OcctGeometryKernel::new();
        let kernel_name = OcctGeometryKernel::occt_version().to_string();
        let report = model.regenerate(&kernel, &registry, parameters, semantic_refs)?;
        let (volume_m3, mass_kg) = mass_for_active_body(model, &kernel);
        Ok(RegenSummary {
            kernel: kernel_name,
            report,
            volume_m3,
            mass_kg,
        })
    }

    #[cfg(not(feature = "occt"))]
    {
        let kernel = opencad_geometry::MockGeometryKernel::new();
        let report = model.regenerate(&kernel, &registry, parameters, semantic_refs)?;
        let (volume_m3, mass_kg) = mass_for_active_body(model, &kernel);
        Ok(RegenSummary {
            kernel: "MockGeometryKernel".into(),
            report,
            volume_m3,
            mass_kg,
        })
    }
}

fn mass_for_active_body<K: GeometryKernel>(
    model: &PartModel,
    kernel: &K,
) -> (Option<f64>, Option<f64>) {
    let Some(body) = model.active_body() else {
        return (None, None);
    };
    let Ok(mass) = kernel.mass_properties(body, DEFAULT_DENSITY_KG_PER_M3) else {
        return (None, None);
    };
    (Some(mass.volume_m3), Some(mass.mass_kg))
}

pub fn print_summary(summary: &RegenSummary) {
    println!("kernel: {}", summary.kernel);
    println!("regenerated: {} features", summary.report.regenerated.len());
    for id in &summary.report.regenerated {
        println!("  {id}");
    }
    if !summary.report.skipped_suppressed.is_empty() {
        println!("suppressed: {}", summary.report.skipped_suppressed.len());
        for id in &summary.report.skipped_suppressed {
            println!("  {id}");
        }
    }
    if let Some(volume) = summary.volume_m3 {
        println!("volume_m3: {volume}");
    }
    if let Some(mass) = summary.mass_kg {
        println!("mass_kg: {mass} (density {DEFAULT_DENSITY_KG_PER_M3} kg/m^3)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_core::{DocumentId, DocumentMetadata};
    use opencad_feature::bracket_with_hole;
    use opencad_file::{write_expanded_dir, OcadDocument};
    use opencad_graph::bracket_parameters;
    use tempfile::tempdir;

    #[test]
    fn regen_bracket_fixture() {
        let part = bracket_with_hole().expect("model");
        let metadata = DocumentMetadata::new(
            DocumentId::new("doc:bracket_001").expect("id"),
            "Bracket with Mounting Hole",
        );
        let mut doc = OcadDocument::from_part_model(metadata, &part);
        doc.parameters = bracket_parameters();
        let dir = tempdir().expect("tempdir");
        write_expanded_dir(dir.path(), &doc).expect("write");

        let summary = regen_document(dir.path().to_str().expect("path"), false).expect("regen");
        assert_eq!(summary.report.regenerated.len(), 4);
        assert!(summary.volume_m3.unwrap_or(0.0) > 0.0);
    }
}
