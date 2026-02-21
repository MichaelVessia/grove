use std::process::Command;

#[test]
fn root_command_outputs_json_command_tree_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");

    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove".to_string())
    );
    assert_eq!(
        value["result"]["command"],
        serde_json::Value::String("grove".to_string())
    );
    assert!(value["result"]["commands"].is_array());
    assert!(value["next_actions"].is_array());
}
