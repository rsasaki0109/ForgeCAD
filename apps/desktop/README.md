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
cargo install tauri-cli --version 2.11.4 --locked
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
- PNG preview (sketch overlay included)
- Edit parameters inline (persist + live preview refresh)
- Undo/redo parameter edits (toolbar buttons or Ctrl+Z / Ctrl+Shift+Z) through
  backend-owned opaque full-document history
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
| `musubicad-desktop-v<version>-windows-x86_64` | `windows-2022` | `x86_64-pc-windows-msvc` |
| `musubicad-desktop-v<version>-linux-x86_64` | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| `musubicad-desktop-v<version>-macos-aarch64` | `macos-15` | `aarch64-apple-darwin` |
| `musubicad-desktop-v<version>-macos-x86_64` | `macos-15-intel` | `x86_64-apple-darwin` |

The desktop package has its own checked-in `Cargo.lock` because the Tauri shell
is intentionally excluded from the Rust workspace. CI installs the pinned
Tauri CLI (`2.11.4`), checks the lockfile with `cargo metadata --locked`, and
uses the locked `cadrum` static OCCT 8.0.0 prebuilt for each target. Linux additionally
requires GTK 3, WebKitGTK 4.1, Ayatana AppIndicator, librsvg, and `patchelf`;
Windows requires the Visual Studio 2022 C++ workload (MSVC 14.44 or newer).

The normal workflow bundles and uploads two unsigned, versioned
installer/archive files per platform together with a verified `SHA256SUMS` file.
Windows produces MSI and NSIS installers; Linux produces Debian and AppImage
packages; macOS produces a DMG and a zip archive of the `.app` bundle. See the
[desktop distribution quick start](../../docs/developer-guide/desktop-releases.md)
for exact names, verification, and local packaging commands. A job fails when
a required system tool is missing, dependency resolution is not locked, the
Tauri build returns a non-zero status, a required bundle is absent, checksums
do not verify, the expected executable is absent or empty, or the build rewrites
`Cargo.lock`.

Signed publication is intentionally separate in the
[`Desktop signed release` workflow](../../.github/workflows/desktop-signed-release.yml).
It requires the protected `desktop-release` environment and real Windows/Apple
credentials. Missing credentials fail closed; no signing secret is read by the
normal CI workflow, and fork pull requests never enter the signed workflow.
The tagged workflow remains unverified until a real credentialed CI run succeeds.

The same workflow gates the packaged Linux artifact on the headless backend
smoke and command-parity tests. Run that backend check locally with
`./tools/desktop-smoke.ps1`; it copies the committed bracket example before
editing it, so the repository Design Graph remains unchanged. A packaged
binary can run the identical contract without opening a window:

```text
musubicad-desktop --version
musubicad-desktop --smoke-test <source.ocad.d> <new-work-dir.ocad.d>
```

The smoke command accepts only part documents and refuses an existing work
directory; it prints a serializable JSON summary on success.
