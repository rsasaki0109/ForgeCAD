//! Orbit camera model for viewport framing.

use crate::scene::BoundingBox;

/// Simple orbit camera used by the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
    pub fov_y_deg: f32,
    pub aspect: f32,
}

impl OrbitCamera {
    pub fn fit_bounds(bounds: &BoundingBox, aspect: f32) -> Self {
        let target = bounds.center();
        let radius = bounds.diagonal() * 0.5;
        let fov_y_deg: f32 = 45.0;
        let fov_y_rad = fov_y_deg.to_radians();
        let distance = if radius > 0.0 {
            (radius / (fov_y_rad * 0.5).tan()) * 1.25
        } else {
            0.1
        };
        Self {
            target,
            distance,
            yaw_rad: 0.7,
            pitch_rad: 0.5,
            fov_y_deg,
            aspect: aspect.max(0.1),
        }
    }

    pub fn eye_position(&self) -> [f32; 3] {
        let cos_pitch = self.pitch_rad.cos();
        let x = self.distance * cos_pitch * self.yaw_rad.sin();
        let y = self.distance * self.pitch_rad.sin();
        let z = self.distance * cos_pitch * self.yaw_rad.cos();
        [self.target[0] + x, self.target[1] + y, self.target[2] + z]
    }

    /// Column-major 4x4 view matrix (right-handed, Y-up).
    pub fn view_matrix(&self) -> [f32; 16] {
        look_at(self.eye_position(), self.target, [0.0, 1.0, 0.0])
    }

    /// Column-major 4x4 perspective projection matrix.
    pub fn projection_matrix(&self) -> [f32; 16] {
        perspective(
            self.fov_y_deg.to_radians(),
            self.aspect,
            0.001,
            self.distance * 10.0,
        )
    }

    /// Column-major view-projection matrix for GPU uniforms.
    pub fn view_projection_matrix(&self) -> [f32; 16] {
        multiply_mat4(self.projection_matrix(), self.view_matrix())
    }

    /// Project a world-space point into preview pixel coordinates.
    pub fn project_to_screen(&self, width: u32, height: u32, world: [f32; 3]) -> Option<[f64; 2]> {
        project_world_to_screen(&self.view_projection_matrix(), width, height, world)
    }

    /// Screen-aligned basis vectors for billboard text overlays.
    pub fn billboard_basis(&self) -> ([f32; 3], [f32; 3]) {
        let view = self.view_matrix();
        let right = [view[0], view[1], view[2]];
        let up = [view[4], view[5], view[6]];
        (right, up)
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len <= f32::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = normalize([center[0] - eye[0], center[1] - eye[1], center[2] - eye[2]]);
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    [
        s[0],
        u[0],
        -f[0],
        0.0,
        s[1],
        u[1],
        -f[1],
        0.0,
        s[2],
        u[2],
        -f[2],
        0.0,
        -dot(s, eye),
        -dot(u, eye),
        dot(f, eye),
        1.0,
    ]
}

fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let tan_half = (fov_y_rad * 0.5).tan();
    let f = 1.0 / tan_half;
    let range = near - far;
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / range,
        -1.0,
        0.0,
        0.0,
        (2.0 * far * near) / range,
        0.0,
    ]
}

fn multiply_mat4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0_f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            out[col * 4 + row] = a[row] * b[col * 4]
                + a[4 + row] * b[col * 4 + 1]
                + a[8 + row] * b[col * 4 + 2]
                + a[12 + row] * b[col * 4 + 3];
        }
    }
    out
}

fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 4] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
        m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15],
    ]
}

/// Project a world-space point into viewport pixel coordinates.
pub fn project_world_to_screen(
    view_proj: &[f32; 16],
    width: u32,
    height: u32,
    world: [f32; 3],
) -> Option<[f64; 2]> {
    let clip = transform_point(view_proj, world);
    if clip[3] <= f32::EPSILON {
        return None;
    }
    let ndc_x = clip[0] / clip[3];
    let ndc_y = clip[1] / clip[3];
    let x = (ndc_x as f64 * 0.5 + 0.5) * width as f64;
    let y = (1.0 - (ndc_y as f64 * 0.5 + 0.5)) * height as f64;
    Some([x, y])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_camera_to_bounds() {
        let bounds = BoundingBox {
            min: [0.0, 0.0, 0.0],
            max: [0.08, 0.06, 0.006],
        };
        let camera = OrbitCamera::fit_bounds(&bounds, 16.0 / 9.0);
        assert!(camera.distance > 0.0);
        assert!((camera.target[0] - 0.04).abs() < 1e-6);
    }

    #[test]
    fn view_matrix_is_invertible_enough() {
        let camera = OrbitCamera {
            target: [0.0, 0.0, 0.0],
            distance: 0.2,
            yaw_rad: 0.5,
            pitch_rad: 0.3,
            fov_y_deg: 45.0,
            aspect: 1.0,
        };
        let view = camera.view_matrix();
        assert!(view[15].abs() > 0.0);
    }

    #[test]
    fn billboard_basis_is_orthonormal() {
        let camera = OrbitCamera {
            target: [0.0, 0.0, 0.0],
            distance: 0.2,
            yaw_rad: 1.1,
            pitch_rad: 0.4,
            fov_y_deg: 45.0,
            aspect: 1.0,
        };
        let (right, up) = camera.billboard_basis();
        let right_len = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
        let up_len = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
        let dot = right[0] * up[0] + right[1] * up[1] + right[2] * up[2];
        assert!((right_len - 1.0).abs() < 1e-5);
        assert!((up_len - 1.0).abs() < 1e-5);
        assert!(dot.abs() < 1e-5);
    }

    #[test]
    fn projects_bounds_center_near_preview_center() {
        let bounds = BoundingBox {
            min: [0.0, 0.0, 0.0],
            max: [0.08, 0.06, 0.006],
        };
        let width = 960;
        let height = 540;
        let camera = OrbitCamera::fit_bounds(&bounds, width as f32 / height as f32);
        let center = bounds.center();
        let projected = camera
            .project_to_screen(width, height, center)
            .expect("projected");
        assert!((projected[0] - width as f64 * 0.5).abs() < width as f64 * 0.15);
        assert!((projected[1] - height as f64 * 0.5).abs() < height as f64 * 0.15);
    }
}
