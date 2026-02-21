use std::process::Command;

#[test]
fn agent_start_without_selector_returns_invalid_argument_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("agent")
        .arg("start")
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
        serde_json::Value::String("grove agent start".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("INVALID_ARGUMENT".to_string())
    );
}

#[test]
fn agent_start_missing_workspace_returns_workspace_not_found_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("agent")
        .arg("start")
        .arg("--workspace")
        .arg("definitely-missing-workspace")
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
        serde_json::Value::String("grove agent start".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("WORKSPACE_NOT_FOUND".to_string())
    );
}

#[test]
fn agent_stop_without_selector_returns_invalid_argument_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("agent")
        .arg("stop")
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
        serde_json::Value::String("grove agent stop".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("INVALID_ARGUMENT".to_string())
    );
}

#[test]
fn agent_stop_missing_workspace_returns_workspace_not_found_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("agent")
        .arg("stop")
        .arg("--workspace")
        .arg("definitely-missing-workspace")
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
        serde_json::Value::String("grove agent stop".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("WORKSPACE_NOT_FOUND".to_string())
    );
}
