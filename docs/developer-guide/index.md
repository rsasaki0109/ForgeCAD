# Developer Guide

Welcome to MusubiCAD development.

Start with the [canonical development roadmap](../plans/roadmap.md) and its
[implementation status](../plans/implementation-status.md). The roadmap uses
`MCAD-P{phase}-{number}` IDs for new work; historical `Task-###` references are
kept only for traceability.

## Prerequisites

- Rust stable (see `rust-toolchain.toml`)
- `rustfmt` and `clippy` components
- OCCT: auto-installed via `cadrum` on first build (see [occt-install.md](occt-install.md))

## Getting started

```bash
git clone https://github.com/rsasaki0109/MusubiCAD.git
cd MusubiCAD
cargo test --workspace
cargo run -p opencad-cli -- help

# Use committed samples
cargo run -p opencad-cli -- regen examples/bracket.ocad.d
cargo run -p opencad-cli -- new my_carrier.ocad.d bearing-carrier
cargo run -p opencad-cli -- new my_part.ocad.d hole-row
cargo run -p opencad-cli -- new my_holes.ocad.d hole-ring
cargo run -p opencad-cli -- new my_bosses.ocad.d pin-row
cargo run -p opencad-cli -- new my_ring.ocad.d pin-ring
cargo run -p opencad-cli -- new my_mirror.ocad.d pin-mirror
cargo run -p opencad-cli -- animate examples/assembly_two_brackets.ocad.d showcase.gif \
  --frames 36 --fps 12 --orbit-deg 220 --pitch-deg 26
```

## Workspace layout

| Crate | Responsibility |
|---|---|
| `opencad-core` | IDs, units, errors, transactions |
| `opencad-graph` | Design graph, parametric graph, diff |
| `opencad-sketch` | 2D sketch entities and constraints |
| `opencad-solver` | Numeric constraint solver |
| `opencad-geometry` | Kernel-neutral geometry IR |
| `opencad-kernel-occt` | OCCT geometry backend (cadrum) |
| `opencad-feature` | Feature tree and regeneration |
| `opencad-file` | `.ocad` serialization |
| `opencad-ai` | DesignPatch and Agent API |
| `opencad-cli` | Command-line interface |
| `opencad-desktop` | Shared preview + template helpers for desktop UI |
| `opencad-render` | wgpu viewport |
| `opencad-assembly` | Assembly model (historical implementation Phase 3) |
| `opencad-drawing` | Drawing model (historical implementation Phase 4) |
| `opencad-plugin-api` | Plugin extension points |

See [ocad-format.md](../architecture/ocad-format.md) for the native file layout.
See [feature.md](../api/feature.md) for the kernel-neutral feature-modeling API and flagship model.
See [plugins.md](plugins.md) for the linked-plugin authoring workflow and
[plugin-api.md](../api/plugin-api.md) for the complete public contract.
See [releases.md](releases.md) for the multi-platform CLI release contract.
See [desktop-releases.md](desktop-releases.md) for the Tauri installer/archive and checksum contract.
See [ADR-005](../adr/ADR-005-desktop-release-trust.md) for the credential-gated desktop trust policy.

## Development workflow

1. Pick a task from the roadmap (for example, `MCAD-P2-001`).
2. Read `AGENTS.md` for module boundaries.
3. Implement the smallest correct change in the relevant crate.
4. Add unit tests.
5. Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

6. Update docs if the public API or architecture changes.

## README review demo

The README hero is generated from `examples/bearing_carrier.ocad.d` and
`examples/agent/review_bearing_carrier_patch.json`; do not edit its images or reports by hand.
The flagship model chains a base extrusion, joined bearing hub, through bore,
and four-hole circular cut pattern. Regenerate the complete bundle on Linux or macOS with:

```bash
./docs/assets/generate-review-demo.sh
```

On Windows PowerShell, use `./docs/assets/generate-review-demo.ps1`. The Design Review workflow
compares reports byte-for-byte and raster output with a 1% normalized mean-absolute-error tolerance.
The tolerance absorbs GPU rasterization differences while still detecting a material visual change,
so Design Graph, flagship patch, or renderer changes may require updating the bundle in the same PR.

## PR checklist

- [ ] Task ID in PR title (`Task-XXX: …`)
- [ ] Tests added
- [ ] `cargo fmt` and `clippy` pass
- [ ] No OCCT types outside `kernel-occt`
- [ ] Serialized data remains deterministic
- [ ] Docs/ADR updated if needed

## Key invariants

See `AGENTS.md` section 7. The Design Graph is always the source of truth.

## Current roadmap

Completed modeling, Assembly, Drawing, and review-demo milestones are recorded in
the [implementation status](../plans/implementation-status.md). Current planned
work is maintained in the [roadmap](../plans/roadmap.md), including desktop
distribution, Sketch solver completion, transaction/DesignPatch unification,
Plugin API contracts, and CAD reference/output quality.

The desktop-specific setup remains in [desktop-ui.md](desktop-ui.md), with distribution details in
[desktop-releases.md](desktop-releases.md).
