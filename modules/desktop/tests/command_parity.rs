//! Static contract audit for Tauri/UI command parity.
//!
//! Tauri itself is intentionally excluded from the Rust workspace.  Including
//! the small command surfaces here keeps the audit headless and makes drift
//! fail in the normal workspace test rather than only during a GUI build.

const TAURI_SOURCE: &str = include_str!("../../../apps/desktop/src-tauri/src/lib.rs");
const UI_SOURCE: &str = include_str!("../../../apps/desktop/ui/main.js");
const CLI_SOURCE: &str = include_str!("../../../modules/cli/src/commands.rs");
const AGENT_SOURCE: &str = include_str!("../../../modules/cli/src/agent.rs");
const DESKTOP_API_DOC: &str = include_str!("../../../docs/api/desktop.md");

#[test]
fn every_ui_model_command_has_a_tauri_handler_and_cli_or_agent_route() {
    // The first item is the exact command string passed to Tauri invoke; the
    // second is the corresponding Rust handler; the third documents the
    // command-line or Agent API route available outside the GUI.
    // `default_example_path` is a shell bootstrap helper: it only resolves a
    // repository-local path before a document is open and has no model command
    // equivalent. Refresh is intentionally only the existing inspect/preview
    // load path; regeneration/export stay reusable backend operations for the
    // headless smoke contract and are not Tauri/UI commands.
    let parity = [
        ("list_templates", "list_templates", "new"),
        ("inspect_document_cmd", "inspect_document_cmd", "inspect"),
        ("preview_document_cmd", "preview_document_cmd", "screenshot"),
        (
            "create_template_document",
            "create_template_document",
            "new",
        ),
        (
            "list_document_parameters_cmd",
            "list_document_parameters_cmd",
            "params",
        ),
        (
            "set_document_parameter_cmd",
            "set_document_parameter_cmd",
            "opencad.patch_apply_document",
        ),
        (
            "undo_document_cmd",
            "undo_document_cmd",
            "opencad.history_undo_document",
        ),
        (
            "redo_document_cmd",
            "redo_document_cmd",
            "opencad.history_redo_document",
        ),
        ("open_viewport_cmd", "open_viewport_cmd", "view"),
        (
            "pick_document_cmd",
            "pick_document_cmd",
            "opencad.pick_document",
        ),
    ];

    let registration = TAURI_SOURCE
        .split("tauri::generate_handler![")
        .nth(1)
        .and_then(|source| source.split("])").next())
        .expect("Tauri command registration must be present");

    for (ui_command, handler, parity_route) in parity {
        assert!(
            UI_SOURCE.contains(&format!("invoke(\"{ui_command}\"")),
            "UI command '{ui_command}' is not invoked by main.js"
        );
        assert!(
            TAURI_SOURCE.contains(&format!("fn {handler}")),
            "UI command '{ui_command}' has no Tauri handler '{handler}'"
        );
        assert!(
            registration.contains(&format!("{handler},")),
            "Tauri handler '{handler}' is not registered in generate_handler"
        );
        assert!(
            CLI_SOURCE.contains(&format!("Some(\"{parity_route}\")"))
                || AGENT_SOURCE.contains(&format!("\"{parity_route}\"")),
            "UI command '{ui_command}' has no CLI/Agent parity route '{parity_route}'"
        );
    }

    assert!(
        !UI_SOURCE.contains("regenerate_document_cmd")
            && !TAURI_SOURCE.contains("regenerate_document_cmd")
            && !UI_SOURCE.contains("export_document_cmd")
            && !TAURI_SOURCE.contains("export_document_cmd"),
        "refresh must not expose removed regenerate/export Tauri commands"
    );
    assert!(
        !UI_SOURCE.contains("paramUndoStack")
            && !UI_SOURCE.contains("paramRedoStack")
            && !UI_SOURCE.contains("applyingParamHistory"),
        "UI must not maintain semantic inverse stacks; backend history is authoritative"
    );
    assert!(
        DESKTOP_API_DOC.contains("Refresh") && DESKTOP_API_DOC.contains("run_desktop_smoke"),
        "desktop API documentation must describe the actual refresh and smoke contracts"
    );
}

#[test]
fn ui_keeps_document_and_viewport_state_separate() {
    assert!(
        UI_SOURCE.contains("currentPath") && UI_SOURCE.contains("previewSync"),
        "UI must keep document path and viewport preview sync state explicit"
    );
    assert!(
        !UI_SOURCE.contains("currentPath.parameters")
            && !UI_SOURCE.contains("currentPath.feature_nodes"),
        "UI must not mutate the Design Graph directly"
    );
}
