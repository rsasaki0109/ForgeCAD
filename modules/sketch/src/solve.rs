use indexmap::IndexMap;

use opencad_core::{OpenCadError, Result};
use opencad_solver::{
    point_x, point_y, radius_var, solve_with_diagnostics, ConstraintResidual, LengthTerm,
    SolveStatus, SolverOptions, VarSet, VariableRegistry,
};

use crate::constraint::{Constraint, DistanceTarget, EntityRef, EqualTarget, LineEnd};
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
    seed_radius_values(&registry, &mut values, sketch);

    let vars = VarSet::new(values);
    let (output, status) = solve_with_diagnostics(&equations, vars, options);
    apply_solution(sketch, &registry, &output.vars)?;
    sketch.solve_state = map_status(&status);
    Ok(status)
}

fn seed_radius_values(registry: &VariableRegistry, values: &mut [f64], sketch: &Sketch) {
    for entity in &sketch.entities {
        let (id, radius) = match entity {
            SketchEntity::Circle(circle) => (&circle.base.id, &circle.radius),
            SketchEntity::Arc(arc) => (&arc.base.id, &arc.radius),
            _ => continue,
        };
        let Some(r_id) = registry.get(&format!("{}.radius", id.as_str())) else {
            continue;
        };
        // Radius expressions may be simple unit-bearing literals (for example,
        // `80 mm`).  Symbolic expressions are evaluated by a higher-level
        // parameter context and retain the existing zero seed here.
        if let Ok(r) = coord_literal(radius) {
            values[r_id.index()] = r;
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
        Constraint::Equal { a, b, .. } => {
            let a = equal_target_length(a, registry, lines, sketch)?;
            let b = equal_target_length(b, registry, lines, sketch)?;
            equations.push(ConstraintResidual::EqualLength { a, b });
        }
        Constraint::Parallel { .. } | Constraint::Perpendicular { .. } => {}
    }
    Ok(())
}

/// Resolve an equal target to a length-valued solver term.
///
/// Both line lengths and circle/arc radii are lengths in internal SI units,
/// so mixed equal constraints are intentionally supported.  The target kind
/// is still checked against the referenced sketch entity to avoid silently
/// treating (for example) a circle as a line or a point as a radius.
fn equal_target_length(
    target: &EqualTarget,
    registry: &VariableRegistry,
    lines: &IndexMap<String, &LineEntity>,
    sketch: &Sketch,
) -> Result<LengthTerm> {
    match target {
        EqualTarget::LineLength(line_id) => {
            let entity = sketch.find_entity(line_id.as_str()).ok_or_else(|| {
                OpenCadError::not_found(format!("equal line-length target '{}'", line_id.as_str()))
            })?;
            if !matches!(entity, SketchEntity::Line(_)) {
                return Err(OpenCadError::validation(format!(
                    "equal line-length target '{}' must reference a line",
                    line_id.as_str()
                )));
            }
            let (x1, y1, x2, y2) = line_endpoints(registry, lines, line_id.as_str())?;
            Ok(LengthTerm::Segment { x1, y1, x2, y2 })
        }
        EqualTarget::Radius(entity_id) => {
            let entity = sketch.find_entity(entity_id.as_str()).ok_or_else(|| {
                OpenCadError::not_found(format!("equal radius target '{}'", entity_id.as_str()))
            })?;
            if !matches!(entity, SketchEntity::Circle(_) | SketchEntity::Arc(_)) {
                return Err(OpenCadError::validation(format!(
                    "equal radius target '{}' must reference a circle or arc",
                    entity_id.as_str()
                )));
            }
            let radius = registry
                .get(&format!("{}.radius", entity_id.as_str()))
                .ok_or_else(|| {
                    OpenCadError::not_found(format!(
                        "radius variable for equal target '{}'",
                        entity_id.as_str()
                    ))
                })?;
            Ok(LengthTerm::Scalar { value: radius })
        }
    }
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
    use crate::constraint::{Constraint, DistanceTarget, EqualTarget};
    use crate::entity::{ArcEntity, CircleEntity, EntityBase, LineEntity, PointEntity};
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

    #[test]
    fn equal_line_lengths_use_si_units() {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:equal_lines").expect("id"),
            "Equal lines",
            Workplane::xy(),
        );
        for (id, x, y) in [
            ("ent:p0", 0.0, 0.0),
            ("ent:p1", 0.08, 0.0),
            ("ent:p2", 0.0, 0.02),
            ("ent:p3", 0.04, 0.02),
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
            ("ent:line_a", "ent:p0", "ent:p1"),
            ("ent:line_b", "ent:p2", "ent:p3"),
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
            .add_constraint(Constraint::Equal {
                id: ConstraintId::new("con:equal_lines").expect("id"),
                a: EqualTarget::LineLength(EntityId::new("ent:line_a").expect("id")),
                b: EqualTarget::LineLength(EntityId::new("ent:line_b").expect("id")),
            })
            .expect("equal");
        sketch
            .add_constraint(Constraint::Distance {
                id: ConstraintId::new("con:line_a_length").expect("id"),
                target: DistanceTarget::LineLength {
                    line: EntityId::new("ent:line_a").expect("id"),
                },
                expr: Expression::new("80 mm").expect("expr"),
            })
            .expect("distance");

        solve_sketch(&mut sketch, &SolverOptions::default()).expect("solve");
        let point = |id: &str| {
            let entity = sketch.find_entity(id).expect("point entity");
            let SketchEntity::Point(point) = entity else {
                panic!("expected point")
            };
            let Coord::Literal(x) = point.x else {
                panic!("expected literal x")
            };
            let Coord::Literal(y) = point.y else {
                panic!("expected literal y")
            };
            [x, y]
        };
        let a0 = point("ent:p0");
        let a1 = point("ent:p1");
        let b0 = point("ent:p2");
        let b1 = point("ent:p3");
        let length =
            |a: [f64; 2], b: [f64; 2]| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        assert!((length(a0, a1) - length(b0, b1)).abs() < 1e-8);
        assert!((length(a0, a1) - 0.08).abs() < 1e-8);
    }

    #[test]
    fn equal_circle_and_arc_radii_use_unit_bearing_values() {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:equal_radii").expect("id"),
            "Equal radii",
            Workplane::xy(),
        );
        for id in ["ent:center_circle", "ent:center_arc"] {
            sketch
                .add_entity(SketchEntity::Point(PointEntity {
                    base: EntityBase {
                        id: EntityId::new(id).expect("id"),
                        construction: false,
                    },
                    x: Coord::literal(0.0),
                    y: Coord::literal(0.0),
                }))
                .expect("center");
        }
        sketch
            .add_entity(SketchEntity::Circle(CircleEntity {
                base: EntityBase {
                    id: EntityId::new("ent:circle").expect("id"),
                    construction: false,
                },
                center: EntityId::new("ent:center_circle").expect("id"),
                radius: Coord::expr("80 mm").expect("radius"),
            }))
            .expect("circle");
        sketch
            .add_entity(SketchEntity::Arc(ArcEntity {
                base: EntityBase {
                    id: EntityId::new("ent:arc").expect("id"),
                    construction: false,
                },
                center: EntityId::new("ent:center_arc").expect("id"),
                radius: Coord::literal(0.04),
                start_angle: Coord::literal(0.0),
                end_angle: Coord::literal(1.0),
            }))
            .expect("arc");
        sketch
            .add_constraint(Constraint::Radius {
                id: ConstraintId::new("con:circle_radius").expect("id"),
                target: EntityId::new("ent:circle").expect("id"),
                expr: Expression::new("80 mm").expect("expr"),
            })
            .expect("radius");
        sketch
            .add_constraint(Constraint::Equal {
                id: ConstraintId::new("con:equal_radii").expect("id"),
                a: EqualTarget::Radius(EntityId::new("ent:circle").expect("id")),
                b: EqualTarget::Radius(EntityId::new("ent:arc").expect("id")),
            })
            .expect("equal");

        solve_sketch(&mut sketch, &SolverOptions::default()).expect("solve");
        let radius = |id: &str| {
            let entity = sketch.find_entity(id).expect("radius entity");
            let radius = match entity {
                SketchEntity::Circle(circle) => &circle.radius,
                SketchEntity::Arc(arc) => &arc.radius,
                _ => panic!("expected circular entity"),
            };
            let Coord::Literal(radius) = radius else {
                panic!("expected literal radius")
            };
            *radius
        };
        assert!((radius("ent:circle") - 0.08).abs() < 1e-8);
        assert!((radius("ent:arc") - 0.08).abs() < 1e-8);
    }

    #[test]
    fn seeds_arc_radius_from_existing_si_value() {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:arc_seed").expect("id"),
            "Arc seed",
            Workplane::xy(),
        );
        sketch
            .add_entity(SketchEntity::Point(PointEntity {
                base: EntityBase {
                    id: EntityId::new("ent:center").expect("id"),
                    construction: false,
                },
                x: Coord::literal(0.0),
                y: Coord::literal(0.0),
            }))
            .expect("center");
        sketch
            .add_entity(SketchEntity::Arc(ArcEntity {
                base: EntityBase {
                    id: EntityId::new("ent:arc").expect("id"),
                    construction: false,
                },
                center: EntityId::new("ent:center").expect("id"),
                radius: Coord::expr("40 mm").expect("radius"),
                start_angle: Coord::literal(0.0),
                end_angle: Coord::literal(1.0),
            }))
            .expect("arc");

        let (_, registry, _) = build_problem(&sketch).expect("problem");
        let mut values = registry.initial_values();
        seed_radius_values(&registry, &mut values, &sketch);
        let radius = registry.get("ent:arc.radius").expect("radius variable");
        assert!((values[radius.index()] - 0.04).abs() < 1e-12);
    }

    #[test]
    fn equal_target_kind_validation_is_explicit() {
        let mut sketch = Sketch::new(
            SketchId::new("sketch:equal_validation").expect("id"),
            "Equal validation",
            Workplane::xy(),
        );
        sketch
            .add_entity(SketchEntity::Point(PointEntity {
                base: EntityBase {
                    id: EntityId::new("ent:point").expect("id"),
                    construction: false,
                },
                x: Coord::literal(0.0),
                y: Coord::literal(0.0),
            }))
            .expect("point");
        sketch
            .add_constraint(Constraint::Equal {
                id: ConstraintId::new("con:invalid_line_target").expect("id"),
                a: EqualTarget::LineLength(EntityId::new("ent:point").expect("id")),
                b: EqualTarget::LineLength(EntityId::new("ent:point").expect("id")),
            })
            .expect("constraint");

        let error = solve_sketch(&mut sketch, &SolverOptions::default()).expect_err("validation");
        assert!(error
            .to_string()
            .contains("equal line-length target 'ent:point' must reference a line"));

        sketch.constraints.clear();
        sketch
            .add_constraint(Constraint::Equal {
                id: ConstraintId::new("con:missing_radius_target").expect("id"),
                a: EqualTarget::Radius(EntityId::new("ent:missing").expect("id")),
                b: EqualTarget::Radius(EntityId::new("ent:missing").expect("id")),
            })
            .expect("constraint");
        let error = solve_sketch(&mut sketch, &SolverOptions::default()).expect_err("not found");
        assert!(error
            .to_string()
            .contains("equal radius target 'ent:missing'"));
    }
}
