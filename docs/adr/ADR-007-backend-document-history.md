# ADR-007: Serializable backend document history

Status: Accepted  
Date: 2026-08-23

## Context

MusubiCAD needs undo/redo for Design Graph edits while keeping the Design Graph
as the source of truth. A history implementation must work for the file layer,
desktop backend, Tauri shell, and Agent API without introducing process-global
document state or writing viewport state into `.ocad` files.

## Decision

`opencad-file` provides a serializable `DocumentHistory` containing complete
`OcadDocument` before/after snapshots and a command description for each
successful change. `apply_patch_with_history` stages the validated
`DesignPatch` candidate first, then records and commits it atomically. Undo and
redo compare canonical `serialize_document_files` output to the expected
source snapshot before changing either the document or history. A new record
clears the redo branch.

History is transport-only and remains outside the `.ocad` schema. Desktop,
Tauri, and Agent commands return a `DocumentHistoryState`; clients pass its
`history` member back opaquely and use only `can_undo`/`can_redo` capability
flags. No global history store is used, and viewport camera, selection, B-Rep,
and mesh caches are excluded.

## Rationale and trade-offs

Full source snapshots make undo/redo deterministic for all serializable
document fields, including metadata, parameter/feature/sketch graphs,
semantic references, assemblies, and drawings. They avoid inverse-command
logic that could drift as feature semantics evolve. Canonical source comparison
also treats history identity as byte-exact serialized source identity without
using geometry floating-point equality.

The trade-off is snapshot size: a long edit session can carry more data than a
reversible command log. History is therefore bounded by the caller's retained
transport value and is not persisted in `.ocad`; a future storage/compaction
decision can be made without changing the document schema or client contract.

## Consequences

- Failed patch validation and stale undo/redo checks leave both values unchanged.
- `.ocad` round trips remain schema-compatible and deterministic.
- UI clients cannot accidentally recreate semantic inverse stacks.
- Preconditions and rebase behavior remain follow-up work under MCAD-P3-004.
