# ADR-011: Semantic TopoRef identity and fingerprint fallback

Status: Accepted  
Date: 2026-08-23

## Context

Kernel face and edge numbers are useful regeneration hints but are not stable
identities: boolean, fillet, chamfer, and tessellation operations can renumber
them. The existing serialized `TopoRef` already carries semantic producer/role
data and optional geometric hints, but the matching thresholds in
`topo_sync.rs` were implicit constants and equal-score selection depended on
discovery order.

## Decision

`TopoRef.ref_id` is the persisted primary identity key.
`TopoRef::identity()` returns a serializable `TopoRefIdentity` containing that
key plus `kind`, `created_by`, `role`, and `intent` metadata used to verify the
intended semantic meaning. Kernel IDs, normals, and geometric fingerprints
remain excluded from identity and are used only for regeneration and fallback.

Fingerprint resolution keeps the existing direct APIs and adds explicit-policy
variants. `TopoRefTolerancePolicy` names every geometric threshold with units:
centroid/midpoint distances are strictly positive meters, direction thresholds
are absolute dimensionless dot products in `[0, 1]`, and vector epsilon is a
strictly positive dimensionless value. The default preserves the prior edge
midpoint `2 mm`, direction `0.99 dot`, and `1e-9` epsilon behavior and applies
the same `2 mm` budget to the newly specified face-centroid fallback. Invalid
policies are rejected before resolution. Equal candidate scores use the
smallest kernel ID as a deterministic tie-break.

The legacy fingerprint shape remains unchanged: `area_range` is square meters;
face `bbox_hint` entries are centroid bounds in meters; edge `bbox_hint` stores
`[midpoint_m, unit_tangent]`. `surface_type` and `area_range` remain reserved
hints in P5-001 and do not participate in scoring. A cleaner shape requires a
future schema version and migration rather than silently reinterpreting files.

The policy is runtime-only. No TopoRef JSON fields, `.ocad` schema, or migration
version changes are introduced. Legacy files are migrated by retaining semantic
identity and refreshing stale kernel/fingerprint hints through the existing
history and sync paths.

## Consequences

- References can be reasoned about and compared without kernel access.
- Fallback behavior is reviewable, unit-explicit, and deterministic.
- Existing documents remain byte/schema compatible until a real sync changes a
  fingerprint hint.
- The default tolerances are intentionally conservative and should be chosen
  explicitly by workflows with stricter engineering budgets.
- This ADR specifies reference identity and matching only; feature-specific
  regeneration regressions remain MCAD-P5-002.
