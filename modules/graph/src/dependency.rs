use serde::{Deserialize, Serialize};

/// Kind of relationship between design graph nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Target depends on source (source must be resolved first).
    DependsOn,
    /// Source creates target (e.g. extrude creates body).
    Creates,
    /// Source references target semantically (e.g. hole references face).
    References,
}

/// Directed edge in the design graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
}

impl DependencyEdge {
    pub fn depends_on(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            kind: EdgeKind::DependsOn,
        }
    }

    pub fn creates(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            kind: EdgeKind::Creates,
        }
    }

    pub fn references(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            kind: EdgeKind::References,
        }
    }
}

/// Detect cycles and return a topologically sorted node order.
///
/// Only `DependsOn` edges participate in ordering. `Creates` and `References`
/// are recorded but do not affect sort order.
pub fn topological_sort(nodes: &[String], edges: &[DependencyEdge]) -> Result<Vec<String>, String> {
    let depends: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::DependsOn)
        .collect();

    let mut in_degree: indexmap::IndexMap<String, usize> =
        nodes.iter().map(|id| (id.clone(), 0_usize)).collect();

    for edge in &depends {
        if let Some(deg) = in_degree.get_mut(&edge.target) {
            *deg += 1;
        }
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| id.clone())
        .collect();
    queue.sort();

    let mut order = Vec::with_capacity(nodes.len());

    while let Some(node) = queue.first().cloned() {
        queue.remove(0);
        order.push(node.clone());

        for edge in &depends {
            if edge.source != node {
                continue;
            }
            if let Some(deg) = in_degree.get_mut(&edge.target) {
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push(edge.target.clone());
                    queue.sort();
                }
            }
        }
    }

    if order.len() != nodes.len() {
        return Err("cyclic dependency detected".into());
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topological_sort_respects_dependencies() {
        let nodes = vec![
            "param:width".into(),
            "sketch:base".into(),
            "feature:extrude".into(),
        ];
        let edges = vec![
            DependencyEdge::depends_on("param:width", "sketch:base"),
            DependencyEdge::depends_on("sketch:base", "feature:extrude"),
        ];

        let order = topological_sort(&nodes, &edges).expect("sort");
        assert_eq!(order[0], "param:width");
        assert_eq!(order[1], "sketch:base");
        assert_eq!(order[2], "feature:extrude");
    }

    #[test]
    fn cyclic_graph_is_rejected() {
        let nodes = vec!["a".into(), "b".into()];
        let edges = vec![
            DependencyEdge::depends_on("a", "b"),
            DependencyEdge::depends_on("b", "a"),
        ];
        assert!(topological_sort(&nodes, &edges).is_err());
    }
}
