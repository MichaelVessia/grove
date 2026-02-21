use std::process::Command;

#[test]
fn workspace_list_returns_json_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("workspace")
        .arg("list")
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
        serde_json::Value::String("grove workspace list".to_string())
    );
    assert!(value["result"]["workspaces"].is_array());
}
