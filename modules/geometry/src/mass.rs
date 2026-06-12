use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl BoundingBox {
    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn contains_point(&self, point: [f64; 3], tolerance: f64) -> bool {
        point.iter().zip(self.min).all(|(p, m)| p + tolerance >= m)
            && point.iter().zip(self.max).all(|(p, m)| p - tolerance <= m)
    }
}

/// Volume, area, mass, and center of gravity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MassProperties {
    pub volume_m3: f64,
    pub area_m2: f64,
    pub mass_kg: f64,
    pub center_of_mass: [f64; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_size() {
        let bbox = BoundingBox {
            min: [0.0, 0.0, 0.0],
            max: [0.08, 0.06, 0.006],
        };
        let size = bbox.size();
        assert!((size[0] - 0.08).abs() < 1e-12);
        assert!((size[2] - 0.006).abs() < 1e-12);
    }
}
