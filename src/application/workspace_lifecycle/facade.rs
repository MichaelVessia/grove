use std::path::Path;

use super::{
    CreateWorkspaceRequest, CreateWorkspaceResult, DeleteWorkspaceRequest, GitCommandRunner,
    MergeWorkspaceRequest, SetupCommandRunner, SetupScriptRunner, UpdateWorkspaceFromBaseRequest,
    WorkspaceLifecycleError, WorkspaceSetupTemplate,
};
use crate::application::agent_runtime::kill_workspace_session_commands;
use crate::domain::AgentType;
use crate::infrastructure::process::execute_command;

pub fn create_workspace_with_template(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    setup_template: Option<&WorkspaceSetupTemplate>,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    super::create_workspace_with_template(
        repo_root,
        request,
        setup_template,
        git_runner,
        setup_script_runner,
        setup_command_runner,
    )
}

pub fn delete_workspace(request: DeleteWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    super::delete_workspace_with_session_stopper(request, stop_workspace_sessions)
}

pub fn merge_workspace(request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    super::merge_workspace_with_session_stopper(request, stop_workspace_sessions)
}

pub fn update_workspace_from_base(
    request: UpdateWorkspaceFromBaseRequest,
) -> (Result<(), String>, Vec<String>) {
    super::update_workspace_from_base_with_session_stopper(request, stop_workspace_sessions)
}

pub fn workspace_lifecycle_error_message(error: &WorkspaceLifecycleError) -> String {
    super::workspace_lifecycle_error_message(error)
}

pub fn write_workspace_agent_marker(
    workspace_path: &Path,
    agent: AgentType,
) -> Result<(), WorkspaceLifecycleError> {
    super::write_workspace_agent_marker(workspace_path, agent)
}

pub fn write_workspace_base_marker(
    workspace_path: &Path,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    super::write_workspace_base_marker(workspace_path, base_branch)
}

fn stop_workspace_sessions(project_name: Option<&str>, workspace_name: &str) {
    for command in kill_workspace_session_commands(project_name, workspace_name) {
        if command.is_empty() {
            continue;
        }
        let _ = execute_command(&command);
    }
}
