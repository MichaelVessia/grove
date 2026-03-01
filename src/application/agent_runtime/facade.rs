use std::path::Path;

use crate::domain::{AgentType, Workspace, WorkspaceStatus};

use super::{
    CommandExecutionMode, LaunchRequest, LivePreviewTarget, SessionActivity,
    SessionExecutionResult, ShellLaunchRequest, WorkspaceStatusTarget,
};

pub fn start_workspace_with_mode(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    super::execution::start_workspace_with_mode(request, mode)
}

pub fn stop_workspace_with_mode(
    workspace: &Workspace,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    super::execution::stop_workspace_with_mode(workspace, mode)
}

pub fn restart_workspace(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    super::restart::restart_workspace(workspace, skip_permissions, agent_env)
}

pub fn poll_targets(
    workspaces: &[Workspace],
    live_preview: Option<&LivePreviewTarget>,
) -> Vec<WorkspaceStatusTarget> {
    super::polling::workspace_status_targets_for_polling_with_live_preview(workspaces, live_preview)
}

pub(crate) fn detect_workspace_status(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &Path,
) -> WorkspaceStatus {
    super::status::detect_status_with_session_override(
        output,
        session_activity,
        is_main,
        has_live_session,
        supported_agent,
        agent,
        workspace_path,
    )
}

pub(crate) fn latest_attention_marker(agent: AgentType, workspace_path: &Path) -> Option<String> {
    super::status::latest_assistant_attention_marker(agent, workspace_path)
}

pub fn launch_request_for_workspace(
    workspace: &Workspace,
    prompt: Option<String>,
    workspace_init_command: Option<String>,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> LaunchRequest {
    super::launch_plan::launch_request_for_workspace(
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
    super::launch_plan::shell_launch_request_for_workspace(
        workspace,
        session_name,
        command,
        workspace_init_command,
        capture_cols,
        capture_rows,
    )
}

pub fn execute_shell_launch_request_for_mode(
    request: &ShellLaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> (String, Result<(), String>) {
    super::execution::execute_shell_launch_request_for_mode(request, mode)
}

pub fn execute_launch_request_with_result_for_mode(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    start_workspace_with_mode(request, mode)
}

pub fn execute_stop_workspace_with_result_for_mode(
    workspace: &Workspace,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    stop_workspace_with_mode(workspace, mode)
}

pub fn execute_restart_workspace_in_pane_with_result(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    restart_workspace(workspace, skip_permissions, agent_env)
}

pub fn workspace_status_targets_for_polling_with_live_preview(
    workspaces: &[Workspace],
    live_preview: Option<&LivePreviewTarget>,
) -> Vec<WorkspaceStatusTarget> {
    poll_targets(workspaces, live_preview)
}

pub(crate) fn detect_status_with_session_override(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &Path,
) -> WorkspaceStatus {
    detect_workspace_status(
        output,
        session_activity,
        is_main,
        has_live_session,
        supported_agent,
        agent,
        workspace_path,
    )
}

pub(crate) fn latest_assistant_attention_marker(
    agent: AgentType,
    workspace_path: &Path,
) -> Option<String> {
    latest_attention_marker(agent, workspace_path)
}
