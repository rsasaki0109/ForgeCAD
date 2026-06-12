use serde::{Deserialize, Serialize};

use opencad_core::{ConstraintId, EntityId, Expression};

/// Geometric or dimensional constraint in a sketch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    Coincident {
        id: ConstraintId,
        a: EntityRef,
        b: EntityRef,
    },
    Horizontal {
        id: ConstraintId,
        line: EntityId,
    },
    Vertical {
        id: ConstraintId,
        line: EntityId,
    },
    Parallel {
        id: ConstraintId,
        line_a: EntityId,
        line_b: EntityId,
    },
    Perpendicular {
        id: ConstraintId,
        line_a: EntityId,
        line_b: EntityId,
    },
    Distance {
        id: ConstraintId,
        target: DistanceTarget,
        expr: Expression,
    },
    Radius {
        id: ConstraintId,
        target: EntityId,
        expr: Expression,
    },
    Diameter {
        id: ConstraintId,
        target: EntityId,
        expr: Expression,
    },
    Equal {
        id: ConstraintId,
        a: EqualTarget,
        b: EqualTarget,
    },
}

/// Reference to a point, line, circle, or a sub-element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntityRef {
    Entity(EntityId),
    PointOnLine { line: EntityId, end: LineEnd },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEnd {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DistanceTarget {
    PointToPoint {
        a: EntityId,
        b: EntityId,
    },
    LineLength {
        line: EntityId,
    },
    /// Dimension on a rectangle edge (e.g. `ent:rect_1.width`).
    RectangleDimension {
        rectangle: EntityId,
        edge: RectangleEdge,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RectangleEdge {
    Width,
    Height,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EqualTarget {
    LineLength(EntityId),
    Radius(EntityId),
}

impl Constraint {
    pub fn id(&self) -> &ConstraintId {
        match self {
            Self::Coincident { id, .. }
            | Self::Horizontal { id, .. }
            | Self::Vertical { id, .. }
            | Self::Parallel { id, .. }
            | Self::Perpendicular { id, .. }
            | Self::Distance { id, .. }
            | Self::Radius { id, .. }
            | Self::Diameter { id, .. }
            | Self::Equal { id, .. } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(id: &str) -> ConstraintId {
        ConstraintId::new(id).expect("valid id")
    }

    fn eid(id: &str) -> EntityId {
        EntityId::new(id).expect("valid id")
    }

    #[test]
    fn coincident_constraint_round_trip() {
        let c = Constraint::Coincident {
            id: cid("con:coincident_1"),
            a: EntityRef::Entity(eid("ent:pt_1")),
            b: EntityRef::Entity(eid("ent:pt_2")),
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let restored: Constraint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, restored);
    }

    #[test]
    fn horizontal_constraint_round_trip() {
        let c = Constraint::Horizontal {
            id: cid("con:horiz_1"),
            line: eid("ent:line_1"),
        };
        round_trip(&c);
    }

    #[test]
    fn vertical_constraint_round_trip() {
        let c = Constraint::Vertical {
            id: cid("con:vert_1"),
            line: eid("ent:line_1"),
        };
        round_trip(&c);
    }

    #[test]
    fn parallel_constraint_round_trip() {
        let c = Constraint::Parallel {
            id: cid("con:parallel_1"),
            line_a: eid("ent:line_1"),
            line_b: eid("ent:line_2"),
        };
        round_trip(&c);
    }

    #[test]
    fn perpendicular_constraint_round_trip() {
        let c = Constraint::Perpendicular {
            id: cid("con:perp_1"),
            line_a: eid("ent:line_1"),
            line_b: eid("ent:line_2"),
        };
        round_trip(&c);
    }

    #[test]
    fn distance_constraint_round_trip() {
        let c = Constraint::Distance {
            id: cid("con:dist_1"),
            target: DistanceTarget::LineLength {
                line: eid("ent:line_1"),
            },
            expr: Expression::new("80 mm").expect("expr"),
        };
        round_trip(&c);
    }

    #[test]
    fn radius_constraint_round_trip() {
        let c = Constraint::Radius {
            id: cid("con:radius_1"),
            target: eid("ent:circle_1"),
            expr: Expression::new("10 mm").expect("expr"),
        };
        round_trip(&c);
    }

    #[test]
    fn diameter_constraint_round_trip() {
        let c = Constraint::Diameter {
            id: cid("con:diam_1"),
            target: eid("ent:circle_1"),
            expr: Expression::new("20 mm").expect("expr"),
        };
        round_trip(&c);
    }

    #[test]
    fn equal_constraint_round_trip() {
        let c = Constraint::Equal {
            id: cid("con:equal_1"),
            a: EqualTarget::LineLength(eid("ent:line_1")),
            b: EqualTarget::LineLength(eid("ent:line_2")),
        };
        round_trip(&c);
    }

    fn round_trip(c: &Constraint) {
        let json = serde_json::to_string(c).expect("serialize");
        let restored: Constraint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*c, restored);
    }
}
