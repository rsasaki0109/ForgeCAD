# Sketch solver API

`opencad-sketch::solve::solve_sketch` maps serialized sketch constraints to
`opencad-solver` residual equations and writes the solved point and radius
values back to the sketch.

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
