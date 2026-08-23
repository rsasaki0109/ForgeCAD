# ADR-012: Future geometry admission gate

## Status

Accepted

## Context

MusubiCAD already has a kernel-neutral `NurbsSurface` DTO and an OCCT backend,
but it has no Design Graph feature, commands, regeneration contract, or schema
migration for NURBS editing. Implementing an editor or adding an unrelated
kernel operation directly would risk making cached geometry authoritative,
leaking OCCT types, introducing ambiguous units/tolerances, or bypassing the
transaction and DesignPatch boundary.

MCAD-P5-006 asks for requirements and an ADR before such implementation.

## Decision

Adopt the mandatory admission requirements in
[`future-geometry-requirements.md`](../plans/future-geometry-requirements.md).

Each NURBS-editing or new-kernel-feature proposal must receive a separate
roadmap task and accepted feature ADR before implementation. That ADR must
define the authoring representation in the Design Graph, units and tolerances,
semantic TopoRefs, transaction/DesignPatch commands, kernel-neutral execution,
failure atomicity, serialization/migration impact, dependencies, examples, and
tests.

The existing `NurbsSurface` remains a descriptive kernel-neutral DTO. This ADR
does not add it to `.ocad`, grant it document ownership, or define editing
semantics. NURBS editing and additional kernel features remain implementation-
deferred until a candidate passes the gate.

## Consequences

### Positive

- Preserves the Design Graph as the source of truth.
- Makes units, tolerances, topology identity, and failure behavior reviewable
  before a schema or FFI commitment.
- Keeps Desktop, CLI, and Agent mutation surfaces aligned through DesignPatch.
- Prevents the current DTO shape from becoming an accidental compatibility
  promise for an unspecified editor.

### Negative

- No NURBS editing or new kernel operation ships as part of MCAD-P5-006.
- A future feature requires up-front design and evidence work before coding.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Implement control-point editing directly on `NurbsSurface` | No stable IDs, command semantics, validation policy, schema location, or regeneration contract exists |
| Expose OCCT surface handles to feature/UI code | Violates the kernel boundary and makes derived state authoritative |
| Add operations opportunistically to `GeometryKernel` | Omits Design Graph ownership, transaction parity, migration, and semantic-reference requirements |
| Remove the existing DTO until a feature is selected | Unnecessary churn; retaining it as a non-editable neutral definition does not choose future semantics |
