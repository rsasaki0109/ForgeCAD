# MusubiCAD development roadmap

Status: active

This is the canonical roadmap for work after the initial modeling, assembly, and
drawing milestones. The companion [implementation status](implementation-status.md)
records what is actually present in the repository. Historical `Task-###` numbers
remain in source comments and older planning notes for traceability; they are not
the active schedule.

## Planning contract

Every planned change has one canonical ID in the form `MCAD-P{phase}-{number}`.
The number is zero-padded within a phase and IDs are never reused. A pull request
continues to follow the repository rule `Task-XXX: Short imperative title`; the
canonical `MCAD-P…` ID belongs in the PR description and commit body.

Status values have a precise meaning:

- **Complete** — implementation, required tests, and documentation are present.
- **In progress** — work has started but the definition of done is not met.
- **Planned** — accepted scope with no implementation committed yet.
- **Deferred** — intentionally held until a dependency or product decision changes.

The Design Graph remains the source of truth in every phase. B-Rep and mesh data
are disposable regeneration outputs, model edits use transactions, AI edits use
`DesignPatch`, serialized maps and traversal are deterministic, and all values in
the public model carry explicit units.

## Phase overview

| Phase | Focus | Depends on | Exit outcome |
|---|---|---|---|
| 0 | Planning and repository baseline | — | One source of truth for scope, status, tests, and links |
| 1 | Desktop distribution | 0 | Reproducible Tauri builds and installable artifacts |
| 2 | Sketch solver completion | 0, existing solver | Complete supported constraint set with diagnostics |
| 3 | Transaction and DesignPatch unification | 0, existing file/AI APIs | Atomic backend edits with reliable undo/redo |
| 4 | Plugin API | 0, 3 | Versioned, deterministic extension boundary with an example |
| 5 | CAD reference and output quality | 2, 3, existing assembly/drawing | Stable references and end-to-end regression coverage |

Phase 1 and Phase 2 may proceed in parallel after Phase 0. Phase 3 is the
integration gate for mutating workflows; Phase 4 depends on that gate so plugins
cannot bypass validation. Phase 5 consumes the solver and transaction contracts.

## Phase 0 — Planning and repository baseline

**Objective:** make the implementation state, active task IDs, acceptance tests,
and documentation links agree with the repository.

**Dependencies:** none.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P0-001 | Canonical roadmap and ID scheme | This document; phase dependencies and gates | Complete |
| MCAD-P0-002 | Implementation inventory | [Implementation status](implementation-status.md) with code evidence and follow-up IDs | Complete |
| MCAD-P0-003 | Documentation consistency | README, developer guide, historical plans, and broken local links corrected | Complete |
| MCAD-P0-004 | Verification contract | Required Rust, integration, fixture, and desktop smoke-test matrix documented | Complete |

**Definition of done:** a contributor can choose an active ID, find its module,
dependencies, required tests, and known risks without relying on stale plan text.

**Tests and checks:** documentation-only review; local Markdown link audit;
`cargo fmt --all -- --check` as a non-mutating repository sanity check. No source
or schema behavior changes are introduced by this phase.

**Known risks:** implementation status can drift as code changes. Every feature
PR must update the status table when its exit criteria change.

## Phase 1 — Desktop distribution

**Objective:** turn the existing Tauri shell into a reproducible, downloadable
desktop product while retaining CLI and Agent API parity.

**Dependencies:** Phase 0; existing Tauri shell, `opencad-desktop`, and release
workflow.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P1-001 | Native build matrix | GitHub Actions builds for Windows x64, Linux x64, macOS arm64, and macOS x64 | In progress |
| MCAD-P1-002 | Artifact contract | Versioned archives/installers, SHA-256 checksums, and quick-start instructions | In progress |
| MCAD-P1-003 | Desktop smoke tests | Open sample, preview, parameter edit, regenerate, pick, and export checks | Complete |
| MCAD-P1-004 | Trust and release policy | Explicit code-signing/notarization scope and credential-gated release steps | In progress |

**Definition of done:** the shared `run_desktop_smoke` contract and integration
test open `examples/bracket.ocad.d`, edit a parameter through the backend,
regenerate, pick, export, and prove the source fixture is byte-for-byte
unchanged; the packaged Linux AppImage invokes the same contract headlessly.
The CLI release contract remains green. Tagged native artifact confirmation is
tracked separately by `MCAD-P1-001` and `MCAD-P1-002`.

**Tests:** `python tools/test_desktop_release_policy.py`; workflow validation;
platform build matrix; install/open smoke tests; CLI/desktop command-parity
tests; checksum verification. OCCT tests are marked integration tests where
the kernel is required.

**Known risks:** OCCT and MSVC ABI compatibility, Tauri system dependencies,
wgpu adapter differences, platform signing credentials, and unsigned-artifact
security warnings.

MCAD-P1-004 keeps `desktop.yml` as an unsigned, `contents: read` CI contract
and adds the separate `desktop-signed-release.yml` workflow. The signed path
uses the protected `desktop-release` environment, validates a `v<version>` tag
on `main`, fails closed when Windows or Apple credentials are missing, verifies
Authenticode/codesign/notarization and checksums before publication, and marks
Linux as checksum-only. End-to-end completion still requires a real tagged CI
run with configured credentials; until that evidence exists this item remains
**In progress**.

## Phase 2 — Sketch solver completion

**Objective:** implement the constraint variants already represented in the
serializable sketch model and make solve diagnostics trustworthy.

**Dependencies:** Phase 0; current `opencad-sketch` and `opencad-solver` APIs.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P2-001 | Equal constraint | Line-length and radius residuals, validation, and unit-aware tests | Complete |
| MCAD-P2-002 | Parallel and perpendicular | Direction residuals with degeneracy handling and tolerance tests | Complete |
| MCAD-P2-003 | Solver diagnostics | DOF, redundancy, over-constraint, and non-convergence messages tied to explicit tolerances | Complete |
| MCAD-P2-004 | Sketch regression coverage | Deterministic fixtures and examples for supported constraint combinations | Complete |

**Definition of done:** every serialized constraint that the public API advertises
contributes equations or returns a clear validation error; solved coordinates,
DOF, and diagnostics are deterministic across repeated runs; all comparisons use
documented tolerances.

**Tests:** pure sketch round trips; solver residual/Jacobian tests; under-, fully-,
over-, and contradictory cases; unit conversion tests; and the canonical
`examples/sketch_constraints_regression.ocad.d` fixture exercised by
`modules/file/tests/sketch_regression.rs`. No OCCT dependency is required for
the solver unit suite.

**Known risks:** singular Jacobians, zero-length lines, conflicting constraints,
expression units, and changing solver convergence behavior for existing fixtures.

## Phase 3 — Transaction and DesignPatch unification

**Objective:** make every model mutation atomic and make dry-run, apply, UI, CLI,
and Agent API use the same validated transaction boundary.

**Dependencies:** Phase 0; existing `opencad-core` transaction primitive,
`opencad-ai` DesignPatch, and `.ocad.d` persistence.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P3-001 | Atomic model transaction | Multi-operation apply, rollback on regeneration failure, and unchanged document on error | Complete |
| MCAD-P3-002 | DesignPatch parity | Shared validation path for dry-run and apply, including assembly and drawing operations | Complete |
| MCAD-P3-003 | Backend history | Serializable backend undo/redo snapshots or reversible commands, independent of viewport state | Planned |
| MCAD-P3-004 | Preconditions | Stale-document detection, deterministic conflict errors, and patch rebase coverage | Planned |
| MCAD-P3-005 | Surface parity | UI commands exposed through CLI and Agent API with one command/patch contract | Planned |

**Definition of done:** a failed patch or regeneration leaves the serialized
Design Graph byte-for-byte unchanged; successful operations are undoable and
redoable through the backend; dry-run and apply return the same validation result;
stale preconditions are rejected before mutation.

**Tests:** core transaction tests; AI patch round trips; failure-injection
rollback tests; file checksum/determinism tests; UI/CLI/Agent parity tests;
assembly and drawing patch regressions.

**Known risks:** snapshot size, partial OCCT side effects, concurrent edits,
legacy desktop-local history, and accidental mutation of cached B-Rep data.

## Phase 4 — Plugin API

**Objective:** replace the current placeholder extension crate with a small,
versioned, deterministic API that cannot bypass model validation or module
boundaries.

**Dependencies:** Phase 0 and Phase 3; stable feature registry and transaction
boundary.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P4-001 | Versioned contracts | Feature, importer, exporter traits; serializable manifest and API version | Planned |
| MCAD-P4-002 | Registry and capabilities | Deterministic registration order, capability declarations, and security boundary | Planned |
| MCAD-P4-003 | Product integration | CLI and Agent API discovery/invocation through validated transactions | Planned |
| MCAD-P4-004 | Compatibility evidence | Example plugin, contract tests, failure handling, and developer documentation | Planned |

**Definition of done:** a versioned example plugin can be discovered and invoked
from CLI and Agent API, produces deterministic output, and cannot access document
ownership, raw OCCT types, or unvalidated mutations.

**Tests:** trait/manifest serialization; registry ordering; capability rejection;
API compatibility; importer/exporter golden files; plugin failure isolation and
example smoke tests.

**Known risks:** ABI and versioning policy, untrusted plugin execution, registry
side effects, dependency bloat, and leaking kernel-specific types.

## Phase 5 — CAD reference and output quality

**Objective:** improve the stability and evidence of already shipped topology,
assembly, drawing, mass-property, and rendering workflows.

**Dependencies:** Phase 2 and Phase 3; existing Assembly and Drawing models.

| ID | Scope | Deliverables | Status |
|---|---|---|---|
| MCAD-P5-001 | Semantic TopoRef specification | Reference identity, fingerprint fallback, tolerance policy, and migration guidance | Planned |
| MCAD-P5-002 | Feature reference stability | Boolean, fillet, chamfer, and pattern regeneration regressions with stable references | Planned |
| MCAD-P5-003 | Drawing HLR quality | Split partially occluded edges and preserve deterministic visible/hidden segments | Planned |
| MCAD-P5-004 | Assembly robustness | Cycle/path validation, nested-document errors, interference tolerance, and recovery behavior | Planned |
| MCAD-P5-005 | End-to-end golden suite | Mass, bounding box, topology, assembly, drawing, and review artifacts across representative fixtures | Planned |
| MCAD-P5-006 | Future geometry scope | Requirements and ADR for NURBS editing or new kernel features before implementation | Deferred |

**Definition of done:** semantic references survive the supported feature edits;
drawing output handles partial occlusion deterministically; assembly failures are
localized and non-destructive; golden fixtures cover the engineering evidence
shown by CLI, desktop, and Agent API.

**Tests:** geometry tolerance tests; OCCT integration tests; TopoRef migration and
round trips; assembly/drawing examples; deterministic SVG/mesh/review golden
regressions; mass and bounding-box comparisons with explicit units.

**Known risks:** kernel topology naming limits, floating-point tolerance choices,
mesh-dependent HLR approximations, external component paths, and fixture churn
when OCCT versions change.

## Cross-phase verification matrix

Every implementation PR must select the applicable rows and record the command
result. Documentation-only changes may use the documentation and formatting rows
alone.

| Evidence | Required check |
|---|---|
| Formatting | `cargo fmt --all -- --check` |
| Static analysis | `cargo clippy --workspace --all-targets -- -D warnings` |
| Workspace behavior | `cargo test --workspace` |
| Data/file contracts | Pure model round trips, canonical JSON, schema/migration tests |
| Sketch/geometry | Solver residual tests and OCCT integration tests where required |
| AI mutations | DesignPatch dry-run/apply, precondition, rollback, and semantic diff tests |
| Regression | Committed examples, golden geometry/render/review artifacts |
| Desktop/release | Platform build, install/open, command parity, checksum, and smoke tests |
