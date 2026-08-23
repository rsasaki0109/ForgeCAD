# ADR-003: Assembly document model

## Status

Accepted

## Context

MusubiCAD Phase 3 adds static assembly modeling: child part references, placed
instances, and compound regeneration without mate solving. The file format and
core ID types already reserved slots (`graph/assemblies.json`, `ComponentId`),
but no assembly data model existed.

Two document-model options were considered:

| Option | Description |
|---|---|
| (A) Separate assembly document type | New top-level type, separate CLI/serialize paths |
| (B) `OcadDocument` + `metadata.kind` | Reuse existing `.ocad` pipeline; part fields stay empty for assemblies |

## Decision

Adopt **option (B)**:

1. Add `DocumentMetadata.kind = part | assembly` (default `part` for backward compatibility).
2. Add `OcadDocument.assembly: Option<AssemblyModel>` (skip when `None`).
3. Serialize assembly data to the existing manifest slot `graph/assemblies.json`.
4. Assembly documents do not own B-Rep geometry; they reference child `.ocad` paths and
   store instance placements as `RigidTransform` values in meters.
5. MVP shipped static placement (M3.1), mate solving (M3.2), and integration
   (connectors, Agent API, multi-instance render, sub-assemblies, patterns in M3.3).

`AssemblyModel` lives in `opencad-assembly`:

```
AssemblyModel
├─ components: Vec<Component>   // child part definitions
├─ instances:  Vec<Instance>    // placed copies
└─ mates:      Vec<Mate>        // empty in MVP
```

Instance placement uses kernel-neutral `opencad_geometry::RigidTransform`
(`translation_m` + 3×3 rotation matrix, no linear-algebra crate in core).

Regeneration resolves each `Component.source_path` relative to the assembly
directory, runs the existing part regen pipeline, applies `transform_body`, and
aggregates instance solids into a compound scene. Failed child regen reports
per-instance errors without corrupting the assembly document.

The robustness contract added in MCAD-P5-004 treats external child references
as untrusted document input. Source paths must be relative and may not traverse
parents. Existing paths are canonicalized beneath the canonical assembly root,
so symbolic-link escape and canonical aliases are rejected. Loaded part and
assembly IDs must equal `Component.source_doc`; Drawing documents are rejected.
The active nested-regeneration stack tracks both `DocumentId` and canonical
path, detects indirect cycles, and is unwound after each attempt so sibling
reuse and retry remain valid.

Interference detection uses a runtime-only
`AssemblyInterferenceTolerance`: meters for broad-phase bounds and cubic meters
for exact common volume. Both thresholds are finite and strictly positive;
defaults are `1e-9 m` and `1e-12 m³`. A volume equal to the threshold is contact,
not interference, and output is deterministically ordered by `InstanceId`.

## Consequences

### Positive

- Minimal change to CLI, Agent API, and expanded-directory I/O
- Part-only documents round-trip unchanged
- Clear path to mate solving (M3.2) without schema breakage

### Negative

- Assembly documents still carry unused part fields (`sketches`, `feature_nodes`)
- Child parts are external files and therefore require explicit containment,
  identity, cycle, and availability diagnostics

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Embedded child documents only | Harder to reuse parts across assemblies; larger diffs |
| Separate `.oasm` format | Duplicates manifest, migration, and CLI plumbing |
