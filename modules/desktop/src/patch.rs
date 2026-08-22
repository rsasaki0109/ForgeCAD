//! Shared patch-and-regenerate orchestration for desktop and CLI surfaces.

use opencad_ai::DesignPatch;
use opencad_core::{DocumentKind, OpenCadError, Result};
use opencad_feature::{FeatureRegistry, RegenReport};
use opencad_file::{apply_patch_to_document, OcadDocument};
use opencad_geometry::GeometryKernel;

/// Apply a patch and validate part regeneration atomically.
///
/// File serialization remains in `opencad-file`; this command-layer helper
/// owns the cross-module operation that combines validated patch application
/// with feature execution. Both patching and regeneration run against a
/// disposable candidate, and the source document is replaced only after both
/// succeed. B-Rep and mesh outputs remain disposable.
pub fn apply_patch_and_regenerate<K: GeometryKernel>(
    doc: &mut OcadDocument,
    patch: &DesignPatch,
    kernel: &K,
    registry: &FeatureRegistry,
) -> Result<RegenReport> {
    if doc.metadata.kind != DocumentKind::Part {
        return Err(OpenCadError::validation(
            "atomic patch regeneration accepts only part documents",
        ));
    }

    let mut candidate = doc.clone();
    apply_patch_to_document(&mut candidate, patch)?;

    let parameters = candidate.parameters.clone();
    let semantic_refs = candidate.semantic_refs.clone();
    let mut model = candidate.clone().into_part_model();
    let report = model.regenerate(kernel, registry, Some(&parameters), Some(&semantic_refs))?;

    *doc = candidate;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencad_ai::{FeatureExprField, PatchOperation};
    use opencad_feature::{bracket_base_plate, FeatureDefinition};
    use opencad_geometry::MockGeometryKernel;
    use opencad_graph::bracket_parameters;

    fn document(name: &str) -> OcadDocument {
        let part = bracket_base_plate().expect("model");
        let metadata = opencad_core::DocumentMetadata::new(
            opencad_core::DocumentId::new(format!("doc:{name}")).expect("id"),
            name,
        );
        let mut doc = OcadDocument::from_part_model(metadata, &part);
        doc.parameters = bracket_parameters();
        doc
    }

    #[test]
    fn atomic_patch_regeneration_commits_all_operations() {
        let mut doc = document("atomic_success");
        let patch = DesignPatch::new(vec![
            PatchOperation::SetParameter {
                id: "param:thickness".into(),
                expr: "8 mm".into(),
            },
            PatchOperation::SetFeatureExpr {
                feature_id: "feature:extrude_base".into(),
                field: FeatureExprField::LengthExpr.as_str().into(),
                expr: "thickness * 2".into(),
            },
        ]);
        let kernel = MockGeometryKernel::new();
        let registry = FeatureRegistry::with_defaults();

        let report = apply_patch_and_regenerate(&mut doc, &patch, &kernel, &registry)
            .expect("atomic patch and regeneration");
        assert!(report
            .regenerated
            .iter()
            .any(|id| id == "feature:extrude_base"));
        assert_eq!(doc.parameters.get("param:thickness").unwrap().expr, "8 mm");
        let node = doc
            .feature_nodes
            .iter()
            .find(|node| node.id == "feature:extrude_base")
            .expect("extrude");
        let FeatureDefinition::Extrude(extrude) = &node.definition else {
            panic!("expected extrude")
        };
        assert_eq!(extrude.length_expr.as_deref(), Some("thickness * 2"));
    }

    #[test]
    fn failed_atomic_regeneration_preserves_serialized_document() {
        let mut doc = document("atomic_failure");
        let before = opencad_file::expanded_dir::serialize_document_files(&doc).expect("serialize");
        let patch = DesignPatch::new(vec![
            PatchOperation::SetParameter {
                id: "param:thickness".into(),
                expr: "8 mm".into(),
            },
            PatchOperation::SetFeatureExpr {
                feature_id: "feature:extrude_base".into(),
                field: FeatureExprField::LengthExpr.as_str().into(),
                expr: "not_a_length".into(),
            },
        ]);
        let kernel = MockGeometryKernel::new();
        let registry = FeatureRegistry::with_defaults();

        apply_patch_and_regenerate(&mut doc, &patch, &kernel, &registry)
            .expect_err("invalid feature expression");
        let after = opencad_file::expanded_dir::serialize_document_files(&doc).expect("serialize");
        assert_eq!(before, after);
        assert_eq!(doc.parameters.get("param:thickness").unwrap().expr, "6 mm");
    }

    #[test]
    fn assembly_documents_are_rejected_before_patch_mutation() {
        let mut doc = document("part");
        doc.metadata.kind = DocumentKind::Assembly;
        let before = doc.clone();
        let error = apply_patch_and_regenerate(
            &mut doc,
            &DesignPatch::set_parameter("param:width", "100 mm"),
            &MockGeometryKernel::new(),
            &FeatureRegistry::with_defaults(),
        )
        .expect_err("part-only API");
        assert!(error.to_string().contains("only part documents"));
        assert_eq!(doc, before);
    }
}
