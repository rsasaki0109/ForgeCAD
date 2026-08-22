# Examples

Ready-to-use MusubiCAD documents and Agent API requests.

## Documents

| Directory | Template | Features |
|---|---|---|
| `bracket.ocad.d` | `opencad new <path>` | Sketch, extrude, hole (`face_ref`) |
| `bracket_boss_join.ocad.d` | `opencad new <path> boss-join` | + extrude join onto plate |
| `bracket_face_pin.ocad.d` | `opencad new <path> face-pin` | + sketch-on-face pin (`face_ref` workplane) |
| `bracket_edge_fillet.ocad.d` | `opencad new <path> edge-fillet` | + single-edge fillet (`edge_ref`) |
| `bracket_hole_row.ocad.d` | `opencad new <path> hole-row` | + linear cut pattern, `hole_pitch` param |
| `bracket_hole_ring.ocad.d` | `opencad new <path> hole-ring` | + circular cut pattern |
| `bracket_pin_row.ocad.d` | `opencad new <path> pin-row` | + linear union pattern on plate |
| `bracket_pin_ring.ocad.d` | `opencad new <path> pin-ring` | + circular union pattern on plate |
| `bracket_pin_mirror.ocad.d` | `opencad new <path> pin-mirror` | + mirror pattern, `plane_face_ref` |
| `revolve_bushing.ocad.d` | `opencad new <path> revolve-bushing` | Revolve bushing (XY profile, Y axis, 360°) |
| `revolve_sector.ocad.d` | `opencad new <path> revolve-sector` | Half bushing sector (180°) |
| `sketch_constraints_regression.ocad.d` | solver regression fixture | Equal line/circle/arc targets, Parallel/Perpendicular combination, and under/fully/over/contradictory cases |

See [docs/examples/patterns.md](../docs/examples/patterns.md) for a full cut vs union comparison table.

### Sketch regression fixture

`sketch_constraints_regression.ocad.d` is a schema-compatible, expanded
`.ocad.d` example rather than a geometry-kernel golden. It keeps the design
graph inputs and canonical checksums under version control so repeated solves
can assert identical coordinates, DOF, and diagnostics. Validate it with:

```bash
cargo test -p opencad-file --test sketch_regression
```

The fixture intentionally records the serialized golden files and their
checksums. A checksum update is expected only when the canonical fixture
serialization changes; the reason for this golden is to detect accidental
solver or serialization drift, not to hide a schema change.

```bash
cargo run -p opencad-cli -- regen examples/bracket_hole_row.ocad.d
cargo run -p opencad-cli -- inspect examples/bracket.ocad.d
cargo run -p opencad-cli -- patch examples/bracket_hole_row.ocad.d examples/agent/spacing_expr_patch.json
```

## Agent API

See `agent/` for JSON-RPC payloads. Pipe them to `opencad agent` on stdio.
