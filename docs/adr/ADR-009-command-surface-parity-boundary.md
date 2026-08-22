# ADR-009: Command-surface parity and regeneration boundary

Status: Accepted  
Date: 2026-08-23

## Context

The desktop UI, Tauri shell, CLI, and Agent API must expose the same model
operations. A static command-name mapping alone does not prevent one surface
from bypassing `DesignPatch` validation or backend history. Separately, the
file module contained an orchestration helper that combined file-layer patch
application with feature regeneration, crossing the module boundary between
serialization and command execution.

## Decision

Model-mutating UI commands delegate to the desktop backend. Parameter edits
construct a `DesignPatch` and use the shared validated history transaction;
the CLI and Agent document patch routes use the same file-layer candidate
application contract. Template creation is shared by the desktop and CLI
surfaces. Command parity tests check the UI/Tauri/CLI/Agent route mapping,
implementation delegation, and a runtime equivalence between the desktop
parameter command and a direct `DesignPatch` transaction.

The file module remains responsible for serialization and validated in-memory
patch application. The cross-module `apply_patch_and_regenerate` orchestration
now lives in `opencad-desktop`, where command-layer code may invoke feature
regeneration on a disposable candidate and commit only after success. This
does not change the `.ocad` schema.

## Consequences

- New UI model commands must add a CLI or Agent route and parity coverage.
- Desktop command paths share transaction and history behavior with headless
  clients instead of maintaining GUI-local model mutation logic.
- File serialization remains independent of feature execution.
- Regeneration tests remain available through the desktop command boundary,
  while `opencad-file` tests cover patch validation and persistence.
