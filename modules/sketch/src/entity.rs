use serde::{Deserialize, Serialize};

use opencad_core::{EntityId, Expression, OpenCadError, Result};

/// 2D coordinate value: literal number or parametric expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Coord {
    Literal(f64),
    Expr(Expression),
}

impl Coord {
    pub fn literal(value: f64) -> Self {
        Self::Literal(value)
    }

    pub fn expr(value: impl Into<String>) -> Result<Self> {
        Ok(Self::Expr(Expression::new(value)?))
    }
}

/// Base fields shared by all sketch entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityBase {
    pub id: EntityId,
    #[serde(default)]
    pub construction: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointEntity {
    #[serde(flatten)]
    pub base: EntityBase,
    pub x: Coord,
    pub y: Coord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineEntity {
    #[serde(flatten)]
    pub base: EntityBase,
    pub start: EntityId,
    pub end: EntityId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircleEntity {
    #[serde(flatten)]
    pub base: EntityBase,
    pub center: EntityId,
    pub radius: Coord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArcEntity {
    #[serde(flatten)]
    pub base: EntityBase,
    pub center: EntityId,
    pub radius: Coord,
    pub start_angle: Coord,
    pub end_angle: Coord,
}

/// Rectangle helper: stores parametric origin/size and expands to four lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RectangleEntity {
    #[serde(flatten)]
    pub base: EntityBase,
    pub origin: [Coord; 2],
    pub size: [Coord; 2],
    /// Generated corner point IDs (filled by `expand_rectangle`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corner_ids: Vec<EntityId>,
    /// Generated edge line IDs (filled by `expand_rectangle`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SketchEntity {
    Point(PointEntity),
    Line(LineEntity),
    Circle(CircleEntity),
    Arc(ArcEntity),
    Rectangle(RectangleEntity),
}

impl SketchEntity {
    pub fn id(&self) -> &EntityId {
        match self {
            Self::Point(e) => &e.base.id,
            Self::Line(e) => &e.base.id,
            Self::Circle(e) => &e.base.id,
            Self::Arc(e) => &e.base.id,
            Self::Rectangle(e) => &e.base.id,
        }
    }

    pub fn is_construction(&self) -> bool {
        match self {
            Self::Point(e) => e.base.construction,
            Self::Line(e) => e.base.construction,
            Self::Circle(e) => e.base.construction,
            Self::Arc(e) => e.base.construction,
            Self::Rectangle(e) => e.base.construction,
        }
    }
}

/// Expand a rectangle helper into four corner points and four edge lines.
pub fn expand_rectangle(rect: &RectangleEntity) -> Result<Vec<SketchEntity>> {
    if rect.corner_ids.len() != 4 || rect.edge_ids.len() != 4 {
        return Err(OpenCadError::validation(
            "rectangle must have 4 corner_ids and 4 edge_ids before expansion",
        ));
    }

    let [c0, c1, c2, c3] = [
        &rect.corner_ids[0],
        &rect.corner_ids[1],
        &rect.corner_ids[2],
        &rect.corner_ids[3],
    ];
    let [e0, e1, e2, e3] = [
        &rect.edge_ids[0],
        &rect.edge_ids[1],
        &rect.edge_ids[2],
        &rect.edge_ids[3],
    ];

    let ox = &rect.origin[0];
    let oy = &rect.origin[1];
    let w = &rect.size[0];
    let h = &rect.size[1];

    let corner_coords = [
        (ox.clone(), oy.clone()),
        (add_coords(ox, w)?, oy.clone()),
        (add_coords(ox, w)?, add_coords(oy, h)?),
        (ox.clone(), add_coords(oy, h)?),
    ];

    let mut entities = Vec::with_capacity(8);
    let corners = [
        (c0, corner_coords[0].0.clone(), corner_coords[0].1.clone()),
        (c1, corner_coords[1].0.clone(), corner_coords[1].1.clone()),
        (c2, corner_coords[2].0.clone(), corner_coords[2].1.clone()),
        (c3, corner_coords[3].0.clone(), corner_coords[3].1.clone()),
    ];
    for (id, x, y) in corners {
        entities.push(SketchEntity::Point(PointEntity {
            base: EntityBase {
                id: id.clone(),
                construction: rect.base.construction,
            },
            x,
            y,
        }));
    }

    let edges = [(e0, c0, c1), (e1, c1, c2), (e2, c2, c3), (e3, c3, c0)];
    for (id, start, end) in edges {
        entities.push(SketchEntity::Line(LineEntity {
            base: EntityBase {
                id: id.clone(),
                construction: rect.base.construction,
            },
            start: start.clone(),
            end: end.clone(),
        }));
    }

    Ok(entities)
}

fn add_coords(a: &Coord, b: &Coord) -> Result<Coord> {
    match (a, b) {
        (Coord::Literal(x), Coord::Literal(y)) => Ok(Coord::Literal(x + y)),
        (Coord::Expr(x), Coord::Literal(y)) => Ok(Coord::Expr(Expression::new(format!(
            "{} + {}",
            x.as_str(),
            y
        ))?)),
        (Coord::Literal(x), Coord::Expr(y)) => Ok(Coord::Expr(Expression::new(format!(
            "{x} + {}",
            y.as_str()
        ))?)),
        (Coord::Expr(x), Coord::Expr(y)) => Ok(Coord::Expr(Expression::new(format!(
            "{} + {}",
            x.as_str(),
            y.as_str()
        ))?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(id: &str) -> EntityId {
        EntityId::new(id).expect("valid id")
    }

    #[test]
    fn point_entity_round_trip() {
        let point = SketchEntity::Point(PointEntity {
            base: EntityBase {
                id: ent("ent:pt_1"),
                construction: false,
            },
            x: Coord::literal(10.0),
            y: Coord::literal(20.0),
        });
        let json = serde_json::to_string(&point).expect("serialize");
        let restored: SketchEntity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(point, restored);
    }

    #[test]
    fn line_entity_references_points() {
        let line = SketchEntity::Line(LineEntity {
            base: EntityBase {
                id: ent("ent:line_1"),
                construction: false,
            },
            start: ent("ent:pt_1"),
            end: ent("ent:pt_2"),
        });
        assert_eq!(line.id().as_str(), "ent:line_1");
    }

    #[test]
    fn circle_with_expression_radius() {
        let circle = SketchEntity::Circle(CircleEntity {
            base: EntityBase {
                id: ent("ent:circle_1"),
                construction: false,
            },
            center: ent("ent:pt_center"),
            radius: Coord::expr("5 mm").expect("expr"),
        });
        let json = serde_json::to_string(&circle).expect("serialize");
        assert!(json.contains("5 mm"));
    }

    #[test]
    fn construction_flag_is_preserved() {
        let line = SketchEntity::Line(LineEntity {
            base: EntityBase {
                id: ent("ent:const_line"),
                construction: true,
            },
            start: ent("ent:pt_1"),
            end: ent("ent:pt_2"),
        });
        assert!(line.is_construction());
    }

    #[test]
    fn rectangle_expands_to_points_and_lines() {
        let rect = RectangleEntity {
            base: EntityBase {
                id: ent("ent:rect_1"),
                construction: false,
            },
            origin: [Coord::literal(0.0), Coord::literal(0.0)],
            size: [Coord::literal(80.0), Coord::literal(60.0)],
            corner_ids: vec![
                ent("ent:rect_1_c0"),
                ent("ent:rect_1_c1"),
                ent("ent:rect_1_c2"),
                ent("ent:rect_1_c3"),
            ],
            edge_ids: vec![
                ent("ent:rect_1_e0"),
                ent("ent:rect_1_e1"),
                ent("ent:rect_1_e2"),
                ent("ent:rect_1_e3"),
            ],
        };
        let expanded = expand_rectangle(&rect).expect("expand");
        assert_eq!(expanded.len(), 8);
        assert_eq!(
            expanded
                .iter()
                .filter(|e| matches!(e, SketchEntity::Point(_)))
                .count(),
            4
        );
        assert_eq!(
            expanded
                .iter()
                .filter(|e| matches!(e, SketchEntity::Line(_)))
                .count(),
            4
        );
    }
}
