//! Kernel-neutral B-Rep abstractions, topology references, and mass properties.
//!
//! OCCT types must not leak outside `opencad-kernel-occt`.

pub mod brep;
pub mod kernel;
pub mod mass;
pub mod nurbs;
pub mod refs;
pub mod stl;
pub mod tessellation;
pub mod topo_sync;
pub mod topology;

pub use kernel::{
    BooleanOp, ExtrudeExtent, ExtrudeOperation, FilletEdgeSelector, GeometryKernel, KernelBody,
    KernelWire, MockGeometryKernel, SolvedSketch,
};
pub use mass::{BoundingBox, MassProperties};
pub use nurbs::NurbsSurface;
pub use refs::{GeometricFingerprint, TopoRef, TopoRefKind, TopoRefSemantic};
pub use topo_sync::{
    assign_face_ref_to_refs, build_src_to_post_map, compose_face_derivation_histories,
    rebind_kernel_face_ids, resolve_kernel_face_id_for_topo_ref, resolve_topo_ref_id,
    resolve_topo_ref_id_with_history, sync_semantic_refs, sync_semantic_refs_with_history,
    FaceDerivation, FaceRefDiscovery, kernel_topo_ref_id,
};
pub use tessellation::{MeshSet, TessellationSettings};
pub use stl::write_binary_stl;
