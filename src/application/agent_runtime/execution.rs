use crate::domain::Workspace;

use super::{CommandExecutionMode, LaunchRequest, SessionExecutionResult, ShellLaunchRequest};

pub fn start_workspace_with_mode(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    super::execute_launch_request_with_result_for_mode(request, mode)
}

pub fn stop_workspace_with_mode(
    workspace: &Workspace,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    super::execute_stop_workspace_with_result_for_mode(workspace, mode)
}

pub fn execute_shell_launch_request_for_mode(
    request: &ShellLaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> (String, Result<(), String>) {
    super::execute_shell_launch_request_for_mode(request, mode)
}
