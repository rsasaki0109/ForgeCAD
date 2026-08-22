# Developing linked plugins

MusubiCAD's v1 plugin boundary is a stable-Rust, in-process contract. Plugin
crates are linked into a host at build time; there is no ABI-stable dynamic
loader or sandbox. Treat every registered plugin as trusted application code.

The complete feature example is `examples/plugin-example`. It depends only on
`opencad-plugin-api`, `opencad-ai`, and `opencad-core`; it does not own a
document, call a geometry kernel, access the UI, or perform filesystem/network
I/O.

## Authoring workflow

1. Implement exactly one of `FeaturePlugin`, `ImporterPlugin`, or
   `ExporterPlugin`.
2. Return a stable `PluginManifest` with schema
   `musubicad.plugin-manifest.v1`, API version `1.0`, the matching kind, and its
   required capability.
3. Use semantic IDs and unit-bearing expressions such as `100 mm` in DTOs.
4. Return a `DesignPatch` from feature/importer plugins. The host owns dry-run,
   validation, transactions, regeneration, history, and persistence.
5. Return caller-owned bytes from exporters. The host alone chooses whether and
   where to write them.
6. Register the implementation in a host-owned `PluginRegistry`. Registration
   rejects incompatible API versions, duplicate IDs, invalid kinds, or
   disallowed capabilities before invocation.

Do not add `OcadDocument`, OCCT handles, viewport state, paths, file handles,
network clients, or mutation callbacks to plugin request/result contracts.

## Compatibility and evidence

A 1.x host accepts a plugin with the same major version and a minor version no
newer than the host. Major mismatches and future minor versions are rejected.
Because plugins execute in process, a Rust panic can still terminate or poison
the host; returned `OpenCadError` values are isolated and leave the document
unchanged, but panic/process isolation is future work.

Run the contract, example, integration, and lint evidence with:

```bash
cargo test -p opencad-plugin-api
cargo test -p opencad-plugin-example
cargo test -p opencad-cli plugin
cargo clippy -p opencad-plugin-api -p opencad-plugin-example -p opencad-cli --all-targets -- -D warnings
```

The example tests pin the manifest and deterministic feature result. CLI tests
pin importer/exporter bytes under `modules/cli/tests/golden` and verify that
plugin errors and invalid patches do not alter persisted documents.
