use std::path::Path;

use crate::application::agent_runtime::kill_workspace_session_commands;
use crate::application::workspace_lifecycle::facade;
use crate::application::workspace_lifecycle::facade::SessionTerminator;
use crate::application::workspace_lifecycle::{
    CreateWorkspaceRequest, CreateWorkspaceResult, DeleteWorkspaceRequest, GitCommandRunner,
    MergeWorkspaceRequest, SetupCommandRunner, SetupScriptRunner, UpdateWorkspaceFromBaseRequest,
    WorkspaceLifecycleError, WorkspaceSetupTemplate,
};
use crate::domain::AgentType;
use crate::infrastructure::process::execute_command;

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeSessionTerminator;

impl SessionTerminator for RuntimeSessionTerminator {
    fn stop_workspace_sessions(&self, project_name: Option<&str>, workspace_name: &str) {
        for command in kill_workspace_session_commands(project_name, workspace_name) {
            if command.is_empty() {
                continue;
            }
            let _ = execute_command(&command);
        }
    }
}

pub(crate) trait WorkspaceService {
    fn create_workspace_with_template<G, S, C>(
        &self,
        repo_root: &Path,
        request: &CreateWorkspaceRequest,
        setup_template: Option<&WorkspaceSetupTemplate>,
        git_runner: &G,
        setup_script_runner: &S,
        setup_command_runner: &C,
    ) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError>
    where
        G: GitCommandRunner,
        S: SetupScriptRunner,
        C: SetupCommandRunner;

    fn delete_workspace(
        &self,
        request: DeleteWorkspaceRequest,
    ) -> (Result<(), String>, Vec<String>);

    fn merge_workspace(&self, request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>);

    fn update_workspace_from_base(
        &self,
        request: UpdateWorkspaceFromBaseRequest,
    ) -> (Result<(), String>, Vec<String>);

    fn workspace_lifecycle_error_message(&self, error: &WorkspaceLifecycleError) -> String;

    fn write_workspace_agent_marker(
        &self,
        workspace_path: &Path,
        agent: AgentType,
    ) -> Result<(), WorkspaceLifecycleError>;

    fn write_workspace_base_marker(
        &self,
        workspace_path: &Path,
        base_branch: &str,
    ) -> Result<(), WorkspaceLifecycleError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CommandWorkspaceService;

impl WorkspaceService for CommandWorkspaceService {
    fn create_workspace_with_template<G, S, C>(
        &self,
        repo_root: &Path,
        request: &CreateWorkspaceRequest,
        setup_template: Option<&WorkspaceSetupTemplate>,
        git_runner: &G,
        setup_script_runner: &S,
        setup_command_runner: &C,
    ) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError>
    where
        G: GitCommandRunner,
        S: SetupScriptRunner,
        C: SetupCommandRunner,
    {
        facade::create_workspace_with_template(
            repo_root,
            request,
            setup_template,
            git_runner,
            setup_script_runner,
            setup_command_runner,
        )
    }

    fn delete_workspace(
        &self,
        request: DeleteWorkspaceRequest,
    ) -> (Result<(), String>, Vec<String>) {
        facade::delete_workspace(request, &RuntimeSessionTerminator)
    }

    fn merge_workspace(&self, request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
        facade::merge_workspace(request, &RuntimeSessionTerminator)
    }

    fn update_workspace_from_base(
        &self,
        request: UpdateWorkspaceFromBaseRequest,
    ) -> (Result<(), String>, Vec<String>) {
        facade::update_workspace_from_base(request, &RuntimeSessionTerminator)
    }

    fn workspace_lifecycle_error_message(&self, error: &WorkspaceLifecycleError) -> String {
        facade::workspace_lifecycle_error_message(error)
    }

    fn write_workspace_agent_marker(
        &self,
        workspace_path: &Path,
        agent: AgentType,
    ) -> Result<(), WorkspaceLifecycleError> {
        facade::write_workspace_agent_marker(workspace_path, agent)
    }

    fn write_workspace_base_marker(
        &self,
        workspace_path: &Path,
        base_branch: &str,
    ) -> Result<(), WorkspaceLifecycleError> {
        facade::write_workspace_base_marker(workspace_path, base_branch)
    }
}

pub fn create_workspace_with_template<G, S, C>(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    setup_template: Option<&WorkspaceSetupTemplate>,
    git_runner: &G,
    setup_script_runner: &S,
    setup_command_runner: &C,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError>
where
    G: GitCommandRunner,
    S: SetupScriptRunner,
    C: SetupCommandRunner,
{
    CommandWorkspaceService.create_workspace_with_template(
        repo_root,
        request,
        setup_template,
        git_runner,
        setup_script_runner,
        setup_command_runner,
    )
}

pub fn delete_workspace(request: DeleteWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    CommandWorkspaceService.delete_workspace(request)
}

pub fn merge_workspace(request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    CommandWorkspaceService.merge_workspace(request)
}

pub fn update_workspace_from_base(
    request: UpdateWorkspaceFromBaseRequest,
) -> (Result<(), String>, Vec<String>) {
    CommandWorkspaceService.update_workspace_from_base(request)
}

pub fn workspace_lifecycle_error_message(error: &WorkspaceLifecycleError) -> String {
    CommandWorkspaceService.workspace_lifecycle_error_message(error)
}

pub fn write_workspace_agent_marker(
    workspace_path: &Path,
    agent: AgentType,
) -> Result<(), WorkspaceLifecycleError> {
    CommandWorkspaceService.write_workspace_agent_marker(workspace_path, agent)
}

pub fn write_workspace_base_marker(
    workspace_path: &Path,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    CommandWorkspaceService.write_workspace_base_marker(workspace_path, base_branch)
}
