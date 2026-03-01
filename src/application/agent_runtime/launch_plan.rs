use crate::domain::Workspace;

use super::{LaunchRequest, ShellLaunchRequest};

pub fn launch_request_for_workspace(
    workspace: &Workspace,
    prompt: Option<String>,
    workspace_init_command: Option<String>,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> LaunchRequest {
    super::launch_request_for_workspace(
        workspace,
        prompt,
        workspace_init_command,
        skip_permissions,
        agent_env,
        capture_cols,
        capture_rows,
    )
}

pub fn shell_launch_request_for_workspace(
    workspace: &Workspace,
    session_name: String,
    command: String,
    workspace_init_command: Option<String>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> ShellLaunchRequest {
    super::shell_launch_request_for_workspace(
        workspace,
        session_name,
        command,
        workspace_init_command,
        capture_cols,
        capture_rows,
    )
}
