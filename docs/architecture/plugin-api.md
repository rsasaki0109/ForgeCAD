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
and does not define dynamic loading or sandboxing. Those concerns, along with
deterministic registry order and CLI/Agent product integration, are deferred to
P4-002/P4-003.

See [the public API contract](../api/plugin-api.md) and
[ADR-010](../adr/ADR-010-versioned-plugin-contracts.md).
