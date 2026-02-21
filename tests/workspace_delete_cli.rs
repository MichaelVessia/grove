use std::process::Command;

#[test]
fn workspace_delete_dry_run_with_workspace_path_returns_json_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("workspace")
        .arg("delete")
        .arg("--workspace-path")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--dry-run")
        .arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");

    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace delete".to_string())
    );
    assert_eq!(value["result"]["dry_run"], serde_json::Value::Bool(true));
}

#[test]
fn workspace_delete_without_selector_returns_invalid_argument_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("workspace")
        .arg("delete")
        .arg("--dry-run")
        .arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");

    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace delete".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("INVALID_ARGUMENT".to_string())
    );
}
