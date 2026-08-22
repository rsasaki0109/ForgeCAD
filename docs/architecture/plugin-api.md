# Plugin API architecture

P4-001 establishes a data-only extension boundary above the command layer:

```text
linked Rust plugin
        │  PluginManifest + version check
        ▼
FeatureRequest / ImportRequest / ExportRequest
        │
        ├── feature/importer → DesignPatch DTO → host validation + transaction
        └── exporter         ← immutable serializable state; → output bytes
```

The plugin crate is deliberately not a new document or geometry owner. It
does not import `OcadDocument`, expose `GeometryKernel` or OCCT types, retain a
mutable graph reference, access the filesystem/network, or carry UI state. A
feature or importer can propose only a serializable `DesignPatch`; the host
still performs precondition, schema, semantic, and transaction validation.
An exporter receives a serializable JSON state DTO and returns caller-owned
bytes, leaving output persistence to the host.

The initial contract is linked Rust only. It has explicit major/minor API
compatibility and a versioned manifest schema, but it is not an ABI guarantee
and does not define dynamic loading or sandboxing. P4-002 adds a
`BTreeMap`-backed registry and a
serializable host capability policy. Discovery validates manifests and returns
stable ID ordering without calling feature/import/export execution methods.
This is a policy boundary for exposed DTO operations, not a sandbox: linked
in-process implementations are trusted, and registration/listing call their
`manifest()` accessor. Capabilities are limited to proposing feature/import
patches or producing export bytes; filesystem, network, UI, kernel, and
document-owner access cannot be declared. CLI/Agent product integration remains
host-owned. P4-003 adds a non-global `PluginHost` in the CLI layer. It supplies
immutable DTOs to linked implementations, routes feature/importer patches
through the existing dry-run and transaction/history boundary, and persists
export bytes only after the plugin returns. The CLI and Agent API use the same
host service, so neither surface can bypass DesignPatch validation.

See [the public API contract](../api/plugin-api.md) and
[ADR-010](../adr/ADR-010-versioned-plugin-contracts.md).
