# Plugin API (P4-001)

`opencad-plugin-api` defines the first versioned extension boundary for
MusubiCAD. It is a linked, stable-Rust contract: plugin implementations are
compiled against the crate and exchange request/result DTOs in process. P4-001
does not claim an ABI-stable dynamic-loading format and does not implement a
registry, discovery, sandbox, CLI route, or Agent route.

## Boundary

The contract deliberately carries data rather than ownership:

| Contract | Request | Result |
|---|---|---|
| `FeaturePlugin` | `FeatureRequest` with semantic IDs, unit-bearing expression strings, and serializable inputs | `FeatureResult` containing a `DesignPatch` and diagnostics |
| `ImporterPlugin` | `ImportRequest` with a format, optional source name, and caller-owned bytes | `ImportResult` containing a `DesignPatch` and diagnostics |
| `ExporterPlugin` | `ExportRequest` with a format and immutable `serde_json::Value` state | `ExportResult` with format, media type, bytes, and diagnostics |

No contract accepts or returns `OcadDocument`, `&mut` document state, OCCT
handles, viewport/camera/selection state, filesystem paths/handles, network
clients, or direct mutation callbacks. The host must validate a returned
`DesignPatch` through the normal command/transaction boundary.

All fallible manifest and trait methods return the repository-standard
`opencad_core::Result<T>` (`OpenCadError`). `PluginError` is a serializable
diagnostic DTO for plugin-facing error data; it is not a replacement error
type for public fallible APIs.

## Manifest and compatibility

Each plugin declares a serializable `PluginManifest`:

```json
{
  "schema": "musubicad.plugin-manifest.v1",
  "id": "example.bracket-feature",
  "name": "Bracket Feature Example",
  "version": "0.1.0",
  "api_version": { "major": 1, "minor": 0 },
  "kind": "feature"
}
```

The current contract is `CURRENT_PLUGIN_API = { major: 1, minor: 0 }`.
Compatibility is directional and explicit: a plugin is accepted when its major
version equals the host major and its minor version is less than or equal to the
host minor. Future minor versions and every major mismatch are rejected before
invocation. The manifest schema is also checked exactly.

The exact checked-in example is
[`examples/plugin-example/manifest.json`](../../examples/plugin-example/manifest.json).

## Linked Rust shape

The core trait shape is intentionally small:

```rust
pub trait FeaturePlugin {
    fn manifest(&self) -> &PluginManifest;
    fn apply(&self, request: FeatureRequest) -> opencad_core::Result<FeatureResult>;
}

pub trait ImporterPlugin {
    fn manifest(&self) -> &PluginManifest;
    fn import(&self, request: ImportRequest) -> opencad_core::Result<ImportResult>;
}

pub trait ExporterPlugin {
    fn manifest(&self) -> &PluginManifest;
    fn export(&self, request: ExportRequest) -> opencad_core::Result<ExportResult>;
}
```

The request/result structures derive `Serialize` and `Deserialize` where both
directions are meaningful. Focused crate tests assert round trips and exact
manifest JSON bytes. Run them with:

```bash
cargo test -p opencad-plugin-api
cargo clippy -p opencad-plugin-api --all-targets -- -D warnings
```

Registry ordering, capability declarations, loading/security, and CLI/Agent
invocation are follow-up contracts in P4-002 and P4-003.
