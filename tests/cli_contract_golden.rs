use std::path::Path;
use std::process::Command;

fn run_grove(args: &[&str], cwd: Option<&Path>) -> serde_json::Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grove"));
    if let Some(directory) = cwd {
        command.current_dir(directory);
    }
    let output = command
        .args(args)
        .output()
        .expect("grove binary should run");
    assert!(output.status.success(), "binary exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    serde_json::from_str(&stdout).expect("stdout should be JSON")
}

fn assert_next_actions_shape(value: &serde_json::Value) {
    let actions = value["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert!(!actions.is_empty(), "next_actions should not be empty");
    for action in actions {
        assert!(action["command"].is_string());
        assert!(action["description"].is_string());
    }
}

#[test]
fn workspace_list_defaults_repo_to_current_directory() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let value = run_grove(&["workspace", "list"], Some(manifest_dir));

    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace list".to_string())
    );
    assert_eq!(
        value["result"]["repo_root"],
        serde_json::Value::String(manifest_dir.display().to_string())
    );
    assert_next_actions_shape(&value);
}

#[test]
fn lifecycle_command_envelopes_keep_stable_shape_and_error_codes() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cases = [
        (vec![] as Vec<&str>, "grove", true, None),
        (
            vec![
                "workspace",
                "create",
                "--name",
                "golden-shape",
                "--base",
                "main",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove workspace create",
            true,
            None,
        ),
        (
            vec![
                "workspace",
                "edit",
                "--workspace",
                "missing",
                "--repo",
                manifest_dir,
            ],
            "grove workspace edit",
            false,
            Some("INVALID_ARGUMENT"),
        ),
        (
            vec![
                "workspace",
                "delete",
                "--workspace",
                "definitely-missing-workspace",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove workspace delete",
            false,
            Some("WORKSPACE_NOT_FOUND"),
        ),
        (
            vec![
                "workspace",
                "merge",
                "--workspace",
                "definitely-missing-workspace",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove workspace merge",
            false,
            Some("WORKSPACE_NOT_FOUND"),
        ),
        (
            vec![
                "workspace",
                "update",
                "--workspace",
                "definitely-missing-workspace",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove workspace update",
            false,
            Some("WORKSPACE_NOT_FOUND"),
        ),
        (
            vec![
                "agent",
                "start",
                "--workspace",
                "definitely-missing-workspace",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove agent start",
            false,
            Some("WORKSPACE_NOT_FOUND"),
        ),
        (
            vec![
                "agent",
                "stop",
                "--workspace",
                "definitely-missing-workspace",
                "--dry-run",
                "--repo",
                manifest_dir,
            ],
            "grove agent stop",
            false,
            Some("WORKSPACE_NOT_FOUND"),
        ),
    ];

    for (args, command_name, ok, expected_error_code) in cases {
        let value = run_grove(&args, None);
        assert_eq!(
            value["command"],
            serde_json::Value::String(command_name.to_string())
        );
        assert_eq!(value["ok"], serde_json::Value::Bool(ok));
        if let Some(error_code) = expected_error_code {
            assert_eq!(
                value["error"]["code"],
                serde_json::Value::String(error_code.to_string())
            );
        }
        assert_next_actions_shape(&value);
    }
}
