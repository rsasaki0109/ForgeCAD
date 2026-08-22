# MusubiCAD implementation status

Last reviewed: 2026-08-23

This inventory is evidence-based against the current repository. **Implemented**
means the capability has code and focused tests; **Partial** means the primary
path exists but an explicit contract or integration is still missing; **Stub**
means the public area is reserved but not implemented. The active follow-up IDs
are defined in the [canonical roadmap](roadmap.md).

| Area | Status | Current evidence | Remaining work |
|---|---|---|---|
| Core IDs, units, validation, and document metadata | Implemented | `modules/core/src/id.rs`, `units.rs`, `validation.rs`, `document.rs` | Keep public values unit-explicit; extend only through the roadmap |
| Core transaction primitive | Implemented | `modules/core/src/transaction.rs` tracks successfully applied actions, rolls them back deterministically on apply/commit failure, and covers failure tests | Integrate serializable backend history and undo/redo (`MCAD-P3-003`) |
| Design Graph, ParamGraph, Feature Graph, and semantic diff | Implemented | `modules/graph/src/` and graph tests | Preserve deterministic traversal while adding consumers |
| Kernel-neutral geometry and Mock backend | Implemented | `modules/geometry/src/kernel.rs`, `transform.rs`, topology/ref and tessellation modules | Formalize reference guarantees (`MCAD-P5-001`) |
| OCCT backend integration | Implemented | `modules/kernel-occt/src/backend.rs`, cadrum-backed integration tests | Track ABI and topology behavior in Phase 1/5 evidence |
| Sketch data model and basic solver | Implemented | `modules/sketch/src/constraint.rs`, `solve.rs`; Coincident, Horizontal, Vertical, distance/radius, Equal line/circle/arc targets, Parallel, and Perpendicular paths with normalized direction residuals; deterministic rank-based DOF/redundancy, contradiction, and non-convergence diagnostics in `modules/solver/src/diagnostics.rs`; canonical mixed-constraint fixture and repeated-solve coverage in `examples/sketch_constraints_regression.ocad.d` and `modules/file/tests/sketch_regression.rs` | Preserve fixture/checksum stability while extending supported constraints |
| Feature regeneration | Implemented | `modules/feature/src/` covers sketch, extrude, hole, revolve, fillet, chamfer, and patterns with regression fixtures | Add reference-focused cases (`MCAD-P5-002`) |
| `.ocad` and expanded `.ocad.d` format | Implemented | `modules/file/src/`, schemas, canonical JSON, checksums, migration and round-trip tests | Keep schema changes synchronized with migrations and fixtures |
| DesignPatch and Agent API | Partial | `modules/ai/src/patch.rs`, `validation.rs`, `agent_api.rs`; top-level multi-group patch apply is atomic, with query/diff/dry-run/apply paths | Unify remaining surfaces, assembly/drawing parity, and stale preconditions (`MCAD-P3-002`–`MCAD-P3-005`) |
| Assembly model and regeneration | Implemented | `modules/assembly/src/`, ADR-003, `examples/assembly_two_brackets.ocad.d` | Harden path/cycle/interference and golden coverage (`MCAD-P5-004`, `MCAD-P5-005`) |
| Drawing model and SVG export | Implemented | `modules/drawing/src/`, ADR-004, `examples/bracket_front_view.ocad.d` | Split partial hidden-line occlusion and expand golden coverage (`MCAD-P5-003`, `MCAD-P5-005`) |
| Render and desktop preview | Partial | `modules/render`, `modules/desktop`, reusable `run_desktop_smoke` with serializable summary, `modules/desktop/tests/desktop_smoke.rs`, `modules/desktop/tests/command_parity.rs`, Tauri shell under `apps/desktop`, unsigned `.github/workflows/desktop.yml` native matrix plus versioned bundle/checksum contract, and credential-gated `.github/workflows/desktop-signed-release.yml` with ADR-005 trust scope | Confirm the native matrix, install/open smoke tests, and signed verification on tagged CI; trust policy is implemented but credentialed end-to-end evidence and backend history remain (`MCAD-P1-001`–`MCAD-P1-004`, `MCAD-P3-003`) |
| CLI and release workflow | Partial | `.github/workflows/ci.yml`, `release.yml`, desktop headless smoke gate, and separate desktop trust/release workflow; the packaged Linux AppImage runs the same `--smoke-test` contract, while cross-platform CLI archives/checksums remain independent of the desktop workflow | Confirm tagged Tauri artifacts, installer checksums, and real Authenticode/macOS notarization evidence on CI (`MCAD-P1-001`–`MCAD-P1-004`) |
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
