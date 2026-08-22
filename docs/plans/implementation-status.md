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
| Core transaction primitive | Implemented | `modules/core/src/transaction.rs` tracks successfully applied actions, rolls them back deterministically on apply/commit failure, and covers failure tests; `modules/file/src/history.rs` adds serializable full-document backend history with stale-snapshot validation | Keep public transaction/history contracts deterministic |
| Design Graph, ParamGraph, Feature Graph, and semantic diff | Implemented | `modules/graph/src/` and graph tests | Preserve deterministic traversal while adding consumers |
| Kernel-neutral geometry and Mock backend | Implemented | `modules/geometry/src/kernel.rs`, `transform.rs`, `topology.rs`, `refs.rs`, `topo_sync.rs`, and tessellation modules provide `TopoRefIdentity`, explicit unit-labelled fallback policy, deterministic tie-breaking, and migration-compatible reference resolution; focused geometry tests cover identity, fallback, policy validation, and ordering | Preserve the P5-001 contract while adding feature-specific reference regressions (`MCAD-P5-002`) |
| OCCT backend integration | Implemented | `modules/kernel-occt/src/backend.rs`, cadrum-backed integration tests | Track ABI and topology behavior in Phase 1/5 evidence |
| Sketch data model and basic solver | Implemented | `modules/sketch/src/constraint.rs`, `solve.rs`; Coincident, Horizontal, Vertical, distance/radius, Equal line/circle/arc targets, Parallel, and Perpendicular paths with normalized direction residuals; deterministic rank-based DOF/redundancy, contradiction, and non-convergence diagnostics in `modules/solver/src/diagnostics.rs`; canonical mixed-constraint fixture and repeated-solve coverage in `examples/sketch_constraints_regression.ocad.d` and `modules/file/tests/sketch_regression.rs` | Preserve fixture/checksum stability while extending supported constraints |
| Feature regeneration | Implemented | `modules/feature/src/` covers sketch, extrude, hole, revolve, fillet, chamfer, and patterns; `modules/feature/tests/occt_regen.rs` verifies semantic TopoRef identity and current-face resolution across boolean-hole, fillet, chamfer, and linear-pattern parameter edits | Preserve reference-stability coverage as feature families expand |
| `.ocad` and expanded `.ocad.d` format | Implemented | `modules/file/src/`, schemas, canonical JSON, checksums, migration and round-trip tests | Keep schema changes synchronized with migrations and fixtures |
| DesignPatch and Agent API | Implemented | `modules/ai/src/patch.rs`, `state.rs`, `validation.rs`, `merge.rs`, and `agent_api.rs` provide versioned complete-state revision guards, deterministic dry-run/apply parity, and target-aware rebase conflicts for parameters, features, assembly, and drawing; `modules/file/src/history.rs`, `modules/desktop/src/parameters.rs`, Tauri commands, and Agent history routes provide full-snapshot undo/redo transport; `modules/desktop/tests/command_parity.rs` verifies UI/Tauri/CLI/Agent routing and the desktop parameter transaction against a direct `DesignPatch` apply | Preserve the single command/patch contract as new UI commands are added |
| Assembly model and regeneration | Implemented | `modules/assembly/src/`, ADR-003, `examples/assembly_two_brackets.ocad.d` | Harden path/cycle/interference and golden coverage (`MCAD-P5-004`, `MCAD-P5-005`) |
| Drawing model and SVG export | Implemented | `modules/drawing/src/`, ADR-004, `examples/bracket_front_view.ocad.d` | Split partial hidden-line occlusion and expand golden coverage (`MCAD-P5-003`, `MCAD-P5-005`) |
| Render and desktop preview | Partial | `modules/render`, `modules/desktop`, reusable `run_desktop_smoke` with serializable summary, `modules/desktop/tests/desktop_smoke.rs`, `modules/desktop/tests/command_parity.rs`, Tauri shell under `apps/desktop`, unsigned `.github/workflows/desktop.yml` native matrix plus versioned bundle/checksum contract, and credential-gated `.github/workflows/desktop-signed-release.yml` with ADR-005 trust scope; parameter edit/undo/redo use backend history and keep viewport state separate | Confirm the native matrix, install/open smoke tests, and signed verification on tagged CI; trust policy is implemented but credentialed end-to-end evidence remains (`MCAD-P1-001`–`MCAD-P1-004`) |
| CLI and release workflow | Partial | `.github/workflows/ci.yml`, `release.yml`, desktop headless smoke gate, and separate desktop trust/release workflow; the packaged Linux AppImage runs the same `--smoke-test` contract, while cross-platform CLI archives/checksums remain independent of the desktop workflow | Confirm tagged Tauri artifacts, installer checksums, and real Authenticode/macOS notarization evidence on CI (`MCAD-P1-001`–`MCAD-P1-004`) |
| Plugin API | Implemented | `modules/plugin-api` provides versioned data-only contracts and a deterministic registry; `examples/plugin-example` is a separately buildable feature plugin with manifest/version/golden tests; CLI and Agent invocation use host-owned validation, transactions, history, and persistence; importer/exporter goldens and returned-error document-isolation tests are checked in | Dynamic loading and untrusted-code process isolation are explicitly deferred |
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
