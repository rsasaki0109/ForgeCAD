# MusubiCAD implementation status

Last reviewed: 2026-08-22

This inventory is evidence-based against the current repository. **Implemented**
means the capability has code and focused tests; **Partial** means the primary
path exists but an explicit contract or integration is still missing; **Stub**
means the public area is reserved but not implemented. The active follow-up IDs
are defined in the [canonical roadmap](roadmap.md).

| Area | Status | Current evidence | Remaining work |
|---|---|---|---|
| Core IDs, units, validation, and document metadata | Implemented | `modules/core/src/id.rs`, `units.rs`, `validation.rs`, `document.rs` | Keep public values unit-explicit; extend only through the roadmap |
| Core transaction primitive | Partial | `modules/core/src/transaction.rs` supports reversible actions and rollback tests | Integrate with document persistence, DesignPatch, and backend history (`MCAD-P3-001`, `MCAD-P3-003`) |
| Design Graph, ParamGraph, Feature Graph, and semantic diff | Implemented | `modules/graph/src/` and graph tests | Preserve deterministic traversal while adding consumers |
| Kernel-neutral geometry and Mock backend | Implemented | `modules/geometry/src/kernel.rs`, `transform.rs`, topology/ref and tessellation modules | Formalize reference guarantees (`MCAD-P5-001`) |
| OCCT backend integration | Implemented | `modules/kernel-occt/src/backend.rs`, cadrum-backed integration tests | Track ABI and topology behavior in Phase 1/5 evidence |
| Sketch data model and basic solver | Partial | `modules/sketch/src/constraint.rs`, `solve.rs`; Coincident, Horizontal, Vertical, distance/radius paths | `Equal`, `Parallel`, and `Perpendicular` currently contribute no residuals (`MCAD-P2-001`, `MCAD-P2-002`) |
| Feature regeneration | Implemented | `modules/feature/src/` covers sketch, extrude, hole, revolve, fillet, chamfer, and patterns with regression fixtures | Add reference-focused cases (`MCAD-P5-002`) |
| `.ocad` and expanded `.ocad.d` format | Implemented | `modules/file/src/`, schemas, canonical JSON, checksums, migration and round-trip tests | Keep schema changes synchronized with migrations and fixtures |
| DesignPatch and Agent API | Partial | `modules/ai/src/patch.rs`, `validation.rs`, `agent_api.rs`; query/diff/dry-run/apply paths exist | Unify with persistent transactions, rollback, and stale preconditions (`MCAD-P3-001`–`MCAD-P3-005`) |
| Assembly model and regeneration | Implemented | `modules/assembly/src/`, ADR-003, `examples/assembly_two_brackets.ocad.d` | Harden path/cycle/interference and golden coverage (`MCAD-P5-004`, `MCAD-P5-005`) |
| Drawing model and SVG export | Implemented | `modules/drawing/src/`, ADR-004, `examples/bracket_front_view.ocad.d` | Split partial hidden-line occlusion and expand golden coverage (`MCAD-P5-003`, `MCAD-P5-005`) |
| Render and desktop preview | Partial | `modules/render`, `modules/desktop`, Tauri shell under `apps/desktop`, and `.github/workflows/desktop.yml` native matrix definition | Verify the native matrix on all four runners, then add install smoke tests, the desktop artifact contract, trust policy, and backend history integration (`MCAD-P1-001`–`MCAD-P1-004`, `MCAD-P3-003`) |
| CLI and release workflow | Partial | `.github/workflows/ci.yml`, `release.yml`; cross-platform CLI archives/checksums remain independent of the desktop workflow | Add Tauri artifact contract and command-parity evidence (`MCAD-P1-002`–`MCAD-P1-003`) |
| Plugin API | Stub | `modules/plugin-api/src/` contains reserved module files without public contracts | Versioned traits, registry, capabilities, integration, and example (`MCAD-P4-001`–`MCAD-P4-004`) |
| Documentation and examples | Partial | Architecture, ADRs, API docs, examples, and this roadmap exist | Keep status/evidence synchronized in feature PRs (`MCAD-P0-002`, `MCAD-P0-003`) |

## Historical plan crosswalk

The older plans use implementation milestone names rather than the active
roadmap IDs:

| Historical document | Current interpretation |
|---|---|
| `assembly-phase3-plan.md` | Phase 3 Assembly milestone is complete; follow-up quality work is `MCAD-P5-004` and `MCAD-P5-005` |
| `drawing-phase4-plan.md` | Phase 4 Drawing milestone is complete; follow-up HLR/golden work is `MCAD-P5-003` and `MCAD-P5-005` |
| `visual-showcase-plan.md` | Showcase baseline is complete; future release and regression work follows `MCAD-P1-003` and `MCAD-P5-005` |

Historical `Task-###` references are retained only where they identify the
original implementation change. New work must use `MCAD-P…` IDs.
