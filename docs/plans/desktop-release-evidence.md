# Desktop release evidence

This log separates locally reproducible release evidence from credentialed or
multi-platform GitHub evidence. It is not a substitute for a tagged release.

## Windows x86_64 local verification — 2026-08-23

Source revision: `41fab53` (`main`, 24 commits ahead of `origin/main` at the
time of verification).

Toolchain:

- `tauri-cli 2.11.4`
- Rust target `x86_64-pc-windows-msvc`
- Tauri package version `0.1.0`

The following native build completed successfully:

```powershell
cargo tauri build --ci --bundles "msi,nsis" --target x86_64-pc-windows-msvc
```

It produced both required installer types. The repository packaging contract
then renamed them and generated a verified `SHA256SUMS` file:

| Artifact | Size (bytes) | SHA-256 for this run |
|---|---:|---|
| `musubicad-v0.1.0-windows-x86_64-nsis.exe` | 11,077,368 | `a7fafa01afd6b19fb3437d1264ced754048a052d5be402757d1d54941d965a83` |
| `musubicad-v0.1.0-windows-x86_64.msi` | 16,150,528 | `edb80096a61be11f4ff0ce4189c5359c9a6b7f9c0ad83cc5e47130098e785e7e` |

Installer containers may include build metadata, so these hashes identify this
verification run; the release contract requires internal checksum agreement,
not identical installer hashes across independent builds.

The release executable completed `--smoke-test` with exit code 0 and wrote the
expected 12-file working `.ocad.d` document. To test the installable payload,
the MSI was also administratively extracted to a new temporary directory with
`msiexec /a`; the executable from `PFiles/MusubiCAD` completed the same smoke
contract with exit code 0 and 12 output files. The source example was not
modified.

`Get-AuthenticodeSignature` reported `NotSigned` and no signer certificate for
both installers. This is expected for the unsigned local/CI workflow and is
positive evidence that the trust policy does not imply signing without the
protected release credentials.

## Linux x86_64 local verification — 2026-08-23

Source revision: `d49493a` (`main`, 25 commits ahead of `origin/main` at the
time of verification).

Ubuntu 22.04 under WSL used the same native dependency set as the GitHub
workflow, Rust 1.98.0, `tauri-cli 2.11.4`, and target
`x86_64-unknown-linux-gnu`. The following native build completed successfully:

```bash
cargo tauri build --ci --bundles deb,appimage \
  --target x86_64-unknown-linux-gnu
```

The repository packaging contract produced and verified these files:

| Artifact | Size (bytes) | SHA-256 for this run |
|---|---:|---|
| `musubicad-v0.1.0-linux-x86_64.AppImage` | 94,112,248 | `b29a5beb71570fdfc905d9c4c66292c6991690112bc43a729e7eb7d909e85b90` |
| `musubicad-v0.1.0-linux-x86_64.deb` | 18,162,860 | `d7776c2dc42a67bd725e8ee635720b65ffbf34d7df28145bbf3f0ef8d719b4bb` |

With `libvulkan1`, Mesa Vulkan, and `WGPU_BACKEND=vulkan`, the packaged
AppImage completed `--smoke-test` through `APPIMAGE_EXTRACT_AND_RUN=1`. It
reported an unchanged source, a `100 mm` width edit, 144 regenerated and
exported triangles, a semantic top-face pick, and 36 highlight segments. The
Debian package was then extracted with `dpkg-deb -x`; its installed
`usr/bin/musubicad-desktop` payload completed the same contract. Both commands
exited with code 0 and wrote new working `.ocad.d` documents.

## GitHub-hosted native matrix — 2026-08-23

Source revision: `130028187a2031a9bbde6613463558a248a65563` on `main`.

GitHub Actions run
[`32612751044`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32612751044)
completed successfully on all four required native runners. Every matrix job
verified its native SDK or toolchain, installed the pinned Tauri CLI, checked
the locked dependency graph and package version, built the application,
verified the native executable and unchanged lockfile, generated two
platform artifacts plus `SHA256SUMS`, checked the artifact contract, and
uploaded the versioned result.

| Job | Result | Uploaded workflow artifact | Archive size (bytes) |
|---|---|---|---:|
| Windows x86_64 | Success | `musubicad-desktop-v0.1.0-windows-x86_64` | 26,960,066 |
| Linux x86_64 | Success | `musubicad-desktop-v0.1.0-linux-x86_64` | 108,606,234 |
| macOS x86_64 | Success | `musubicad-desktop-v0.1.0-macos-x86_64` | 29,533,685 |
| macOS arm64 | Success | `musubicad-desktop-v0.1.0-macos-aarch64` | 27,444,592 |

The dependent `headless backend and packaged Linux smoke` job downloaded the
uploaded Linux artifact, verified `SHA256SUMS`, ran the OCCT backend smoke and
command-parity audit, and executed the packaged AppImage under Mesa Vulkan.
The independent CI run
[`32612751048`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32612751048)
also passed the workflow policy, formatting, clippy, examples, golden, render,
and workspace test gates for the same revision.

## Remaining external evidence

The following release-level evidence remains external:

- downloadable artifacts and checksums attached to a matching version tag;
- Authenticode signing with the protected Windows certificate;
- macOS codesign verification, notarization, stapling, and Gatekeeper checks.

At verification time, the GitHub repository exposed no Actions secrets and no
`desktop-release` environment through the authenticated read-only audit. These
These external items remain the completion evidence for MCAD-P1-002 and
MCAD-P1-004. The successful hosted matrix completes MCAD-P1-001.
