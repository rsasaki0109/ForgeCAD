//! Resolve persisted TopoRefs during feature regeneration.

use opencad_core::{OpenCadError, Result};
use opencad_geometry::{resolve_kernel_face_id_for_topo_ref, FilletEdgeSelector};

use crate::feature::RegenContext;

pub fn edge_selector_for_face_ref(
    ctx: &dyn RegenContext,
    face_ref: &str,
    fallback: FilletEdgeSelector,
) -> Result<FilletEdgeSelector> {
    if face_ref.trim().is_empty() {
        return Ok(fallback);
    }

    let topo_ref = ctx
        .semantic_refs()
        .iter()
        .find(|topo_ref| topo_ref.ref_id.as_str() == face_ref)
        .ok_or_else(|| OpenCadError::not_found(format!("topo ref '{face_ref}'")))?;

    if let Ok(kernel_face_id) = resolve_kernel_face_id_for_topo_ref(
        ctx.semantic_refs(),
        ctx.face_history(),
        face_ref,
    ) {
        return Ok(FilletEdgeSelector::FacePerimeter { kernel_face_id });
    }

    match topo_ref.semantic.role.as_deref() {
        Some("top") => Ok(FilletEdgeSelector::TopPerimeter),
        _ => Ok(fallback),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::RegenContext;
    use crate::regenerate::TestRegenContext;
    use opencad_core::TopoRefId;
    use opencad_geometry::{KernelBody, TopoRef};

    struct RefContext {
        inner: TestRegenContext,
        semantic_refs: Vec<TopoRef>,
    }

    impl RegenContext for RefContext {
        fn kernel(&self) -> &dyn opencad_geometry::GeometryKernel {
            self.inner.kernel()
        }

        fn sketch_for_feature(&self, sketch_feature_id: &str) -> Result<&opencad_sketch::Sketch> {
            self.inner.sketch_for_feature(sketch_feature_id)
        }

        fn body_for_feature(&self, feature_id: &str) -> Result<KernelBody> {
            self.inner.body_for_feature(feature_id)
        }

        fn semantic_refs(&self) -> &[TopoRef] {
            &self.semantic_refs
        }
    }

    #[test]
    fn face_ref_without_kernel_id_falls_back_to_top_perimeter() {
        let ctx = RefContext {
            inner: TestRegenContext::with_body("feature:base", KernelBody::new(42)),
            semantic_refs: vec![TopoRef::face(
                TopoRefId::new("ref:face:bracket_top").expect("id"),
                "feature:extrude_base",
                "top",
            )],
        };
        let selector =
            edge_selector_for_face_ref(&ctx, "ref:face:bracket_top", FilletEdgeSelector::All)
                .expect("selector");
        assert_eq!(selector, FilletEdgeSelector::TopPerimeter);
    }
}
