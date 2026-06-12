//! Topology hierarchy: Body → Lump → Shell → Face → Loop → CoEdge → Edge → Vertex.
//!
//! OpenCAD stores topology separately from geometry. Kernel B-Rep is a cache.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyTopology {
    pub lumps: Vec<Lump>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lump {
    pub shells: Vec<Shell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shell {
    pub faces: Vec<Face>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Face {
    pub id: String,
    pub loops: Vec<Loop>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loop {
    pub coedges: Vec<CoEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoEdge {
    pub edge_id: String,
    pub forward: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub start_vertex: String,
    pub end_vertex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    pub id: String,
    pub position: [f64; 3],
}
