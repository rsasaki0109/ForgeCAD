//! File-backed regeneration for desktop and headless smoke clients.

use opencad_core::{DocumentKind, OpenCadError, Result};
use opencad_feature::FeatureRegistry;
use opencad_file::read_ocad;
use opencad_geometry::GeometryKernel;
use serde::{Deserialize, Serialize};

/// Serializable result of regenerating one document through the desktop backend.
///
/// The geometry kernel remains an implementation detail.  The desktop shell and
/// headless clients receive only stable identifiers and unit-free counts here;
/// all model dimensions remain in the Design Graph and are not copied into this
/// transport type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentRegeneration {
    pub kernel: String,
    pub regenerated: Vec<String>,
    pub skipped_suppressed: Vec<String>,
    pub triangles: usize,
}

/// Regenerate a file-backed part without persisting cached B-Rep or mesh data.
///
/// This is intentionally a read-only backend operation.  Parameter edits must
/// be persisted by an explicit command, after which this function can be used
/// to validate the resulting Design Graph.  Assembly regeneration continues to
/// use the assembly-specific backend path exposed by the CLI/Agent API.
pub fn regenerate_document(path: &str) -> Result<DocumentRegeneration> {
    let doc = read_ocad(path)?;
    if doc.metadata.kind != DocumentKind::Part {
        return Err(OpenCadError::validation(
            "desktop part regeneration accepts only DocumentKind::Part documents",
        ));
    }

    let parameters = doc.parameters.clone();
    let semantic_refs = doc.semantic_refs.clone();
    let mut model = doc.into_part_model();
    let registry = FeatureRegistry::with_defaults();

    #[cfg(feature = "occt")]
    {
        let kernel = opencad_kernel_occt::OcctGeometryKernel::new();
        let report =
            model.regenerate(&kernel, &registry, Some(&parameters), Some(&semantic_refs))?;
        let body = model
            .active_body()
            .ok_or_else(|| OpenCadError::validation("document has no solid body to regenerate"))?;
        let mesh = kernel.tessellate(body, &opencad_geometry::TessellationSettings::default())?;
        Ok(DocumentRegeneration {
            kernel: "occt".into(),
            regenerated: report.regenerated,
            skipped_suppressed: report.skipped_suppressed,
            triangles: mesh.triangle_count(),
        })
    }

    #[cfg(not(feature = "occt"))]
    {
        let _ = (model, parameters, semantic_refs, registry);
        Err(OpenCadError::Other(
            "OCCT backend disabled; rebuild with --features occt".into(),
        ))
    }
}
