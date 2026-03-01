use std::path::Path;

use super::{
    CreateWorkspaceRequest, CreateWorkspaceResult, DeleteWorkspaceRequest, GitCommandRunner,
    MergeWorkspaceRequest, SetupCommandRunner, SetupScriptRunner, UpdateWorkspaceFromBaseRequest,
    WorkspaceLifecycleError, WorkspaceSetupTemplate,
};
use crate::domain::AgentType;

pub trait SessionTerminator {
    fn stop_workspace_sessions(&self, project_name: Option<&str>, workspace_name: &str);
}

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

pub fn delete_workspace(
    request: DeleteWorkspaceRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    super::delete_workspace_with_session_stopper(request, |project_name, workspace_name| {
        session_terminator.stop_workspace_sessions(project_name, workspace_name);
    })
}

pub fn merge_workspace(
    request: MergeWorkspaceRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    super::merge_workspace_with_session_stopper(request, |project_name, workspace_name| {
        session_terminator.stop_workspace_sessions(project_name, workspace_name);
    })
}

pub fn update_workspace_from_base(
    request: UpdateWorkspaceFromBaseRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    super::update_workspace_from_base_with_session_stopper(
        request,
        |project_name, workspace_name| {
            session_terminator.stop_workspace_sessions(project_name, workspace_name);
        },
    )
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
