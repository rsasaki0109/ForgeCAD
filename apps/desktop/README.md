# MusubiCAD Desktop (Tauri)

Minimal desktop shell for previewing `.ocad.d` documents with OCCT regeneration and wgpu rendering.

## Prerequisites (Linux)

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Install the Tauri CLI once:

```bash
cargo install tauri-cli --version "^2.0.0"
```

## Run (dev)

From the repository root:

```bash
cd apps/desktop/src-tauri
cargo tauri dev
```

The app loads `examples/bracket.ocad.d` automatically when launched from the workspace.

## Features (MVP)

- Open `.ocad.d` directory
- Regenerate + PNG preview (sketch overlay included)
- Edit parameters inline (persist + live preview refresh)
- Undo/redo parameter edits (toolbar buttons or Ctrl+Z / Ctrl+Shift+Z)
- Click preview to pick faces/sketch lines (topo ref + feature inference)
- Picking geometry highlights related parameters in the panel
- Selected geometry is highlighted on the preview image (face-group boundary edges; cylindrical faces use ring outlines)
- Open interactive wgpu 3D viewport (separate window; picks sync to Selection panel and preview highlight)
- Create built-in sample templates
- Document inspect panel (features, sketches, bounds)

## Architecture

| Layer | Crate / path |
|---|---|
| Preview API | `modules/desktop` (`opencad-desktop`) |
| Desktop shell | `apps/desktop/src-tauri` |
| Web UI | `apps/desktop/ui` |

The shared `opencad-desktop` crate is built in the workspace CI. The Tauri shell has a native
build-matrix workflow and can also be built locally with the commands above.

## Native build matrix

The [`Desktop` workflow](../../.github/workflows/desktop.yml) builds the Tauri
shell on every supported native target:

| Artifact | GitHub runner | Rust target |
|---|---|---|
| `musubicad-desktop-windows-x86_64` | `windows-2022` | `x86_64-pc-windows-msvc` |
| `musubicad-desktop-linux-x86_64` | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| `musubicad-desktop-macos-aarch64` | `macos-15` | `aarch64-apple-darwin` |
| `musubicad-desktop-macos-x86_64` | `macos-15-intel` | `x86_64-apple-darwin` |

The desktop package has its own checked-in `Cargo.lock` because the Tauri shell
is intentionally excluded from the Rust workspace. CI installs the pinned
Tauri CLI (`2.11.4`), checks the lockfile with `cargo metadata --locked`, and
uses the locked `cadrum` static OCCT 8.0.0 prebuilt for each target. Linux additionally
requires GTK 3, WebKitGTK 4.1, Ayatana AppIndicator, librsvg, and `patchelf`;
Windows requires the Visual Studio 2022 C++ workload (MSVC 14.44 or newer).

The workflow currently uploads the unsigned native executable from each build.
`bundle.active` is intentionally still `false`, so installers, checksums,
code-signing, and notarization are separate release work (`MCAD-P1-002` and
`MCAD-P1-004`). A job fails when a required system tool is missing, dependency
resolution is not locked, the Tauri build returns a non-zero status, the
expected executable is absent or empty, or the build rewrites `Cargo.lock`.
