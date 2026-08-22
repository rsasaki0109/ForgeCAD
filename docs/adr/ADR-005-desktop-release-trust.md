# ADR-005: Separate unsigned desktop CI from trusted release artifacts

- Status: Accepted
- Date: 2026-08-23
- Scope: `MCAD-P1-004`

## Context

The Tauri desktop bundle is useful as a pull-request and `main` CI artifact,
but Windows Authenticode certificates and Apple Developer ID/notarization
credentials must never be exposed to code from an untrusted pull request. A
checksum also proves integrity only relative to the checksum file; it does not
prove publisher identity. Linux packages currently have no project signing
key.

Tauri v2 documents importing a Windows PFX certificate and configuring the
bundle's `certificateThumbprint`, `digestAlgorithm`, and `timestampUrl`. Its
macOS CI guidance uses a base64-encoded P12 certificate in an ephemeral
keychain and Apple credentials for notarization. GitHub Actions documents that
fork pull-request workflows do not receive secrets and that `GITHUB_TOKEN`
permissions should be scoped per job.

## Decision

1. `.github/workflows/desktop.yml` remains the unsigned, read-only-permission
   build. It may run for pull requests, `main`, tags, and manual dispatches and
   uploads the existing versioned artifacts plus `SHA256SUMS`. It never reads a
   signing secret and never publishes a release.
2. `.github/workflows/desktop-signed-release.yml` is a separate workflow. It
   starts after the `Release` workflow successfully completes for a tag push,
   or from a manual dispatch naming an existing tag. It verifies
   the tag commit is on `main` and matches both desktop manifests before a
   build starts.
3. Every signed-build matrix job uses the protected `desktop-release`
   environment. Missing credentials fail closed. Windows imports a base64 PFX
   and configures Tauri's Authenticode settings, then verifies every executable
   and installer with `Get-AuthenticodeSignature`. macOS imports a base64 P12
   into a temporary keychain, verifies `codesign`, submits the app and DMG to
   `xcrun notarytool`, staples and validates both, and re-runs `codesign` and
   `spctl` before packaging.
4. Linux artifacts are explicitly checksum-only. They are not represented as
   code-signed or notarized, and their release metadata says so.
5. The publish job is the only job with `contents: write`. It runs only after
   all four platform jobs pass, re-checks each `SHA256SUMS` and trust-evidence
   file, generates an aggregate checksum, and then uploads to the immutable
   version release created by the CLI release workflow. It never creates the
   shared release and never replaces an existing asset. The desktop
   aggregate is named `MUSUBICAD-DESKTOP-SHA256SUMS` so it cannot replace the
   CLI workflow's generic `SHA256SUMS` asset. The protected environment
   provides a maintainer review gate before credentials or publication are
   authorized.
6. The workflow intentionally has no `pull_request` or `workflow_call` entry
   point. A future release orchestrator requires a separate security review;
   it must not gain signing credentials merely by calling this workflow.
7. The validated commit SHA, rather than the movable tag name, is passed to
   every build and publish job. The completed CLI release workflow SHA must
   match the tag, and the publish job re-fetches the tag and rejects
   publication if it moved.
8. Third-party actions in the credential-bearing workflow are pinned to full
   commit SHAs. Imported PFX/P12 files and temporary certificate stores are
   removed in `always()` cleanup steps.

## Required protected-environment configuration

Create a GitHub Actions environment named `desktop-release` with required
reviewers. Store these as environment secrets, not repository files:

- `WINDOWS_CERTIFICATE`: base64-encoded Authenticode `.pfx`.
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX export password.
- `APPLE_CERTIFICATE`: base64-encoded Developer ID Application `.p12`.
- `APPLE_CERTIFICATE_PASSWORD`: P12 export password.
- `KEYCHAIN_PASSWORD`: random ephemeral keychain password.
- `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID`: Apple notarization
  credentials; `APPLE_PASSWORD` must be an app-specific password.

Store these non-secret Actions variables in the same environment:

- `WINDOWS_SIGNING_THUMBPRINT`: normalized certificate thumbprint.
- `WINDOWS_TIMESTAMP_URL`: approved Authenticode RFC 3161 timestamp service.
- `APPLE_SIGNING_IDENTITY` (optional): exact `Developer ID Application: ...`
  identity; if omitted, the workflow discovers the imported identity.

The platform preflight steps make credentials mandatory for the relevant
runner, so a missing value cannot silently produce an unsigned artifact.

## Consequences

Unsigned artifacts remain easy to produce and test, while release credentials
are not present in pull-request execution. A signed release rebuilds the tag,
so it does not trust or mutate an unsigned artifact from another workflow run.
The four-platform signed build is slower and requires real certificates and
Apple account access; until those are configured and a tagged run succeeds,
the repository must describe signed publication as not yet verified.

## References

- [Tauri v2 Windows code signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri v2 macOS code signing and notarization](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri v2 GitHub Actions pipeline](https://v2.tauri.app/distribute/pipelines/github/)
- [GitHub Actions workflow syntax and permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
- [GitHub Actions events and fork secrets](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)
