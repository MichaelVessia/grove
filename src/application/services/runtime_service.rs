use std::path::Path;

use crate::application::agent_runtime::{
    CommandExecutionMode, LaunchRequest, LivePreviewTarget, SessionActivity,
    SessionExecutionResult, ShellLaunchRequest, WorkspaceStatusTarget,
    detect_status_with_session_override as runtime_detect_status_with_session_override,
    execute_launch_request_with_result_for_mode as runtime_execute_launch_request_with_result_for_mode,
    execute_restart_workspace_in_pane_with_result as runtime_execute_restart_workspace_in_pane_with_result,
    execute_shell_launch_request_for_mode as runtime_execute_shell_launch_request_for_mode,
    execute_stop_workspace_with_result_for_mode as runtime_execute_stop_workspace_with_result_for_mode,
    latest_assistant_attention_marker as runtime_latest_assistant_attention_marker,
    launch_request_for_workspace as runtime_launch_request_for_workspace,
    shell_launch_request_for_workspace as runtime_shell_launch_request_for_workspace,
    workspace_status_targets_for_polling_with_live_preview as runtime_workspace_status_targets_for_polling_with_live_preview,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};

pub(crate) struct RuntimeLaunchRequestInput<'a> {
    pub workspace: &'a Workspace,
    pub prompt: Option<String>,
    pub workspace_init_command: Option<String>,
    pub skip_permissions: bool,
    pub agent_env: Vec<(String, String)>,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

pub(crate) struct RuntimeStatusDetectionInput<'a> {
    pub output: &'a str,
    pub session_activity: SessionActivity,
    pub is_main: bool,
    pub has_live_session: bool,
    pub supported_agent: bool,
    pub agent: AgentType,
    pub workspace_path: &'a Path,
}

pub(crate) trait RuntimeService {
    fn launch_request_for_workspace(&self, request: RuntimeLaunchRequestInput<'_>)
    -> LaunchRequest;

    fn shell_launch_request_for_workspace(
        &self,
        workspace: &Workspace,
        session_name: String,
        command: String,
        workspace_init_command: Option<String>,
        capture_cols: Option<u16>,
        capture_rows: Option<u16>,
    ) -> ShellLaunchRequest;

    fn execute_shell_launch_request_for_mode<'a>(
        &self,
        request: &ShellLaunchRequest,
        mode: CommandExecutionMode<'a>,
    ) -> (String, Result<(), String>);

    fn execute_launch_request_with_result_for_mode<'a>(
        &self,
        request: &LaunchRequest,
        mode: CommandExecutionMode<'a>,
    ) -> SessionExecutionResult;

    fn execute_stop_workspace_with_result_for_mode<'a>(
        &self,
        workspace: &Workspace,
        mode: CommandExecutionMode<'a>,
    ) -> SessionExecutionResult;

    fn execute_restart_workspace_in_pane_with_result(
        &self,
        workspace: &Workspace,
        skip_permissions: bool,
        agent_env: Vec<(String, String)>,
    ) -> SessionExecutionResult;

    fn workspace_status_targets_for_polling_with_live_preview(
        &self,
        workspaces: &[Workspace],
        live_preview: Option<&LivePreviewTarget>,
    ) -> Vec<WorkspaceStatusTarget>;

    fn detect_status_with_session_override(
        &self,
        request: RuntimeStatusDetectionInput<'_>,
    ) -> WorkspaceStatus;

    fn latest_assistant_attention_marker(
        &self,
        agent: AgentType,
        workspace_path: &Path,
    ) -> Option<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CommandRuntimeService;

impl RuntimeService for CommandRuntimeService {
    fn launch_request_for_workspace(
        &self,
        request: RuntimeLaunchRequestInput<'_>,
    ) -> LaunchRequest {
        runtime_launch_request_for_workspace(
            request.workspace,
            request.prompt,
            request.workspace_init_command,
            request.skip_permissions,
            request.agent_env,
            request.capture_cols,
            request.capture_rows,
        )
    }

    fn shell_launch_request_for_workspace(
        &self,
        workspace: &Workspace,
        session_name: String,
        command: String,
        workspace_init_command: Option<String>,
        capture_cols: Option<u16>,
        capture_rows: Option<u16>,
    ) -> ShellLaunchRequest {
        runtime_shell_launch_request_for_workspace(
            workspace,
            session_name,
            command,
            workspace_init_command,
            capture_cols,
            capture_rows,
        )
    }

    fn execute_shell_launch_request_for_mode<'a>(
        &self,
        request: &ShellLaunchRequest,
        mode: CommandExecutionMode<'a>,
    ) -> (String, Result<(), String>) {
        runtime_execute_shell_launch_request_for_mode(request, mode)
    }

    fn execute_launch_request_with_result_for_mode<'a>(
        &self,
        request: &LaunchRequest,
        mode: CommandExecutionMode<'a>,
    ) -> SessionExecutionResult {
        runtime_execute_launch_request_with_result_for_mode(request, mode)
    }

    fn execute_stop_workspace_with_result_for_mode<'a>(
        &self,
        workspace: &Workspace,
        mode: CommandExecutionMode<'a>,
    ) -> SessionExecutionResult {
        runtime_execute_stop_workspace_with_result_for_mode(workspace, mode)
    }

    fn execute_restart_workspace_in_pane_with_result(
        &self,
        workspace: &Workspace,
        skip_permissions: bool,
        agent_env: Vec<(String, String)>,
    ) -> SessionExecutionResult {
        runtime_execute_restart_workspace_in_pane_with_result(
            workspace,
            skip_permissions,
            agent_env,
        )
    }

    fn workspace_status_targets_for_polling_with_live_preview(
        &self,
        workspaces: &[Workspace],
        live_preview: Option<&LivePreviewTarget>,
    ) -> Vec<WorkspaceStatusTarget> {
        runtime_workspace_status_targets_for_polling_with_live_preview(workspaces, live_preview)
    }

    fn detect_status_with_session_override(
        &self,
        request: RuntimeStatusDetectionInput<'_>,
    ) -> WorkspaceStatus {
        runtime_detect_status_with_session_override(
            request.output,
            request.session_activity,
            request.is_main,
            request.has_live_session,
            request.supported_agent,
            request.agent,
            request.workspace_path,
        )
    }

    fn latest_assistant_attention_marker(
        &self,
        agent: AgentType,
        workspace_path: &Path,
    ) -> Option<String> {
        runtime_latest_assistant_attention_marker(agent, workspace_path)
    }
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
    let request = RuntimeLaunchRequestInput {
        workspace,
        prompt,
        workspace_init_command,
        skip_permissions,
        agent_env,
        capture_cols,
        capture_rows,
    };
    CommandRuntimeService.launch_request_for_workspace(request)
}

pub fn shell_launch_request_for_workspace(
    workspace: &Workspace,
    session_name: String,
    command: String,
    workspace_init_command: Option<String>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> ShellLaunchRequest {
    CommandRuntimeService.shell_launch_request_for_workspace(
        workspace,
        session_name,
        command,
        workspace_init_command,
        capture_cols,
        capture_rows,
    )
}

pub fn execute_shell_launch_request_for_mode<'a>(
    request: &ShellLaunchRequest,
    mode: CommandExecutionMode<'a>,
) -> (String, Result<(), String>) {
    CommandRuntimeService.execute_shell_launch_request_for_mode(request, mode)
}

pub fn execute_launch_request_with_result_for_mode<'a>(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'a>,
) -> SessionExecutionResult {
    CommandRuntimeService.execute_launch_request_with_result_for_mode(request, mode)
}

pub fn execute_stop_workspace_with_result_for_mode<'a>(
    workspace: &Workspace,
    mode: CommandExecutionMode<'a>,
) -> SessionExecutionResult {
    CommandRuntimeService.execute_stop_workspace_with_result_for_mode(workspace, mode)
}

pub fn execute_restart_workspace_in_pane_with_result(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    CommandRuntimeService.execute_restart_workspace_in_pane_with_result(
        workspace,
        skip_permissions,
        agent_env,
    )
}

pub fn workspace_status_targets_for_polling_with_live_preview(
    workspaces: &[Workspace],
    live_preview: Option<&LivePreviewTarget>,
) -> Vec<WorkspaceStatusTarget> {
    CommandRuntimeService
        .workspace_status_targets_for_polling_with_live_preview(workspaces, live_preview)
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
    let request = RuntimeStatusDetectionInput {
        output,
        session_activity,
        is_main,
        has_live_session,
        supported_agent,
        agent,
        workspace_path,
    };
    CommandRuntimeService.detect_status_with_session_override(request)
}

pub(crate) fn latest_assistant_attention_marker(
    agent: AgentType,
    workspace_path: &Path,
) -> Option<String> {
    CommandRuntimeService.latest_assistant_attention_marker(agent, workspace_path)
}
