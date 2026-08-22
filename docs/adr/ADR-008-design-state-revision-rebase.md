# ADR-008: Complete-state revision preconditions and patch rebase

Status: Accepted  
Date: 2026-08-23

## Context

`DesignPatch` edits can be authored from an immutable snapshot and applied later
through dry-run, the in-memory Agent API, or the file layer. Parameter-only
guards do not detect a concurrent edit to semantic references, assembly
placements, or drawing views. Rebase also needs to distinguish independent
target edits from a changed value that would overwrite another author's work.

## Decision

`PatchPrecondition::RevisionEquals` carries an explicit `algorithm`, `version`,
and `digest`. The current contract is SHA-256 over the compact JSON bytes of a
versioned canonical `DesignState` envelope (`musubicad.design-state.v1`). The
canonical state includes parameters, feature nodes, semantic references, and
optional assembly and drawing models. Object/map keys are ordered
deterministically; source vectors retain their serialized order because it can
be meaningful to regeneration and document identity. Source geometry values remain serialized values and
are never compared with floating-point tolerances for concurrency.

The shared candidate builder validates this precondition against the complete
state before applying any operation. Dry-run, Agent, and file paths therefore
return the same deterministic stale error and leave the source unchanged.

`rebase_patch` applies a patch to the old base to obtain its desired (`ours`)
values, then compares canonical serialized target values for parameters,
features, assembly instances/mates/connectors, and drawing views. Independent
targets rebase successfully. Same-target divergent edits produce one conflict
per target, sorted by conflict kind and stable ID; conflict `ours` is the
patch's desired value. An existing revision precondition is updated to the new
base digest after a successful rebase.

## Consequences

- Revision guards detect changes outside the operation's immediate target.
- Existing `.ocad` and `.ocad.d` schemas do not change; the precondition is a
  transport-level `DesignPatch` field.
- Digest algorithm and canonicalization versions can evolve explicitly without
  silently accepting incompatible revisions.
- Rebase is deterministic and does not require geometry-kernel access, at the
  cost of serializing target values for conflict comparison.
