use indexmap::IndexMap;

use opencad_core::{OpenCadError, Result};
use opencad_solver::{
    point_x, point_y, radius_var, solve_with_diagnostics, ConstraintResidual, SolveStatus,
    SolverOptions, VarSet, VariableRegistry,
};

use crate::constraint::{Constraint, DistanceTarget, EntityRef, LineEnd};
use crate::entity::{Coord, LineEntity, PointEntity, SketchEntity};
use crate::solve_state::SolveState;
use crate::Sketch;

type SketchProblem = (
    Vec<ConstraintResidual>,
    VariableRegistry,
    IndexMap<String, (f64, f64)>,
);

/// Solve sketch constraints and write coordinates back into point entities.
pub fn solve_sketch(sketch: &mut Sketch, options: &SolverOptions) -> Result<SolveStatus> {
    let (mut equations, registry, point_coords) = build_problem(sketch)?;

    // Anchor the first point to remove translation DOF.
    if let Some((id, (x, y))) = point_coords.iter().next() {
        if let (Some(x_id), Some(y_id)) = (
            registry.get(&format!("{id}.x")),
            registry.get(&format!("{id}.y")),
        ) {
            equations.push(ConstraintResidual::FixedX { x: x_id, value: *x });
            equations.push(ConstraintResidual::FixedY { y: y_id, value: *y });
        }
    }

    if equations.is_empty() {
        sketch.solve_state = SolveState::UnderConstrained {
            dof: registry.len() as i32,
        };
        return Ok(SolveStatus::UnderConstrained {
            dof: registry.len() as i32,
            iterations: 0,
            max_error: 0.0,
        });
    }

    let mut values = registry.initial_values();
    for (key, (x, y)) in &point_coords {
        if let Some(x_id) = registry.get(&format!("{key}.x")) {
            values[x_id.index()] = *x;
        }
        if let Some(y_id) = registry.get(&format!("{key}.y")) {
            values[y_id.index()] = *y;
        }
    }
    seed_circle_radius(&registry, &mut values, sketch);

    let vars = VarSet::new(values);
    let (output, status) = solve_with_diagnostics(&equations, vars, options);
    apply_solution(sketch, &registry, &output.vars)?;
    sketch.solve_state = map_status(&status);
    Ok(status)
}

fn seed_circle_radius(registry: &VariableRegistry, values: &mut [f64], sketch: &Sketch) {
    for entity in &sketch.entities {
        let SketchEntity::Circle(circle) = entity else {
            continue;
        };
        let Some(r_id) = registry.get(&format!("{}.radius", circle.base.id.as_str())) else {
            continue;
        };
        if let Coord::Literal(r) = &circle.radius {
            values[r_id.index()] = *r;
        }
    }
}

fn map_status(status: &SolveStatus) -> SolveState {
    match status {
        SolveStatus::Solved { .. } => SolveState::FullyConstrained,
        SolveStatus::UnderConstrained { dof, .. } => SolveState::UnderConstrained { dof: *dof },
        SolveStatus::OverConstrained { redundant, .. } => SolveState::OverConstrained {
            redundant: *redundant,
        },
        SolveStatus::Failed { message, .. } => SolveState::Failed {
            message: message.clone(),
        },
    }
}

fn build_problem(sketch: &Sketch) -> Result<SketchProblem> {
    let mut registry = VariableRegistry::new();
    let mut point_coords = IndexMap::new();
    let lines: IndexMap<String, &LineEntity> = sketch
        .entities
        .iter()
        .filter_map(|e| match e {
            SketchEntity::Line(l) => Some((l.base.id.as_str().to_string(), l)),
            _ => None,
        })
        .collect();

    for entity in &sketch.entities {
        match entity {
            SketchEntity::Point(p) => {
                register_point(&mut registry, &mut point_coords, p)?;
            }
            SketchEntity::Line(l) => {
                register_point_ref(&mut registry, &mut point_coords, sketch, &l.start)?;
                register_point_ref(&mut registry, &mut point_coords, sketch, &l.end)?;
            }
            SketchEntity::Circle(c) => {
                register_point_ref(&mut registry, &mut point_coords, sketch, &c.center)?;
                radius_var(&mut registry, c.base.id.as_str());
            }
            SketchEntity::Arc(a) => {
                register_point_ref(&mut registry, &mut point_coords, sketch, &a.center)?;
                radius_var(&mut registry, a.base.id.as_str());
            }
            SketchEntity::Rectangle(_) => {}
        }
    }

    let mut equations = Vec::new();
    for constraint in &sketch.constraints {
        build_constraint(constraint, &mut equations, &registry, &lines, sketch)?;
    }

    Ok((equations, registry, point_coords))
}

fn register_point(
    registry: &mut VariableRegistry,
    coords: &mut IndexMap<String, (f64, f64)>,
    point: &PointEntity,
) -> Result<()> {
    let id = point.base.id.as_str();
    point_x(registry, id);
    point_y(registry, id);
    let x = coord_literal(&point.x)?;
    let y = coord_literal(&point.y)?;
    coords.insert(id.to_string(), (x, y));
    Ok(())
}

fn register_point_ref(
    registry: &mut VariableRegistry,
    coords: &mut IndexMap<String, (f64, f64)>,
    sketch: &Sketch,
    point_id: &opencad_core::EntityId,
) -> Result<()> {
    let id = point_id.as_str();
    point_x(registry, id);
    point_y(registry, id);
    if !coords.contains_key(id) {
        if let Some(p) = find_point(sketch, id) {
            let x = coord_literal(&p.x)?;
            let y = coord_literal(&p.y)?;
            coords.insert(id.to_string(), (x, y));
        } else {
            coords.insert(id.to_string(), (0.0, 0.0));
        }
    }
    Ok(())
}

fn find_point<'a>(sketch: &'a Sketch, id: &str) -> Option<&'a PointEntity> {
    sketch.entities.iter().find_map(|e| match e {
        SketchEntity::Point(p) if p.base.id.as_str() == id => Some(p),
        _ => None,
    })
}

fn build_constraint(
    constraint: &Constraint,
    equations: &mut Vec<ConstraintResidual>,
    registry: &VariableRegistry,
    lines: &IndexMap<String, &LineEntity>,
    sketch: &Sketch,
) -> Result<()> {
    match constraint {
        Constraint::Coincident { a, b, .. } => {
            let (ax, ay) = entity_ref_xy(registry, lines, sketch, a)?;
            let (bx, by) = entity_ref_xy(registry, lines, sketch, b)?;
            equations.extend(ConstraintResidual::coincident(ax, ay, bx, by));
        }
        Constraint::Horizontal { line, .. } => {
            let (x1, y1, x2, y2) = line_endpoints(registry, lines, line.as_str())?;
            equations.push(ConstraintResidual::Horizontal { x1, y1, x2, y2 });
        }
        Constraint::Vertical { line, .. } => {
            let (x1, y1, x2, y2) = line_endpoints(registry, lines, line.as_str())?;
            equations.push(ConstraintResidual::Vertical { x1, y1, x2, y2 });
        }
        Constraint::Distance { target, expr, .. } => match target {
            DistanceTarget::PointToPoint { a, b } => {
                let (x1, y1) = point_xy(registry, a.as_str())?;
                let (x2, y2) = point_xy(registry, b.as_str())?;
                equations.push(ConstraintResidual::Distance {
                    x1,
                    y1,
                    x2,
                    y2,
                    target: parse_length_expr(expr.as_str())?,
                });
            }
            DistanceTarget::LineLength { line } => {
                let (x1, y1, x2, y2) = line_endpoints(registry, lines, line.as_str())?;
                equations.push(ConstraintResidual::Distance {
                    x1,
                    y1,
                    x2,
                    y2,
                    target: parse_length_expr(expr.as_str())?,
                });
            }
            DistanceTarget::RectangleDimension { rectangle, edge } => {
                let rect = sketch
                    .entities
                    .iter()
                    .find(|e| e.id().as_str() == rectangle.as_str())
                    .ok_or_else(|| OpenCadError::not_found(format!("rectangle '{rectangle}'")))?;
                if let SketchEntity::Rectangle(r) = rect {
                    let target = parse_length_expr(expr.as_str())?;
                    match edge {
                        crate::constraint::RectangleEdge::Width => {
                            let (x1, y1) = point_xy(registry, r.corner_ids[0].as_str())?;
                            let (x2, y2) = point_xy(registry, r.corner_ids[1].as_str())?;
                            equations.push(ConstraintResidual::Distance {
                                x1,
                                y1,
                                x2,
                                y2,
                                target,
                            });
                        }
                        crate::constraint::RectangleEdge::Height => {
                            let (x1, y1) = point_xy(registry, r.corner_ids[0].as_str())?;
                            let (x2, y2) = point_xy(registry, r.corner_ids[3].as_str())?;
                            equations.push(ConstraintResidual::Distance {
                                x1,
                                y1,
                                x2,
                                y2,
                                target,
                            });
                        }
                    }
                }
            }
        },
        Constraint::Radius { target, expr, .. } => {
            let radius = registry
                .get(&format!("{}.radius", target.as_str()))
                .ok_or_else(|| {
                    OpenCadError::not_found(format!("radius for '{}'", target.as_str()))
                })?;
            equations.push(ConstraintResidual::Radius {
                radius,
                target: parse_length_expr(expr.as_str())?,
            });
        }
        Constraint::Diameter { target, expr, .. } => {
            let radius = registry
                .get(&format!("{}.radius", target.as_str()))
                .ok_or_else(|| {
                    OpenCadError::not_found(format!("radius for '{}'", target.as_str()))
                })?;
            equations.push(ConstraintResidual::Radius {
                radius,
                target: parse_length_expr(expr.as_str())? / 2.0,
            });
        }
        Constraint::Equal { .. } => {}
        Constraint::Parallel { .. } | Constraint::Perpendicular { .. } => {}
    }
    Ok(())
}

fn entity_ref_xy(
    registry: &VariableRegistry,
    lines: &IndexMap<String, &LineEntity>,
    _sketch: &Sketch,
    reference: &EntityRef,
) -> Result<(opencad_solver::VarId, opencad_solver::VarId)> {
    match reference {
        EntityRef::Entity(id) => point_xy(registry, id.as_str()),
        EntityRef::PointOnLine { line, end } => {
            let line_ent = lines
                .get(line.as_str())
                .ok_or_else(|| OpenCadError::not_found(format!("line '{}'", line.as_str())))?;
            let point_id = match end {
                LineEnd::Start => line_ent.start.as_str(),
                LineEnd::End => line_ent.end.as_str(),
            };
            point_xy(registry, point_id)
        }
    }
}

fn point_xy(
    registry: &VariableRegistry,
    point_id: &str,
) -> Result<(opencad_solver::VarId, opencad_solver::VarId)> {
    let x = registry
        .get(&format!("{point_id}.x"))
        .ok_or_else(|| OpenCadError::not_found(format!("point x '{point_id}'")))?;
    let y = registry
        .get(&format!("{point_id}.y"))
        .ok_or_else(|| OpenCadError::not_found(format!("point y '{point_id}'")))?;
    Ok((x, y))
}

fn line_endpoints(
    registry: &VariableRegistry,
    lines: &IndexMap<String, &LineEntity>,
    line_id: &str,
) -> Result<(
    opencad_solver::VarId,
    opencad_solver::VarId,
    opencad_solver::VarId,
    opencad_solver::VarId,
)> {
    let line = lines
        .get(line_id)
        .ok_or_else(|| OpenCadError::not_found(format!("line '{line_id}'")))?;
    let (x1, y1) = point_xy(registry, line.start.as_str())?;
    let (x2, y2) = point_xy(registry, line.end.as_str())?;
    Ok((x1, y1, x2, y2))
}

fn coord_literal(coord: &Coord) -> Result<f64> {
    match coord {
        Coord::Literal(v) => Ok(*v),
        Coord::Expr(expr) => parse_length_expr(expr.as_str()),
    }
}

/// Parse simple length literals: `80`, `80 mm`, `0.08 m`.
pub fn parse_length_expr(expr: &str) -> Result<f64> {
    let trimmed = expr.trim();
    if let Some((value, unit)) = trimmed.split_once(char::is_whitespace) {
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| OpenCadError::InvalidExpression(expr.into()))?;
        return Ok(convert_length(value, unit.trim()));
    }
    trimmed
        .parse::<f64>()
        .map_err(|_| OpenCadError::InvalidExpression(expr.into()))
}

fn convert_length(value: f64, unit: &str) -> f64 {
    match unit {
        "m" => value,
        "mm" => value * 0.001,
        "cm" => value * 0.01,
        "in" => value * 0.0254,
        _ => value,
    }
}

fn apply_solution(sketch: &mut Sketch, registry: &VariableRegistry, vars: &VarSet) -> Result<()> {
    for entity in &mut sketch.entities {
        match entity {
            SketchEntity::Point(point) => {
                let id = point.base.id.as_str();
                if let Some(x_id) = registry.get(&format!("{id}.x")) {
                    point.x = Coord::Literal(vars.get(x_id));
                }
                if let Some(y_id) = registry.get(&format!("{id}.y")) {
                    point.y = Coord::Literal(vars.get(y_id));
                }
            }
            SketchEntity::Circle(circle) => {
                if let Some(r_id) = registry.get(&format!("{}.radius", circle.base.id.as_str())) {
                    circle.radius = Coord::Literal(vars.get(r_id));
                }
            }
            SketchEntity::Arc(arc) => {
                if let Some(r_id) = registry.get(&format!("{}.radius", arc.base.id.as_str())) {
                    arc.radius = Coord::Literal(vars.get(r_id));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{Constraint, DistanceTarget};
    use crate::entity::{EntityBase, LineEntity, PointEntity};
    use crate::workplane::Workplane;
    use opencad_core::{ConstraintId, EntityId, Expression, SketchId};

    fn rectangle_sketch() -> Sketch {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:rect").expect("id"),
            "Rectangle",
            Workplane::xy(),
        );

        let corners = ["ent:c0", "ent:c1", "ent:c2", "ent:c3"];
        let edges = ["ent:e0", "ent:e1", "ent:e2", "ent:e3"];

        for (id, x, y) in [
            (corners[0], 0.0, 0.0),
            (corners[1], 70.0, 0.0),
            (corners[2], 70.0, 50.0),
            (corners[3], 0.0, 50.0),
        ] {
            sketch
                .add_entity(SketchEntity::Point(PointEntity {
                    base: EntityBase {
                        id: EntityId::new(id).expect("id"),
                        construction: false,
                    },
                    x: Coord::literal(x),
                    y: Coord::literal(y),
                }))
                .expect("point");
        }

        for (id, start, end) in [
            (edges[0], corners[0], corners[1]),
            (edges[1], corners[1], corners[2]),
            (edges[2], corners[2], corners[3]),
            (edges[3], corners[3], corners[0]),
        ] {
            sketch
                .add_entity(SketchEntity::Line(LineEntity {
                    base: EntityBase {
                        id: EntityId::new(id).expect("id"),
                        construction: false,
                    },
                    start: EntityId::new(start).expect("id"),
                    end: EntityId::new(end).expect("id"),
                }))
                .expect("line");
        }

        sketch
            .add_constraint(Constraint::Horizontal {
                id: ConstraintId::new("con:h0").expect("id"),
                line: EntityId::new(edges[0]).expect("id"),
            })
            .expect("h");
        sketch
            .add_constraint(Constraint::Horizontal {
                id: ConstraintId::new("con:h1").expect("id"),
                line: EntityId::new(edges[2]).expect("id"),
            })
            .expect("h");
        sketch
            .add_constraint(Constraint::Vertical {
                id: ConstraintId::new("con:v0").expect("id"),
                line: EntityId::new(edges[1]).expect("id"),
            })
            .expect("v");
        sketch
            .add_constraint(Constraint::Vertical {
                id: ConstraintId::new("con:v1").expect("id"),
                line: EntityId::new(edges[3]).expect("id"),
            })
            .expect("v");
        sketch
            .add_constraint(Constraint::Distance {
                id: ConstraintId::new("con:w").expect("id"),
                target: DistanceTarget::LineLength {
                    line: EntityId::new(edges[0]).expect("id"),
                },
                expr: Expression::new("80 mm").expect("expr"),
            })
            .expect("w");
        sketch
            .add_constraint(Constraint::Distance {
                id: ConstraintId::new("con:h").expect("id"),
                target: DistanceTarget::LineLength {
                    line: EntityId::new(edges[1]).expect("id"),
                },
                expr: Expression::new("60 mm").expect("expr"),
            })
            .expect("h");
        sketch
    }

    #[test]
    fn solves_rectangle_sketch() {
        let mut sketch = rectangle_sketch();
        let status = solve_sketch(&mut sketch, &SolverOptions::default()).expect("solve");
        assert!(
            status.is_solved() || matches!(status, SolveStatus::UnderConstrained { dof: 0, .. })
        );

        let c1 = sketch
            .find_entity("ent:c1")
            .and_then(|e| match e {
                SketchEntity::Point(p) => Some(p),
                _ => None,
            })
            .expect("c1");
        let x = match c1.x {
            Coord::Literal(v) => v,
            _ => panic!("expected literal"),
        };
        assert!((x - 0.08).abs() < 1e-4);
    }

    #[test]
    fn parses_mm_expression() {
        assert!((parse_length_expr("80 mm").expect("parse") - 0.08).abs() < 1e-9);
    }
}
