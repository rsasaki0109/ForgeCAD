//! CLI surface regression tests for help formatting and parameter inspection.

use std::path::PathBuf;
use std::process::Command;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_opencad"))
        .args(args)
        .output()
        .expect("run opencad")
}

fn bracket_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/bracket.ocad.d")
}

#[test]
fn help_heading_and_commands_have_expected_indentation() {
    let output = run_cli(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(stdout.contains("\nCOMMANDS:\n"));
    assert!(stdout.contains("\n    help        Show this help\n"));
}

#[test]
fn params_json_lists_evaluated_width_with_explicit_units() {
    let fixture = bracket_fixture();
    let output = run_cli(&["params", fixture.to_str().expect("fixture path"), "--json"]);
    assert!(
        output.status.success(),
        "params failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("params JSON");
    let width = json
        .as_array()
        .expect("parameter rows")
        .iter()
        .find(|row| row["id"] == "param:width")
        .expect("width row");
    assert_eq!(width["expr"], "80 mm");
    assert_eq!(width["value_mm"], 80.0);
}
