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

## Remaining external evidence

The following cannot be established by this Windows checkout:

- GitHub-hosted Windows, Linux x86_64, macOS arm64, and macOS x64 matrix runs
  from the 24 unpushed roadmap commits;
- downloadable artifacts and checksums attached to a matching version tag;
- Linux AppImage execution on a Linux runner;
- Authenticode signing with the protected Windows certificate;
- macOS codesign verification, notarization, stapling, and Gatekeeper checks.

At verification time, the GitHub repository exposed no Actions secrets and no
`desktop-release` environment through the authenticated read-only audit. These
external items remain the completion evidence for MCAD-P1-001, MCAD-P1-002,
and MCAD-P1-004.
