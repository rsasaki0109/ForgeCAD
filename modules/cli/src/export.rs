//! `opencad export` command (Task-125+).

use std::path::Path;

use opencad_core::{OpenCadError, Result};
use opencad_feature::{FeatureRegistry, PartModel};
use opencad_file::read_ocad;
use opencad_geometry::{write_binary_stl, FaceDerivation, GeometryKernel, MeshSet, TessellationSettings, TopoRef};
use serde::{Deserialize, Serialize};

#[cfg(feature = "occt")]
use opencad_kernel_occt::OcctGeometryKernel;

/// Summary printed by `opencad export`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportSummary {
    pub format: String,
    pub triangles: usize,
    pub output: String,
}

pub fn export_stl(input: &str, output: &str) -> Result<ExportSummary> {
    let doc = read_ocad(input)?;
    let name = doc.metadata.name.clone();
    let parameters = doc.parameters.clone();
    let semantic_refs = doc.semantic_refs.clone();
    let mut model = doc.into_part_model();
    let mesh = tessellate_active_body(&mut model, Some(&parameters), Some(&semantic_refs))?;
    let output_path = Path::new(output);
    if output_path.extension().and_then(|s| s.to_str()) != Some("stl") {
        return Err(OpenCadError::validation("export output must use .stl extension"));
    }
    write_binary_stl(output_path, &mesh, &name)?;
    Ok(ExportSummary {
        format: "stl".into(),
        triangles: mesh.triangle_count(),
        output: output.to_string(),
    })
}

/// Tessellated active body with kernel face derivation history.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TessellatedBody {
    pub mesh_set: MeshSet,
    pub face_history: Vec<FaceDerivation>,
}

pub(crate) fn tessellate_active_body(
    model: &mut PartModel,
    parameters: Option<&opencad_graph::ParamGraph>,
    semantic_refs: Option<&[TopoRef]>,
) -> Result<MeshSet> {
    Ok(tessellate_active_body_detailed(model, parameters, semantic_refs)?.mesh_set)
}

pub(crate) fn tessellate_active_body_detailed(
    model: &mut PartModel,
    parameters: Option<&opencad_graph::ParamGraph>,
    semantic_refs: Option<&[TopoRef]>,
) -> Result<TessellatedBody> {
    let registry = FeatureRegistry::with_defaults();

    #[cfg(feature = "occt")]
    {
        let kernel = OcctGeometryKernel::new();
        let report = model.regenerate(&kernel, &registry, parameters, semantic_refs)?;
        let body = model
            .active_body()
            .ok_or_else(|| OpenCadError::validation("document has no solid body to export"))?;
        let mesh_set = kernel.tessellate(body, &TessellationSettings::default())?;
        let face_history = if report.face_history.is_empty() {
            kernel.face_derivation_history(body)
        } else {
            report.face_history
        };
        Ok(TessellatedBody {
            mesh_set,
            face_history,
        })
    }

    #[cfg(not(feature = "occt"))]
    {
        let kernel = opencad_geometry::MockGeometryKernel::new();
        let report = model.regenerate(&kernel, &registry, parameters, semantic_refs)?;
        let body = model
            .active_body()
            .ok_or_else(|| OpenCadError::validation("document has no solid body to export"))?;
        let mesh_set = kernel.tessellate(body, &TessellationSettings::default())?;
        let face_history = if report.face_history.is_empty() {
            kernel.face_derivation_history(body)
        } else {
            report.face_history
        };
        Ok(TessellatedBody {
            mesh_set,
            face_history,
        })
    }
}

pub fn print_summary(summary: &ExportSummary) {
    println!("exported: {}", summary.output);
    println!("format: {}", summary.format);
    println!("triangles: {}", summary.triangles);
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_core::{DocumentId, DocumentMetadata};
    use opencad_feature::bracket_base_plate;
    use opencad_file::{write_expanded_dir, OcadDocument};
    use opencad_graph::bracket_parameters;
    use tempfile::tempdir;

    #[test]
    fn exports_bracket_to_stl() {
        let part = bracket_base_plate().expect("model");
        let metadata = DocumentMetadata::new(
            DocumentId::new("doc:bracket_001").expect("id"),
            "Bracket Base Plate",
        );
        let mut doc = OcadDocument::from_part_model(metadata, &part);
        doc.parameters = bracket_parameters();
        let dir = tempdir().expect("tempdir");
        write_expanded_dir(dir.path(), &doc).expect("write");
        let output = dir.path().join("bracket.stl");
        let summary = export_stl(
            dir.path().to_str().expect("path"),
            output.to_str().expect("stl"),
        )
        .expect("export");
        assert!(summary.triangles > 0);
        assert!(output.is_file());
    }
}
