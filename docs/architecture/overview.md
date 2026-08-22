# Architecture Overview

MusubiCAD is an AI-native, open-source parametric 3D CAD engine.

## Core equation

```
CAD = Geometry Kernel + Parametric Program + Design Intent Graph + Validated AI Patch System
```

## Layered architecture

```
Human UI / AI Agents / CLI / Plugins
        │
        ▼
Command Layer (transactions, undo, dry-run, design patch)
        │
        ▼
MusubiCAD Design Graph
        │
        ▼
Regeneration Engine
        │
        ├── Sketch Solver
        └── Geometry Kernel Interface → OCCT backend
                │
                ▼
        Shape Snapshot / B-Rep Cache / Tessellation Cache
                │
                ▼
        Rendering / Export / Mass Properties
                │
                ▼
        .ocad Native Format
```

## Source of truth

The **Design Graph** is authoritative. B-Rep and meshes are disposable caches regenerated from the graph.
Document mutations use staged candidate state: patch operations and part
regeneration run on a clone, and the Design Graph is swapped into the document
only after the complete operation succeeds. A failed operation therefore
leaves the serialized source document byte-for-byte unchanged.
The Agent API, CLI document path, and file-layer patch path share the same
validated candidate builder, including assembly and drawing context checks, so
dry-run and apply expose the same semantic diff and deterministic errors.
`DesignPatch` can additionally carry a versioned SHA-256 revision of the
complete patchable DesignState. The revision is checked before candidate
mutation, while B-Rep and mesh caches remain outside the identity.

Backend history is a separate serializable transport value, not a document
schema field. Each successful file-layer `DesignPatch` records a deterministic
full `OcadDocument` before/after snapshot and a description. Undo and redo
validate that the current source document matches the expected snapshot before
swapping it, and a new record clears the redo branch. The desktop/Tauri/Agent
surfaces pass this value opaquely; only `can_undo` and `can_redo` are exposed as
capabilities. Viewport camera, selection, B-Rep, and mesh caches never enter
history.

## Principles

| Principle | Meaning |
|---|---|
| Design Graph First | Graph before UI |
| AI Editable | Stable IDs, semantic tags, explicit units |
| Kernel Abstracted | OCCT behind a trait; internal IR is kernel-neutral |
| Deterministic Regeneration | Same `.ocad` + kernel version → same result |
| Semantic Topological Naming | Faces referenced by intent, not raw indices |
| Headless First | CLI/API before GUI |
| Local-first Collaboration | `.ocad` zip or git-friendly directory |
| Testable CAD | Volume, mass, constraints, regen are testable |

## Technology stack

- **Core:** Rust
- **Kernel:** OpenCASCADE (initial backend)
- **UI:** Tauri + Web (MVP)
- **Rendering:** wgpu
- **Format:** `.ocad` (JSON-based, git-friendly)

## Further reading

- [ADR-001: Rust-first](../adr/ADR-001-rust-first.md)
- [ADR-007: Serializable backend document history](../adr/ADR-007-backend-document-history.md)
- [ADR-008: DesignState revision preconditions and patch rebase](../adr/ADR-008-design-state-revision-rebase.md)
- [Developer guide](../developer-guide/index.md)
- [AGENTS.md](../../AGENTS.md)
