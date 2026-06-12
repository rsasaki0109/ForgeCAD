# Examples

Ready-to-use ForgeCAD documents and Agent API requests.

## Documents

| Directory | Template | Features |
|---|---|---|
| `bracket.ocad.d` | `opencad new <path>` | Sketch, extrude, hole |
| `bracket_hole_row.ocad.d` | `opencad new <path> hole-row` | + linear cut pattern, `hole_pitch` param |

```bash
cargo run -p opencad-cli -- regen examples/bracket_hole_row.ocad.d
cargo run -p opencad-cli -- inspect examples/bracket.ocad.d
cargo run -p opencad-cli -- patch examples/bracket_hole_row.ocad.d examples/agent/spacing_expr_patch.json
```

## Agent API

See `agent/` for JSON-RPC payloads. Pipe them to `opencad agent` on stdio.
