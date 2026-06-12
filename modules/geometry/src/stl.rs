//! Binary STL export from tessellated meshes.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use opencad_core::{OpenCadError, Result};

use crate::tessellation::MeshSet;

/// Write a binary STL file from a tessellated mesh.
pub fn write_binary_stl(path: impl AsRef<Path>, mesh: &MeshSet, name: &str) -> Result<()> {
    if mesh.indices.len() < 3 || mesh.indices.len() % 3 != 0 {
        return Err(OpenCadError::validation(
            "mesh must contain at least one triangle",
        ));
    }

    let mut file = File::create(path.as_ref()).map_err(io_error)?;
    let header = stl_header(name);
    file.write_all(&header).map_err(io_error)?;

    let triangle_count = (mesh.indices.len() / 3) as u32;
    file.write_all(&triangle_count.to_le_bytes()).map_err(io_error)?;

    for chunk in mesh.indices.chunks_exact(3) {
        let v0 = mesh.positions[chunk[0] as usize];
        let v1 = mesh.positions[chunk[1] as usize];
        let v2 = mesh.positions[chunk[2] as usize];
        let normal = triangle_normal(v0, v1, v2);
        file.write_all(&normal[0].to_le_bytes()).map_err(io_error)?;
        file.write_all(&normal[1].to_le_bytes()).map_err(io_error)?;
        file.write_all(&normal[2].to_le_bytes()).map_err(io_error)?;
        for vertex in [v0, v1, v2] {
            file.write_all(&vertex[0].to_le_bytes()).map_err(io_error)?;
            file.write_all(&vertex[1].to_le_bytes()).map_err(io_error)?;
            file.write_all(&vertex[2].to_le_bytes()).map_err(io_error)?;
        }
        file.write_all(&0u16.to_le_bytes()).map_err(io_error)?;
    }

    Ok(())
}

fn stl_header(name: &str) -> [u8; 80] {
    let mut header = [0u8; 80];
    let bytes = name.as_bytes();
    let len = bytes.len().min(80);
    header[..len].copy_from_slice(&bytes[..len]);
    header
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ux = b[0] - a[0];
    let uy = b[1] - a[1];
    let uz = b[2] - a[2];
    let vx = c[0] - a[0];
    let vy = c[1] - a[1];
    let vz = c[2] - a[2];
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len <= f32::EPSILON {
        return [0.0, 0.0, 1.0];
    }
    [nx / len, ny / len, nz / len]
}

fn io_error(err: std::io::Error) -> OpenCadError {
    OpenCadError::Other(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tessellation::TessellationSettings;
    use crate::MeshSet;

    #[test]
    fn writes_binary_stl_for_box_mesh() {
        let mesh = MeshSet::box_prism(0.01, TessellationSettings::default().linear_deflection);
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("box.stl");
        write_binary_stl(&path, &mesh, "box").expect("write");
        let bytes = std::fs::read(&path).expect("read");
        assert!(bytes.len() > 84);
        let count = u32::from_le_bytes(bytes[80..84].try_into().expect("count"));
        assert_eq!(count as usize, mesh.triangle_count());
    }
}
