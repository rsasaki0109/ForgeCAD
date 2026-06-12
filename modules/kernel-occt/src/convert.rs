#[cfg(feature = "occt")]
use cadrum::{DVec3, Edge, Error as OcctError};

use opencad_core::{OpenCadError, Result};
use opencad_geometry::SolvedSketch;

#[cfg(feature = "occt")]
pub fn sketch_to_edges(sketch: &SolvedSketch) -> Result<Vec<Edge>> {
    if sketch.points.len() < 3 {
        return Err(OpenCadError::validation(
            "profile needs at least three points",
        ));
    }
    if !sketch.closed {
        return Err(OpenCadError::validation(
            "only closed profiles can be extruded in MVP",
        ));
    }

    let points: Vec<DVec3> = sketch
        .points
        .iter()
        .map(|p| DVec3::new(p[0], p[1], 0.0))
        .collect();

    Edge::polygon(&points).map_err(map_occt_error)
}

#[cfg(feature = "occt")]
pub fn sketch_to_edge(sketch: &SolvedSketch) -> Result<Edge> {
    let edges = sketch_to_edges(sketch)?;
    edges
        .into_iter()
        .next()
        .ok_or_else(|| OpenCadError::validation("polygon produced no edges"))
}

#[cfg(feature = "occt")]
pub fn map_occt_error(err: OcctError) -> OpenCadError {
    OpenCadError::Other(format!("OCCT error: {err}"))
}

#[cfg(not(feature = "occt"))]
pub fn sketch_to_edge(_sketch: &SolvedSketch) -> Result<()> {
    Err(OpenCadError::Other(
        "OCCT backend not enabled; rebuild with --features occt".into(),
    ))
}
