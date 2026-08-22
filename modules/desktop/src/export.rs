//! File-backed export operations shared by desktop and headless clients.

use std::path::Path;

use opencad_core::{DocumentKind, OpenCadError, Result};
use opencad_file::read_ocad;
use opencad_geometry::write_binary_stl;
use serde::{Deserialize, Serialize};

use crate::regen::tessellate_active_body;

/// Serializable result of an export operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportSummary {
    pub format: String,
    pub triangles: usize,
    pub output: String,
}

/// Export a part document through the same backend used by desktop preview.
///
/// The output is disposable geometry; the Design Graph is read but never
/// mutated.  Assembly and drawing exports retain their specialized CLI/Agent
/// paths until those workflows are folded into the backend transaction layer.
pub fn export_stl_document(input: &str, output: &str) -> Result<ExportSummary> {
    let output_path = Path::new(output);
    if output_path.extension().and_then(|ext| ext.to_str()) != Some("stl") {
        return Err(OpenCadError::validation(
            "desktop export output must use .stl extension",
        ));
    }

    let doc = read_ocad(input)?;
    if doc.metadata.kind != DocumentKind::Part {
        return Err(OpenCadError::validation(
            "desktop part export accepts only DocumentKind::Part documents",
        ));
    }

    let name = doc.metadata.name.clone();
    let parameters = doc.parameters.clone();
    let semantic_refs = doc.semantic_refs.clone();
    let mut model = doc.into_part_model();
    let mesh = tessellate_active_body(&mut model, Some(&parameters), Some(&semantic_refs))?;
    let triangles = mesh.triangle_count();
    write_binary_stl(output_path, &mesh, &name)?;

    Ok(ExportSummary {
        format: "stl".into(),
        triangles,
        output: output.to_string(),
    })
}
