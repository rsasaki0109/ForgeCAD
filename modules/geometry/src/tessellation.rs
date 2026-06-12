use serde::{Deserialize, Serialize};

/// Tessellation quality settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TessellationSettings {
    pub linear_deflection: f64,
    pub angular_deflection_deg: f64,
}

impl Default for TessellationSettings {
    fn default() -> Self {
        Self {
            linear_deflection: 0.001,
            angular_deflection_deg: 12.0,
        }
    }
}

/// Render mesh: positions and triangle indices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshSet {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// OCCT B-Rep face ID per triangle (`indices.len() / 3` entries).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triangle_face_ids: Vec<u64>,
}

impl MeshSet {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn has_triangle_face_ids(&self) -> bool {
        self.triangle_face_ids.len() == self.triangle_count()
    }

    pub fn box_prism(side_m: f64, _deflection: f64) -> Self {
        let s = side_m as f32;
        let positions = vec![
            [0.0, 0.0, 0.0],
            [s, 0.0, 0.0],
            [s, s, 0.0],
            [0.0, s, 0.0],
            [0.0, 0.0, s],
            [s, 0.0, s],
            [s, s, s],
            [0.0, s, s],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 8];
        let indices = vec![
            0, 1, 2, 0, 2, 3, // bottom
            4, 6, 5, 4, 7, 6, // top
        ];
        Self {
            positions,
            normals,
            indices,
            triangle_face_ids: vec![1, 1, 2, 2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_has_triangles() {
        let mesh = MeshSet::box_prism(0.01, 0.001);
        assert!(mesh.triangle_count() > 0);
    }

    #[test]
    fn box_prism_propagates_triangle_face_ids() {
        let mesh = MeshSet::box_prism(0.01, 0.001);
        assert!(mesh.has_triangle_face_ids());
        assert_eq!(mesh.triangle_face_ids, vec![1, 1, 2, 2]);
    }
}
