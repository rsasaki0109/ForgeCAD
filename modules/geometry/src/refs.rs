use serde::{Deserialize, Serialize};

use opencad_core::{OpenCadError, Result, TopoRefId};

/// Semantic topological reference with optional geometric fingerprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopoRef {
    pub ref_id: TopoRefId,
    pub kind: TopoRefKind,
    pub semantic: TopoRefSemantic,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometric_fingerprint: Option<GeometricFingerprint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_query: Option<String>,
}

/// Stable identity of a semantic topology reference.
///
/// The persisted `ref_id` is the primary identity. The remaining fields
/// describe the semantic producer and role and are used to explain or recover
/// a reference after kernel topology IDs change. Kernel IDs, normals, and
/// geometric fingerprints are deliberately excluded: they are regeneration
/// hints, not identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopoRefIdentity {
    pub ref_id: TopoRefId,
    pub kind: TopoRefKind,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopoRefKind {
    Face,
    Edge,
    Vertex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopoRefSemantic {
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_hint: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometricFingerprint {
    pub surface_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_face_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_edge_id: Option<u64>,
    /// Inclusive area range in square meters. Reserved as a persisted hint;
    /// the P5-001 matcher does not score it yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_range: Option<[f64; 2]>,
    /// Legacy two-vector hint. Faces store centroid bounds in meters; edges
    /// store `[midpoint_m, unit_tangent]`. Kept unchanged for `.ocad`
    /// compatibility until a versioned fingerprint shape is introduced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox_hint: Option<[[f64; 3]; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjacent_feature_ids: Vec<String>,
}

/// Explicit units and thresholds used by fingerprint fallback matching.
///
/// This policy is a runtime contract, not a persisted `.ocad` field. The
/// default preserves the original edge matcher thresholds and applies the same
/// distance budget to face-centroid fallback, while making units and meaning
/// reviewable at each call site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TopoRefTolerancePolicy {
    /// Maximum face-centroid distance in meters for geometric fallback.
    pub face_centroid_tolerance_m: f64,
    /// Maximum edge-midpoint distance in meters for geometric fallback.
    pub edge_midpoint_tolerance_m: f64,
    /// Minimum absolute normal dot product (dimensionless).
    pub normal_alignment_min_dot: f64,
    /// Minimum absolute tangent dot product (dimensionless).
    pub tangent_alignment_min_dot: f64,
    /// Epsilon for rejecting near-zero direction vectors (dimensionless).
    pub vector_norm_epsilon: f64,
}

pub const DEFAULT_FACE_CENTROID_TOLERANCE_M: f64 = 0.002;
pub const DEFAULT_EDGE_MIDPOINT_TOLERANCE_M: f64 = 0.002;
pub const DEFAULT_NORMAL_ALIGNMENT_MIN_DOT: f64 = 0.99;
pub const DEFAULT_TANGENT_ALIGNMENT_MIN_DOT: f64 = 0.99;
pub const DEFAULT_VECTOR_NORM_EPSILON: f64 = 1e-9;

impl Default for TopoRefTolerancePolicy {
    fn default() -> Self {
        Self {
            face_centroid_tolerance_m: DEFAULT_FACE_CENTROID_TOLERANCE_M,
            edge_midpoint_tolerance_m: DEFAULT_EDGE_MIDPOINT_TOLERANCE_M,
            normal_alignment_min_dot: DEFAULT_NORMAL_ALIGNMENT_MIN_DOT,
            tangent_alignment_min_dot: DEFAULT_TANGENT_ALIGNMENT_MIN_DOT,
            vector_norm_epsilon: DEFAULT_VECTOR_NORM_EPSILON,
        }
    }
}

impl TopoRefTolerancePolicy {
    /// Validate finite, positive distances/epsilon and dimensionless dot bounds.
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("face_centroid_tolerance_m", self.face_centroid_tolerance_m),
            ("edge_midpoint_tolerance_m", self.edge_midpoint_tolerance_m),
            ("vector_norm_epsilon", self.vector_norm_epsilon),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(OpenCadError::validation(format!(
                    "topo ref tolerance '{name}' must be finite and strictly positive"
                )));
            }
        }
        for (name, value) in [
            ("normal_alignment_min_dot", self.normal_alignment_min_dot),
            ("tangent_alignment_min_dot", self.tangent_alignment_min_dot),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(OpenCadError::validation(format!(
                    "topo ref tolerance '{name}' must be a finite absolute dot product in [0, 1]"
                )));
            }
        }
        Ok(())
    }
}

impl TopoRef {
    pub fn face(ref_id: TopoRefId, created_by: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            ref_id,
            kind: TopoRefKind::Face,
            semantic: TopoRefSemantic {
                created_by: created_by.into(),
                role: Some(role.into()),
                normal_hint: None,
                intent: None,
            },
            geometric_fingerprint: None,
            fallback_query: None,
        }
    }

    pub fn kernel_face(
        ref_id: TopoRefId,
        created_by: impl Into<String>,
        role: impl Into<String>,
        kernel_face_id: u64,
        normal_hint: [f32; 3],
    ) -> Self {
        Self {
            ref_id,
            kind: TopoRefKind::Face,
            semantic: TopoRefSemantic {
                created_by: created_by.into(),
                role: Some(role.into()),
                normal_hint: Some([
                    normal_hint[0] as f64,
                    normal_hint[1] as f64,
                    normal_hint[2] as f64,
                ]),
                intent: None,
            },
            geometric_fingerprint: Some(GeometricFingerprint {
                surface_type: "brep_face".into(),
                kernel_face_id: Some(kernel_face_id),
                kernel_edge_id: None,
                area_range: None,
                bbox_hint: None,
                adjacent_feature_ids: Vec::new(),
            }),
            fallback_query: None,
        }
    }

    pub fn edge(ref_id: TopoRefId, created_by: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            ref_id,
            kind: TopoRefKind::Edge,
            semantic: TopoRefSemantic {
                created_by: created_by.into(),
                role: Some(role.into()),
                normal_hint: None,
                intent: None,
            },
            geometric_fingerprint: None,
            fallback_query: None,
        }
    }

    pub fn kernel_edge(
        ref_id: TopoRefId,
        created_by: impl Into<String>,
        role: impl Into<String>,
        kernel_edge_id: u64,
        midpoint_hint: [f32; 3],
        tangent_hint: [f32; 3],
    ) -> Self {
        Self {
            ref_id,
            kind: TopoRefKind::Edge,
            semantic: TopoRefSemantic {
                created_by: created_by.into(),
                role: Some(role.into()),
                normal_hint: None,
                intent: None,
            },
            geometric_fingerprint: Some(GeometricFingerprint {
                surface_type: "brep_edge".into(),
                kernel_face_id: None,
                kernel_edge_id: Some(kernel_edge_id),
                area_range: None,
                bbox_hint: Some([
                    [
                        midpoint_hint[0] as f64,
                        midpoint_hint[1] as f64,
                        midpoint_hint[2] as f64,
                    ],
                    [
                        tangent_hint[0] as f64,
                        tangent_hint[1] as f64,
                        tangent_hint[2] as f64,
                    ],
                ]),
                adjacent_feature_ids: Vec::new(),
            }),
            fallback_query: None,
        }
    }

    pub fn kernel_face_id(&self) -> Option<u64> {
        self.geometric_fingerprint
            .as_ref()
            .and_then(|fingerprint| fingerprint.kernel_face_id)
    }

    pub fn kernel_edge_id(&self) -> Option<u64> {
        self.geometric_fingerprint
            .as_ref()
            .and_then(|fingerprint| fingerprint.kernel_edge_id)
    }

    /// Return the stable semantic identity, excluding regeneration hints.
    pub fn identity(&self) -> TopoRefIdentity {
        TopoRefIdentity {
            ref_id: self.ref_id.clone(),
            kind: self.kind,
            created_by: self.semantic.created_by.clone(),
            role: self.semantic.role.clone(),
            intent: self.semantic.intent.clone(),
        }
    }

    /// Alias that makes the semantic (rather than geometric) nature explicit
    /// at call sites.
    pub fn semantic_identity(&self) -> TopoRefIdentity {
        self.identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topo_ref_round_trip() {
        let topo = TopoRef::face(
            TopoRefId::new("ref:face:base_top").expect("id"),
            "feature:extrude_base",
            "top_face",
        );
        let json = serde_json::to_string(&topo).expect("serialize");
        let restored: TopoRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(topo, restored);
    }

    #[test]
    fn identity_excludes_kernel_and_geometric_hints() {
        let mut first = TopoRef::face(
            TopoRefId::new("ref:face:base_top").expect("id"),
            "feature:extrude",
            "top",
        );
        first.semantic.intent = Some("mounting face".into());
        let mut second = first.clone();
        second.geometric_fingerprint = Some(GeometricFingerprint {
            surface_type: "brep_face".into(),
            kernel_face_id: Some(42),
            kernel_edge_id: None,
            area_range: Some([0.01, 0.02]),
            bbox_hint: Some([[0.0, 0.0, 0.0], [0.01, 0.01, 0.01]]),
            adjacent_feature_ids: vec!["feature:fillet".into()],
        });
        second.semantic.normal_hint = Some([0.0, 0.0, 1.0]);
        assert_eq!(first.identity(), second.identity());
        assert_eq!(first.identity(), first.semantic_identity());
    }

    #[test]
    fn tolerance_policy_is_serializable_and_unit_explicit() {
        let policy = TopoRefTolerancePolicy::default();
        let json = serde_json::to_string(&policy).expect("policy JSON");
        assert_eq!(
            json,
            r#"{"face_centroid_tolerance_m":0.002,"edge_midpoint_tolerance_m":0.002,"normal_alignment_min_dot":0.99,"tangent_alignment_min_dot":0.99,"vector_norm_epsilon":1e-9}"#
        );
        assert!(policy.validate().is_ok());
        assert!(TopoRefTolerancePolicy {
            normal_alignment_min_dot: 1.1,
            ..policy
        }
        .validate()
        .is_err());
        assert!(TopoRefTolerancePolicy {
            normal_alignment_min_dot: -0.1,
            ..policy
        }
        .validate()
        .is_err());
        assert!(TopoRefTolerancePolicy {
            edge_midpoint_tolerance_m: 0.0,
            ..policy
        }
        .validate()
        .is_err());
    }

    #[test]
    fn semantic_toporef_example_uses_the_existing_json_shape() {
        let json = include_str!("../../../examples/topo-ref-semantic.json");
        let topo_ref: TopoRef = serde_json::from_str(json).expect("TopoRef example");
        assert_eq!(topo_ref.ref_id.as_str(), "ref:face:bracket_top");
        assert_eq!(topo_ref.identity().created_by, "feature:extrude_base");
        assert_eq!(topo_ref.kernel_face_id(), Some(42));
    }
}
