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
| Kernel-neutral geometry and Mock backend | Implemented | `modules/geometry/src/kernel.rs`, `transform.rs`, `topology.rs`, `refs.rs`, `topo_sync.rs`, and tessellation modules provide `TopoRefIdentity`, explicit unit-labelled fallback policy, deterministic tie-breaking, and migration-compatible reference resolution; focused geometry tests cover identity, fallback, policy validation, and ordering; ADR-012 gates future NURBS editing and kernel operations | Preserve the reference contract; require the future-geometry admission gate before expanding editable geometry |
| OCCT backend integration | Implemented | `modules/kernel-occt/src/backend.rs`, cadrum-backed integration tests | Track ABI and topology behavior in Phase 1/5 evidence |
| Sketch data model and basic solver | Implemented | `modules/sketch/src/constraint.rs`, `solve.rs`; Coincident, Horizontal, Vertical, distance/radius, Equal line/circle/arc targets, Parallel, and Perpendicular paths with normalized direction residuals; deterministic rank-based DOF/redundancy, contradiction, and non-convergence diagnostics in `modules/solver/src/diagnostics.rs`; canonical mixed-constraint fixture and repeated-solve coverage in `examples/sketch_constraints_regression.ocad.d` and `modules/file/tests/sketch_regression.rs` | Preserve fixture/checksum stability while extending supported constraints |
| Feature regeneration | Implemented | `modules/feature/src/` covers sketch, extrude, hole, revolve, fillet, chamfer, and patterns; `RegenerationTrace` records deterministic order, solver/kernel calls, explicit millisecond timing, and logical output hashes; `robot_joint_actuator_housing()` composes 22 deterministic nodes | MCAD-P6-002 will use this trace as the oracle for incremental execution |
| `.ocad` and expanded `.ocad.d` format | Implemented | `modules/file/src/`, schemas, canonical JSON, checksums, migration and round-trip tests | Keep schema changes synchronized with migrations and fixtures |
| DesignPatch and Agent API | Implemented | `ChangeImpact` is returned by dry-run/apply; document-backed prediction maps changed parameter/feature/reference input to an exact topological dirty suffix. Revision guards, atomic candidate validation, history, rebase, and CLI/Desktop/Agent transport remain shared | Extend the same evidence contract with reference provenance and assertions in MCAD-P6-003/004 |
| Assembly model and regeneration | Implemented | `modules/assembly/src/`, ADR-003, `examples/assembly_two_brackets.ocad.d`; canonical root containment, document kind/identity checks, indirect-cycle detection, localized retryable failures, deterministic explicit-unit interference tolerances, and the OCCT assembly result in `fixtures/golden/mcad_p5_005_end_to_end.json` | Preserve the cross-artifact manifest when assembly evidence changes |
| Drawing model and SVG export | Implemented | `modules/drawing/src/`, ADR-004, and `examples/bracket_front_view.ocad.d`; hidden-line extraction subdivides edges at projected occluder boundaries and depth crossings with explicit tolerances and deterministic ordering; `modules/drawing/tests/golden/partial-occlusion.svg` and the P5-005 cross-artifact test pin visible/hidden SVG output | Preserve the SVG and manifest goldens when HLR changes |
| Render and desktop preview | Implemented | `modules/render`, `modules/desktop`, reusable `run_desktop_smoke` with serializable summary, `modules/desktop/tests/desktop_smoke.rs`, `modules/desktop/tests/command_parity.rs`, Tauri shell under `apps/desktop`, unsigned `.github/workflows/desktop.yml` native matrix plus versioned bundle/checksum contract, and credential-gated `.github/workflows/desktop-signed-release.yml` with ADR-005 trust scope; local Windows/Linux checks and hosted runs `32612751044` and tagged `32616853187` prove all four native builds, versioned artifacts, independent checksums, and packaged Linux smoke as recorded in `docs/plans/desktop-release-evidence.md`; parameter edit/undo/redo use backend history and keep viewport state separate; CLI Feature-build animation renders non-tool body milestones against one final-shape camera for the 22-node robot-joint flagship | Credentialed Authenticode/macOS publication is explicitly deferred; preserve the fail-closed path |
| CLI and release workflow | Implemented | `.github/workflows/ci.yml`, `release.yml`, desktop headless smoke gate, and separate desktop trust/release workflow; tagged run `32616853192` published four CLI archives plus checksums, and the downloaded Windows archive passed an independent version/OCCT smoke; tagged Desktop run `32616853187` published all four workflow artifacts with verified installer checksums | Credentialed desktop publication is explicitly deferred; unsigned verified artifacts remain supported |
| Plugin API | Implemented | `modules/plugin-api` provides versioned data-only contracts and a deterministic registry; `examples/plugin-example` is a separately buildable feature plugin with manifest/version/golden tests; CLI and Agent invocation use host-owned validation, transactions, history, and persistence; importer/exporter goldens and returned-error document-isolation tests are checked in | Dynamic loading and untrusted-code process isolation are explicitly deferred |
| Documentation and examples | Implemented | Architecture, ADRs, API docs, examples, the P5-005 evidence guide, and this roadmap exist | Keep status/evidence synchronized in future feature changes |

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
