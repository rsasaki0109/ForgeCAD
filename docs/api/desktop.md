# Desktop backend and command parity

`opencad-desktop` is the backend boundary used by the Tauri shell and by
headless desktop tests. It reads the Design Graph from `.ocad`/`.ocad.d`,
regenerates disposable B-Rep/mesh data, and returns serializable summaries.
The web UI does not own or mutate model structures; it invokes a Tauri command
for every document operation. Viewport camera and selection state remain in the
render/viewport layer and are not written to the document.

## Shared backend API

| Function | Contract |
|---|---|
| `inspect_document(path)` | Open and summarize document metadata, graph counts, and semantic references |
| `preview_document(path)` | Regenerate, tessellate, and render a deterministic offscreen PNG preview |
| `regenerate_document(path)` | Read-only OCCT regeneration report and triangle count for a part document |
| `list_document_parameters(path)` | Return parameters in deterministic evaluation order with explicit unit values |
| `set_document_parameter(path, id, expr)` | Persist one parameter expression (the Phase 3 transaction boundary will unify this with `DesignPatch`) |
| `pick_document(path, options)` | Headless offscreen selection query with semantic/topological context |
| `export_stl_document(path, output)` | Regenerate a part and write a disposable binary STL |
| `run_desktop_smoke(source, work_dir)` | Copy a part fixture into a new work directory and return serializable open/preview/edit/regenerate/pick/export evidence |

The regeneration and export result types contain only serializable strings,
counts, and paths. They do not expose OCCT handles or cached geometry
ownership. Assembly and drawing export continue to use their specialized
CLI/Agent API paths until the corresponding Phase 5 backend contracts are
completed. `regenerate_document` and `export_stl_document` are shared backend
functions, not Tauri/UI commands.

## Tauri/UI/CLI/Agent parity

| UI action | Tauri command | CLI route | Agent API route |
|---|---|---|---|
| Open | `inspect_document_cmd` + `preview_document_cmd` | `inspect`, `screenshot` | `opencad.inspect` for document metadata |
| Preview | `preview_document_cmd` | `screenshot` | — (CLI provides parity) |
| Refresh | `inspect_document_cmd` + `preview_document_cmd` | `inspect`, `screenshot` | — (CLI provides parity) |
| List parameters | `list_document_parameters_cmd` | `params [--json]` | `opencad.query_document` / document inspection |
| Edit parameter | `set_document_parameter_cmd` | `patch` with `set_parameter` | `opencad.patch_apply_document` |
| Pick | `pick_document_cmd` | `pick` | `opencad.pick_document` |
| Create sample | `create_template_document` | `new` | — (CLI provides parity) |
| Open viewport | `open_viewport_cmd` | `view` | — (CLI provides parity) |

The command parity integration test includes the UI and Tauri command surfaces
as compile-time fixtures and checks the actual `generate_handler!` registration.
It fails if a document/viewport model operation loses its Tauri handler or its
CLI/Agent route. The `default_example_path` call is
intentionally classified as a shell-only bootstrap path resolver, not a model
command. Refresh deliberately calls the same inspect/preview load path as Open;
there is no separate UI regenerate or export command. The test also checks that
the UI keeps document path state separate from viewport preview synchronization
and does not contain direct Design Graph mutation expressions.

## Headless smoke test

Run the real checked-in example through the backend on a machine with OCCT and
a headless Vulkan adapter:

```powershell
./tools/desktop-smoke.ps1
```

The test calls `run_desktop_smoke(source, work_dir)`, which copies
`examples/bracket.ocad.d` to a new temporary directory and returns a JSON-safe
summary after verifying open/validate, PNG preview, a `100 mm` width edit, OCCT
regeneration, offscreen pick/highlight, and binary STL export. It accepts only
`DocumentKind::Part`, rejects an existing work directory, validates the edited
copy, and compares every source fixture file as a `BTreeMap<String, Vec<u8>>`
so the committed example remains byte-for-byte unchanged.

The packaged Tauri binary exposes the same check without opening a GUI:

```text
musubicad-desktop --version
musubicad-desktop --smoke-test <source.ocad.d> <new-work-dir.ocad.d>
```

The Linux workflow downloads its AppImage and runs this mode with
`APPIMAGE_EXTRACT_AND_RUN=1` under Mesa Vulkan before accepting the artifact.
