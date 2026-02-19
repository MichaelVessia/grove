use super::execute_command;

#[test]
fn execute_command_ignores_empty_commands() {
    let result = execute_command(&Vec::new());
    assert!(result.is_ok());
}

#[test]
fn execute_command_reports_exit_status_when_stderr_missing() {
    let command = vec!["sh".to_string(), "-lc".to_string(), "exit 7".to_string()];
    let result = execute_command(&command);
    let error_text = result.expect_err("command should fail").to_string();

    assert_eq!(
        error_text,
        "command failed: sh -lc exit 7; exit status exit status: 7"
    );
}

#[test]
fn execute_command_reports_stderr_when_available() {
    let command = vec![
        "sh".to_string(),
        "-lc".to_string(),
        "echo boom >&2; exit 1".to_string(),
    ];
    let result = execute_command(&command);
    let error_text = result.expect_err("command should fail").to_string();

    assert_eq!(
        error_text,
        "command failed: sh -lc echo boom >&2; exit 1; boom"
    );
}
