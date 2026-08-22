# Desktop distribution quick start

The `Desktop` workflow builds the Tauri shell on four native GitHub-hosted
runners. It uploads one versioned artifact per runner, containing the
platform's supported installer/archive formats and a `SHA256SUMS` file:

| Platform | Workflow artifact | Files inside the artifact |
|---|---|---|
| Windows x86-64 | `musubicad-desktop-v<version>-windows-x86_64` | `musubicad-v<version>-windows-x86_64.msi`, `musubicad-v<version>-windows-x86_64-nsis.exe`, `SHA256SUMS` |
| Linux x86-64 | `musubicad-desktop-v<version>-linux-x86_64` | `musubicad-v<version>-linux-x86_64.deb`, `musubicad-v<version>-linux-x86_64.AppImage`, `SHA256SUMS` |
| macOS arm64 | `musubicad-desktop-v<version>-macos-aarch64` | `musubicad-v<version>-macos-aarch64.dmg`, `musubicad-v<version>-macos-aarch64-app.zip`, `SHA256SUMS` |
| macOS Intel | `musubicad-desktop-v<version>-macos-x86_64` | `musubicad-v<version>-macos-x86_64.dmg`, `musubicad-v<version>-macos-x86_64-app.zip`, `SHA256SUMS` |

The normalized names are independent of Tauri's generated bundle filename.
`SHA256SUMS` contains exactly the two installable/archive files in that
platform artifact; the checksum file does not hash itself.

## Verify a downloaded artifact

Extract one workflow artifact and run the checksum verification before opening
an installer. On Linux:

```bash
sha256sum --check SHA256SUMS
```

On macOS, use the system `shasum` implementation:

```bash
shasum -a 256 --check SHA256SUMS
```

On Windows PowerShell:

```powershell
Get-Content .\SHA256SUMS | ForEach-Object {
  $parts = $_ -split '  ', 2
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $parts[1]).Hash.ToLowerInvariant()
  if ($actual -ne $parts[0].ToLowerInvariant()) { throw "SHA-256 mismatch: $($parts[1])" }
}
```

The version in every filename must match the package version. A version tag is
accepted only when it is exactly `v<version>` and matches both
`apps/desktop/src-tauri/Cargo.toml` and `tauri.conf.json`.

## Install

- Windows: run either the `.msi` package (WiX) or the `-nsis.exe` setup package.
- Linux: install the Debian package with `sudo apt install ./musubicad-*.deb`, or
  run the AppImage after `chmod +x musubicad-*.AppImage`.
- macOS: open the `.dmg` and drag MusubiCAD to Applications. The `-app.zip`
  file contains the same `.app` bundle and is useful for scripted extraction.

## Trust and signed publication

The normal [`Desktop` workflow](../../.github/workflows/desktop.yml) is an
unsigned CI workflow. It is intentionally safe for pull requests and `main`:
it has only `contents: read`, never reads signing credentials, and uploads
versioned artifacts plus `SHA256SUMS`. Windows SmartScreen and macOS
Gatekeeper may warn when those CI artifacts are launched. A checksum verifies
the downloaded bytes against the checksum file; it does not assert publisher
identity.

Signed publication is a separate, credential-gated
[`Desktop signed release` workflow](../../.github/workflows/desktop-signed-release.yml).
It starts only after the CLI `Release` workflow successfully publishes a
`v<version>` tag, or from a protected manual dispatch naming an existing tag.
The workflow verifies that
the tag is on `main` and matches both desktop manifests before entering the
protected `desktop-release` environment. It rebuilds the tag with signing
credentials instead of trusting an artifact from an untrusted workflow run.

Windows uses an Authenticode PFX and verifies every executable and installer
with `Get-AuthenticodeSignature`. macOS uses a Developer ID Application P12,
an ephemeral keychain, Apple notarization credentials, `codesign`,
`xcrun notarytool`, `xcrun stapler`, and `spctl` verification. The publish job
has the only `contents: write` permission and re-checks checksums and the
platform trust evidence before calling `gh release upload`.
The CLI workflow owns creation of the shared version release. The desktop
workflow fails if that release is absent or if any same-named asset already
exists; it never overwrites the CLI's generic `SHA256SUMS`. The aggregate
desktop checksum is named
`MUSUBICAD-DESKTOP-SHA256SUMS`.

Linux packages are deliberately checksum-only in both trust scope and release
metadata; MusubiCAD does not currently publish a Linux signing key or package
signature. Verify `SHA256SUMS` through a trusted release channel before
execution.

Pull requests, including fork pull requests, never trigger the signed
workflow, and the workflow exposes no reusable-workflow entry point. Missing
Windows or Apple credentials fail closed rather than falling back to unsigned
publication.

### Protected environment configuration

Create an Actions environment named `desktop-release` with required reviewers.
Add these environment secrets:

| Secret | Purpose |
|---|---|
| `WINDOWS_CERTIFICATE` | Base64 Authenticode `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX export password |
| `APPLE_CERTIFICATE` | Base64 Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | P12 export password |
| `KEYCHAIN_PASSWORD` | Ephemeral macOS CI keychain password |
| `APPLE_ID` | Apple notarization account |
| `APPLE_PASSWORD` | Apple app-specific notarization password |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

Add these non-secret environment variables:

| Variable | Purpose |
|---|---|
| `WINDOWS_SIGNING_THUMBPRINT` | Certificate selected by Authenticode |
| `WINDOWS_TIMESTAMP_URL` | RFC 3161 timestamp service |
| `APPLE_SIGNING_IDENTITY` | Optional exact `Developer ID Application: ...` identity |

The platform preflight checks require all relevant values, so a missing
credential stops the release before artifacts are published. The decision and
limitations are recorded in
[ADR-005](../adr/ADR-005-desktop-release-trust.md).

## Build locally

Install the platform prerequisites from the [desktop UI guide](desktop-ui.md)
and install the pinned Tauri CLI:

```bash
cargo install tauri-cli --version 2.11.4 --locked
```

From `apps/desktop/src-tauri`, build only the bundles supported by the host:

```bash
# Windows
cargo tauri build --ci --bundles msi,nsis --target x86_64-pc-windows-msvc

# Linux
cargo tauri build --ci --bundles deb,appimage --target x86_64-unknown-linux-gnu

# macOS (use the target matching the runner/host)
cargo tauri build --ci --bundles app,dmg --target aarch64-apple-darwin
```

After a successful build, the workflow's deterministic packaging and checksum
contract can be exercised from the repository root. The output directory must
be new or empty:

```powershell
./tools/package-desktop-release.ps1 `
  -Version 0.1.0 `
  -Platform windows-x86_64 `
  -BundleDirectory apps/desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle `
  -OutputDirectory dist
```

The packaging helper fails if a required bundle is missing, more than one
candidate is present, an output is empty, or a generated SHA-256 entry does not
verify. It uses `ditto` for the macOS `.app` archive so bundle metadata is
preserved.

The four-runner build and install/open smoke tests have not yet been confirmed
on a tagged CI run. They remain part of the Phase 1 acceptance evidence for
`MCAD-P1-001` and `MCAD-P1-002`; the shared desktop smoke contract itself is
implemented by `MCAD-P1-003`.
