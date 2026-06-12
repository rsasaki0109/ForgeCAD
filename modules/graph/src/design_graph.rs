use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use opencad_core::OpenCadError;
use opencad_core::Result;

use crate::dependency::{topological_sort, DependencyEdge, EdgeKind};

/// Discriminated node kinds in the design graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphNodeKind {
    Document,
    Parameter,
    Sketch,
    Constraint,
    Feature,
    Body,
    FaceRef,
    EdgeRef,
    Material,
    Validation,
    Intent,
}

impl GraphNodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Parameter => "parameter",
            Self::Sketch => "sketch",
            Self::Constraint => "constraint",
            Self::Feature => "feature",
            Self::Body => "body",
            Self::FaceRef => "face_ref",
            Self::EdgeRef => "edge_ref",
            Self::Material => "material",
            Self::Validation => "validation",
            Self::Intent => "intent",
        }
    }
}

/// A node in the OpenCAD design graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub name: String,
    #[serde(default)]
    pub dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

impl GraphNode {
    pub fn new(id: impl Into<String>, kind: GraphNodeKind, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            name: name.into(),
            dirty: false,
            role: None,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }
}

/// Authoritative design intent graph.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DesignGraph {
    nodes: IndexMap<String, GraphNode>,
    edges: Vec<DependencyEdge>,
}

impl DesignGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    pub fn edges(&self) -> &[DependencyEdge] {
        &self.edges
    }

    pub fn add_node(&mut self, node: GraphNode) -> Result<()> {
        if self.nodes.contains_key(&node.id) {
            return Err(OpenCadError::validation(format!(
                "node '{}' already exists",
                node.id
            )));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.nodes.get_mut(id)
    }

    pub fn add_edge(&mut self, edge: DependencyEdge) -> Result<()> {
        if !self.nodes.contains_key(&edge.source) {
            return Err(OpenCadError::validation(format!(
                "edge source '{}' not found",
                edge.source
            )));
        }
        if !self.nodes.contains_key(&edge.target) {
            return Err(OpenCadError::validation(format!(
                "edge target '{}' not found",
                edge.target
            )));
        }
        self.edges.push(edge);
        Ok(())
    }

    pub fn find_by_id(&self, id: &str) -> Option<&GraphNode> {
        self.get_node(id)
    }

    pub fn find_by_type(&self, kind: GraphNodeKind) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.kind == kind).collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.name == name).collect()
    }

    pub fn find_by_role(&self, role: &str) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.role.as_deref() == Some(role))
            .collect()
    }

    pub fn dependency_order(&self) -> Result<Vec<String>> {
        let ids: Vec<String> = self.nodes.keys().cloned().collect();
        topological_sort(&ids, &self.edges).map_err(OpenCadError::validation)
    }

    /// Mark `node_id` dirty and propagate through outgoing `DependsOn` edges.
    pub fn mark_dirty(&mut self, node_id: &str) {
        let mut queue = vec![node_id.to_string()];
        let mut visited = indexmap::IndexSet::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(&current) {
                node.dirty = true;
            }
            for edge in &self.edges {
                if edge.kind == EdgeKind::DependsOn && edge.source == current {
                    queue.push(edge.target.clone());
                }
            }
        }
    }

    pub fn clear_dirty(&mut self) {
        for node in self.nodes.values_mut() {
            node.dirty = false;
        }
    }

    pub fn dirty_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.values().filter(|n| n.dirty).collect()
    }

    /// Direct `DependsOn` predecessors of a node.
    pub fn dependencies_of(&self, node_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.kind == EdgeKind::DependsOn && e.target == node_id)
            .map(|e| e.source.as_str())
            .collect()
    }

    /// Direct `DependsOn` successors of a node.
    pub fn dependents_of(&self, node_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.kind == EdgeKind::DependsOn && e.source == node_id)
            .map(|e| e.target.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> DesignGraph {
        let mut graph = DesignGraph::new();
        graph
            .add_node(GraphNode::new("doc:part", GraphNodeKind::Document, "Part"))
            .unwrap();
        graph
            .add_node(GraphNode::new(
                "param:width",
                GraphNodeKind::Parameter,
                "width",
            ))
            .unwrap();
        graph
            .add_node(GraphNode::new(
                "sketch:base",
                GraphNodeKind::Sketch,
                "Base Sketch",
            ))
            .unwrap();
        graph
            .add_node(GraphNode::new(
                "feature:extrude",
                GraphNodeKind::Feature,
                "Extrude",
            ))
            .unwrap();
        graph
            .add_edge(DependencyEdge::depends_on("param:width", "sketch:base"))
            .unwrap();
        graph
            .add_edge(DependencyEdge::depends_on("sketch:base", "feature:extrude"))
            .unwrap();
        graph
    }

    #[test]
    fn add_and_query_nodes() {
        let graph = sample_graph();
        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.find_by_type(GraphNodeKind::Parameter).len(), 1);
        assert_eq!(graph.find_by_name("width").len(), 1);
    }

    #[test]
    fn dependency_order_is_valid() {
        let graph = sample_graph();
        let order = graph.dependency_order().expect("order");
        let width_pos = order.iter().position(|id| id == "param:width").unwrap();
        let sketch_pos = order.iter().position(|id| id == "sketch:base").unwrap();
        let extrude_pos = order.iter().position(|id| id == "feature:extrude").unwrap();
        assert!(width_pos < sketch_pos);
        assert!(sketch_pos < extrude_pos);
    }

    #[test]
    fn dirty_propagation_follows_dependencies() {
        let mut graph = sample_graph();
        graph.mark_dirty("param:width");
        let dirty: Vec<_> = graph.dirty_nodes().iter().map(|n| n.id.as_str()).collect();
        assert!(dirty.contains(&"param:width"));
        assert!(dirty.contains(&"sketch:base"));
        assert!(dirty.contains(&"feature:extrude"));
    }
}
