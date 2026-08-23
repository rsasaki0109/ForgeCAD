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

## Tagged v0.1.1 distribution — 2026-08-23

Source revision: `06eaa3849c06db991bf39c459819b30130598351`, tagged
`v0.1.1` on `main`.

GitHub Actions run
[`32616853187`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32616853187)
completed the tagged Desktop matrix on all four native runners. Its four
downloadable workflow artifacts contain the expected two installers or
archives plus `SHA256SUMS`:

| Workflow artifact | Download size (bytes) |
|---|---:|
| `musubicad-desktop-v0.1.1-windows-x86_64` | 26,951,243 |
| `musubicad-desktop-v0.1.1-linux-x86_64` | 108,587,206 |
| `musubicad-desktop-v0.1.1-macos-aarch64` | 27,446,131 |
| `musubicad-desktop-v0.1.1-macos-x86_64` | 29,535,591 |

The dependent job downloaded the Linux artifact, verified its checksums, and
ran the packaged AppImage smoke contract. A separate post-run download of all
four artifacts verified every internal checksum independently:

| Packaged file | Size (bytes) | SHA-256 |
|---|---:|---|
| `musubicad-v0.1.1-windows-x86_64-nsis.exe` | 11,089,372 | `e02b9c502e37273e764a6ca5e4c523f194d167eabbabbc820ae896726c688509` |
| `musubicad-v0.1.1-windows-x86_64.msi` | 16,150,528 | `77cfbfe3260ec84fbc6f5891e581467cc2b455714fea6988afd9da9e808728dc` |
| `musubicad-v0.1.1-linux-x86_64.AppImage` | 91,122,168 | `cba35092a569cf952f5f1832f4fd1faa47d533923275f569a55f8dfe1e89f86e` |
| `musubicad-v0.1.1-linux-x86_64.deb` | 18,163,976 | `a505e542e4c33c1d7c6122e0fa3476289d0082f1055fefd7aebfe1e4b47918c5` |
| `musubicad-v0.1.1-macos-aarch64-app.zip` | 13,477,985 | `0774d7d6e983aa698769841a3637ad908a1a1ec3d211d3837325c62ae6dc8904` |
| `musubicad-v0.1.1-macos-aarch64.dmg` | 14,087,217 | `af25dc6616cd0e534e8ceadd84ac944a54bbab300b45f56f3b31a2aad1db2687` |
| `musubicad-v0.1.1-macos-x86_64-app.zip` | 14,531,167 | `5f812c04a5dc623463ee5d65eb48e93b0ecb22738a416844b0a92e04433a5e1f` |
| `musubicad-v0.1.1-macos-x86_64.dmg` | 15,109,047 | `c624a417cddbfeb06b5d799aaf4daaf091c71de7103163947c90b4845a8ac4e6` |

The independent CLI
[`Release` run `32616853192`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32616853192)
built and smoke-tested all four CLI targets and published the non-draft,
non-prerelease
[`MusubiCAD CLI v0.1.1`](https://github.com/rsasaki0109/MusubiCAD/releases/tag/v0.1.1)
release with four archives and `SHA256SUMS`. A clean post-publication download
verified all four checksums; the released Windows executable reported
`opencad 0.1.1` and `OCCT 8.0.0 (cadrum static)`. This tagged evidence, together
with the checked-in distribution quick start, completes MCAD-P1-002.

## Credential-gated signed release attempt — 2026-08-23

The successful CLI release triggered
[`Desktop signed release` run `32617085661`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32617085661).
The trusted preflight verified the immutable tag, `main` ancestry, and shared
desktop version. The Linux checksum-only job built, verified, and uploaded its
artifact. Windows then failed before build/publication because its certificate,
password, thumbprint, and timestamp URL were empty. Both macOS jobs failed
before build/publication because the required Apple certificate and associated
notarization credentials were empty. Consequently, the all-platform publish
job was skipped and no unsigned artifact was presented as a signed release.
The workflow reference created the `desktop-release` environment, but a
post-run API audit found zero environment secrets, zero environment variables,
and zero protection rules. Repository administrators would need to configure
the credentials and reviewer/branch protection described in the distribution
guide before enabling signed publication.

The observed failures are positive evidence that the release path fails closed
when protected credentials are absent. By product-owner direction on
2026-08-23, production credential provisioning and the resulting Authenticode,
timestamp, codesign, notarization, stapling, and Gatekeeper evidence are
deferred. MCAD-P1-004 is complete at the policy and gated-workflow scope; no
signed-publication claim is made, and the verified unsigned artifacts remain
the current supported desktop distribution.

After the credential gates were moved ahead of SDK, Rust, cache, Tauri CLI,
dependency, and build setup, a manual dispatch against the same immutable
`v0.1.1` tag verified the optimized ordering in
[`Desktop signed release` run `32619521109`](https://github.com/rsasaki0109/MusubiCAD/actions/runs/32619521109).
The Windows job failed closed in 13 seconds and both macOS jobs failed closed
in 8 seconds; every expensive setup and build step was skipped on those three
runners. The independent Linux checksum-only job remained unaffected and
completed successfully in 3 minutes 31 seconds. Because the signed platform
jobs did not all succeed, the publish job was skipped. This confirms that
missing credentials are rejected early without weakening the all-platform
publication gate.
