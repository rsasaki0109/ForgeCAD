# Sketch solver API

`opencad-sketch::solve::solve_sketch` maps serialized sketch constraints to
`opencad-solver` residual equations and writes the solved point and radius
values back to the sketch.

## Direction constraints

`Constraint::Parallel` and `Constraint::Perpendicular` reference two line
entities by `line_a` and `line_b`.  The numeric engine uses normalized
direction residuals: the 2D cross product divided by both line lengths for
parallel, and the dot product divided by both line lengths for perpendicular.
Both residuals are dimensionless (`sin(angle)` and `cos(angle)`), so their
magnitude is independent of the lines' absolute scale.

Direction constraints validate both referenced lines before solving.  A line
whose initial length is non-finite or at/below
`opencad_solver::DIRECTION_DEGENERACY_TOLERANCE_M` (`1e-12 m`) returns a
validation error rather than constructing an undefined direction.  The
same tolerance is checked again before solved coordinates are written back, so
a numeric iteration cannot satisfy the relation by collapsing a line. The
serialized sketch constraint shape is unchanged; only the transient solver
residual adds direction equations.

## Equal targets

`EqualTarget` keeps its stable untagged public representation:

- `LineLength(entity_id)` references a line entity's Euclidean length.
- `Radius(entity_id)` references a circle or arc entity's radius.

Both targets are lengths and are compared in internal SI meters. Therefore a
line-length target and a radius target may be mixed in one `Equal` constraint.
The solver rejects a target whose entity kind does not match its variant, and
reports a missing entity as an error before solving.

The numeric engine represents these terms with
`ConstraintResidual::EqualLength` and `LengthTerm`; the latter is an internal
solver API and is not serialized into `.ocad` documents.
