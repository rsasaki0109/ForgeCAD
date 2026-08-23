# Change impact and regeneration trace

MCAD-P6-001 adds two serializable, kernel-neutral evidence contracts. They are
observations of the Design Graph; neither DTO owns document state, B-Rep, mesh,
or a cache.

## `ChangeImpact`

Patch dry-run returns `impact` beside validation and semantic diff:

```json
{
  "version": "opencad.change-impact.v1",
  "no_op": false,
  "changed_inputs": [{ "kind": "parameter", "id": "param:upper_hub_height" }],
  "directly_affected_nodes": ["feature:upper_hub", "feature:shaft_bore"],
  "predicted_dirty_nodes": [
    "feature:upper_hub", "feature:shaft_bore", "feature:counterbore",
    "feature:pcd_fasteners", "feature:radial_ribs",
    "feature:mounting_ears", "feature:mounting_holes"
  ]
}
```

Document-backed dry-run has the complete Feature Graph and sketches, so it
returns dependency-propagated nodes in deterministic topological order.
In-memory Agent calls without that authoring context return the directly
affected feature nodes. A patch that produces no semantic diff sets `no_op` and
returns empty node lists.

## `RegenerationTrace`

Every part regeneration returns:

```json
{
  "executed_nodes": ["feature:sketch_base", "feature:extrude_base"],
  "skipped_nodes": [],
  "solver_call_count": 1,
  "geometry_kernel_call_count": 3,
  "elapsed_time_ms": 2,
  "output_hashes_sha256": { "feature:extrude_base": "<sha256>" },
  "trace_hash_sha256": "<sha256>"
}
```

`geometry_kernel_call_count` is collected by a borrowing adapter around the
kernel-neutral `GeometryKernel`. `output_hashes_sha256` identify logical feature
outputs from canonical feature inputs and ordered upstream hashes; they do not
serialize kernel handles. The trace hash includes execution order, skipped
nodes, call counts, and output hashes. It deliberately excludes
`elapsed_time_ms`, because elapsed time is explicit millisecond evidence but is
not deterministic identity.

`opencad_desktop::apply_patch_and_regenerate_with_trace` returns prediction and
execution evidence together and commits only after regeneration succeeds. A
no-op patch returns `RegenerationTrace::no_op()`, performs zero solver/kernel
calls, and leaves the document byte-for-byte unchanged.
