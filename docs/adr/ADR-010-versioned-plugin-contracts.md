# ADR-010: Versioned linked Rust plugin contracts

Status: Accepted  
Date: 2026-08-23

## Context

The placeholder `opencad-plugin-api` crate needs an extension boundary for
custom features, importers, and exporters. Plugins must not acquire document
ownership, bypass DesignPatch validation, leak concrete OCCT types, or couple
the core to filesystem, network, or UI state. A version marker is required so
future contract changes fail explicitly instead of being interpreted silently.

The first milestone is intentionally smaller than a product plugin system:
registry ordering, capability policy, loading, sandboxing, and CLI/Agent
integration are separate roadmap work.

## Decision

Define `PluginManifest` with the exact serialized schema
`musubicad.plugin-manifest.v1` and `PluginApiVersion { major, minor }`. A host
accepts a plugin when the major versions match and the plugin minor is no newer
than the host minor. Manifest schema, identity, and version fields are checked
before invocation. The current linked contract is API `1.0`.

Expose three small linked Rust traits:

- `FeaturePlugin::apply(FeatureRequest) -> OpenCadError-compatible Result`,
  returning a `FeatureResult` with a `DesignPatch`.
- `ImporterPlugin::import(ImportRequest) -> OpenCadError-compatible Result`,
  accepting caller-owned bytes and returning a `DesignPatch`.
- `ExporterPlugin::export(ExportRequest) -> OpenCadError-compatible Result`,
  accepting immutable serializable state and returning caller-owned bytes.

Request/result DTOs are serializable. Public fallible methods use the
repository-standard `opencad_core::Result<T>`; `PluginError` remains available
as a serializable diagnostic DTO rather than introducing a second public error
convention.

This is a linked Rust contract, not an ABI-stable dynamic-loading promise. No
registry, loader, capability negotiation, document integration, or product
surface is introduced by this decision.

P4-002 extends the v1 manifest with a serde-defaulted, sorted capability set
and stores trusted linked implementations in a `BTreeMap` keyed by manifest
ID. Each kind declares exactly one data-only capability (`feature_patch`,
`import_patch`, or `export_bytes`), which an explicit host policy may reject
before registration. Missing, extra, kind-mismatched, duplicate, and
policy-disallowed declarations fail deterministically. Discovery does not call
feature/import/export execution methods. This is not process isolation; linked
code and its `manifest()` accessor are trusted.

## Consequences

- Plugin proposals still pass through host DesignPatch validation and
  transactions.
- Manifest compatibility failures are deterministic and explicit.
- The API can evolve by major/minor version without silently accepting a
  changed representation.
- The boundary is easy to serialize and test without OCCT or filesystem I/O.
- Future registry/loading/security decisions remain independent follow-up
  work and may add process isolation without changing the DTO direction.
