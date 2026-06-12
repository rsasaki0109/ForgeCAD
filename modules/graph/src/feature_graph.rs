use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use opencad_core::OpenCadError;
use opencad_core::Result;

use crate::dependency::{topological_sort, DependencyEdge};
use crate::design_graph::{DesignGraph, GraphNode, GraphNodeKind};

/// Feature node in the ordered feature tree / dependency DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureEntry {
    pub id: String,
    pub name: String,
    pub feature_type: String,
    #[serde(default)]
    pub suppressed: bool,
    #[serde(default)]
    pub dirty: bool,
}

impl FeatureEntry {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        feature_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            feature_type: feature_type.into(),
            suppressed: false,
            dirty: false,
        }
    }
}

/// Ordered feature list plus dependency DAG for recomputation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureGraph {
    /// UI display order.
    order: Vec<String>,
    features: IndexMap<String, FeatureEntry>,
    edges: Vec<DependencyEdge>,
}

impl FeatureGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ordered_ids(&self) -> &[String] {
        &self.order
    }

    pub fn add_feature(&mut self, entry: FeatureEntry) -> Result<()> {
        if self.features.contains_key(&entry.id) {
            return Err(OpenCadError::validation(format!(
                "feature '{}' already exists",
                entry.id
            )));
        }
        self.order.push(entry.id.clone());
        self.features.insert(entry.id.clone(), entry);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&FeatureEntry> {
        self.features.get(id)
    }

    pub fn add_dependency(
        &mut self,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<()> {
        let source = source.into();
        let target = target.into();
        if !self.features.contains_key(&source) {
            return Err(OpenCadError::validation(format!(
                "feature source '{source}' not found"
            )));
        }
        if !self.features.contains_key(&target) {
            return Err(OpenCadError::validation(format!(
                "feature target '{target}' not found"
            )));
        }
        self.edges.push(DependencyEdge::depends_on(source, target));
        Ok(())
    }

    pub fn recompute_order(&self) -> Result<Vec<String>> {
        let ids: Vec<String> = self.features.keys().cloned().collect();
        topological_sort(&ids, &self.edges).map_err(OpenCadError::validation)
    }

    /// All dependency edges recorded in this graph.
    pub fn dependency_edges(&self) -> &[DependencyEdge] {
        &self.edges
    }

    pub fn validate_order(&self) -> Result<()> {
        let topo = self.recompute_order()?;
        let mut last_pos = IndexMap::new();
        for (pos, id) in topo.iter().enumerate() {
            last_pos.insert(id.clone(), pos);
        }
        let mut prev = -1_i32;
        for id in &self.order {
            if let Some(entry) = self.features.get(id) {
                if entry.suppressed {
                    continue;
                }
            }
            let pos = *last_pos.get(id).ok_or_else(|| {
                OpenCadError::validation(format!("feature '{id}' missing from DAG"))
            })? as i32;
            if pos < prev {
                return Err(OpenCadError::validation(format!(
                    "feature order violates dependency: '{id}' must come before a dependent feature"
                )));
            }
            prev = pos;
        }
        Ok(())
    }

    pub fn mark_dirty(&mut self, feature_id: &str) {
        let mut queue = vec![feature_id.to_string()];
        let mut visited = indexmap::IndexSet::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(entry) = self.features.get_mut(&current) {
                entry.dirty = true;
            }
            for edge in &self.edges {
                if edge.source == current {
                    queue.push(edge.target.clone());
                }
            }
        }
    }

    pub fn sync_to_design_graph(&self, graph: &mut DesignGraph) -> Result<()> {
        for entry in self.features.values() {
            if graph.get_node(&entry.id).is_none() {
                graph.add_node(GraphNode::new(
                    entry.id.clone(),
                    GraphNodeKind::Feature,
                    entry.name.clone(),
                ))?;
            }
        }
        for edge in &self.edges {
            if graph
                .edges()
                .iter()
                .any(|e| e.source == edge.source && e.target == edge.target && e.kind == edge.kind)
            {
                continue;
            }
            graph.add_edge(edge.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bracket_features() -> FeatureGraph {
        let mut graph = FeatureGraph::new();
        graph
            .add_feature(FeatureEntry::new(
                "feature:sketch_base",
                "Base Sketch",
                "sketch",
            ))
            .unwrap();
        graph
            .add_feature(FeatureEntry::new(
                "feature:extrude_base",
                "Extrude",
                "extrude",
            ))
            .unwrap();
        graph
            .add_feature(FeatureEntry::new(
                "feature:hole_pattern",
                "Hole Pattern",
                "hole_pattern",
            ))
            .unwrap();
        graph
            .add_dependency("feature:sketch_base", "feature:extrude_base")
            .unwrap();
        graph
            .add_dependency("feature:extrude_base", "feature:hole_pattern")
            .unwrap();
        graph
    }

    #[test]
    fn feature_order_is_valid() {
        let graph = bracket_features();
        graph.validate_order().expect("valid order");
    }

    #[test]
    fn invalid_feature_order_is_detected() {
        let mut graph = bracket_features();
        graph.order.swap(1, 2);
        assert!(graph.validate_order().is_err());
    }

    #[test]
    fn dirty_propagation_through_feature_chain() {
        let mut graph = bracket_features();
        graph.mark_dirty("feature:sketch_base");
        assert!(graph.get("feature:extrude_base").unwrap().dirty);
        assert!(graph.get("feature:hole_pattern").unwrap().dirty);
    }
}
