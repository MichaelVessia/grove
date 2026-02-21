use std::path::{Path, PathBuf};

use crate::application::agent_runtime::{
    CommandExecutionMode, execute_launch_request_with_result_for_mode,
    execute_stop_workspace_with_result_for_mode, launch_request_for_workspace,
    tmux_capture_error_indicates_missing_session, tmux_launch_error_indicates_duplicate_session,
};
use crate::application::workspace_lifecycle::{
    self, BranchMode, CommandGitRunner, CommandSetupCommandRunner, CommandSetupScriptRunner,
    WorkspaceLifecycleError, WorkspaceMarkerError, WorkspaceSetupTemplate, read_workspace_markers,
    workspace_lifecycle_error_message, write_workspace_agent_marker, write_workspace_base_marker,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{CommandGitAdapter, GitAdapter, GitAdapterError};
use crate::infrastructure::paths::refer_to_same_location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContext {
    pub repo_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSelector {
    Name(String),
    Path(PathBuf),
    NameAndPath { name: String, path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    Conflict,
    RuntimeFailure,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
}

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceListRequest {
    pub context: RepoContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceListResponse {
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreateRequest {
    pub context: RepoContext,
    pub name: String,
    pub base_branch: Option<String>,
    pub existing_branch: Option<String>,
    pub agent: Option<AgentType>,
    pub start: bool,
    pub dry_run: bool,
    pub setup_template: Option<WorkspaceCreateSetupTemplate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreateSetupTemplate {
    pub auto_run_setup_commands: bool,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEditRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub agent: Option<AgentType>,
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDeleteRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub delete_branch: bool,
    pub force_stop: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMergeRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub cleanup_workspace: bool,
    pub cleanup_branch: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceUpdateRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStartRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub workspace_hint: Option<Workspace>,
    pub prompt: Option<String>,
    pub pre_launch_command: Option<String>,
    pub skip_permissions: bool,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStopRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
    pub workspace_hint: Option<Workspace>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMutationResponse {
    pub workspace: Workspace,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMutationResponse {
    pub workspace: Workspace,
    pub status: WorkspaceStatus,
    pub warnings: Vec<String>,
}

pub trait LifecycleCommandService {
    fn workspace_list(&self, request: WorkspaceListRequest)
    -> CommandResult<WorkspaceListResponse>;

    fn workspace_create(
        &self,
        request: WorkspaceCreateRequest,
    ) -> CommandResult<WorkspaceMutationResponse>;

    fn workspace_edit(
        &self,
        request: WorkspaceEditRequest,
    ) -> CommandResult<WorkspaceMutationResponse>;

    fn workspace_delete(
        &self,
        request: WorkspaceDeleteRequest,
    ) -> CommandResult<WorkspaceMutationResponse>;

    fn workspace_merge(
        &self,
        request: WorkspaceMergeRequest,
    ) -> CommandResult<WorkspaceMutationResponse>;

    fn workspace_update(
        &self,
        request: WorkspaceUpdateRequest,
    ) -> CommandResult<WorkspaceMutationResponse>;

    fn agent_start(&self, request: AgentStartRequest) -> CommandResult<AgentMutationResponse>;

    fn agent_stop(&self, request: AgentStopRequest) -> CommandResult<AgentMutationResponse>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessLifecycleCommandService;

impl InProcessLifecycleCommandService {
    pub const fn new() -> Self {
        Self
    }

    fn list_workspaces_in_repo(&self, repo_root: &Path) -> CommandResult<Vec<Workspace>> {
        let adapter = CommandGitAdapter::for_repo(repo_root.to_path_buf());
        adapter
            .list_workspaces()
            .map_err(command_error_from_git_adapter)
    }

    fn resolve_workspace(
        &self,
        context: &RepoContext,
        selector: &WorkspaceSelector,
    ) -> CommandResult<Workspace> {
        let workspaces = self.list_workspaces_in_repo(&context.repo_root)?;
        resolve_workspace_from_inventory(&workspaces, selector)
    }

    fn read_workspace_base_branch(&self, workspace: &Workspace) -> CommandResult<String> {
        if let Some(base_branch) = workspace.base_branch.clone()
            && !base_branch.trim().is_empty()
        {
            return Ok(base_branch);
        }

        read_workspace_markers(&workspace.path)
            .map(|markers| markers.base_branch)
            .map_err(command_error_from_workspace_marker)
    }

    fn resolve_workspace_with_hint(
        &self,
        context: &RepoContext,
        selector: &WorkspaceSelector,
        workspace_hint: Option<Workspace>,
    ) -> CommandResult<Workspace> {
        match self.resolve_workspace(context, selector) {
            Ok(workspace) => Ok(workspace),
            Err(error) => {
                if let Some(workspace) = workspace_hint {
                    return Ok(workspace);
                }
                Err(error)
            }
        }
    }

    pub fn agent_start_for_mode(
        &self,
        request: AgentStartRequest,
        mode: CommandExecutionMode<'_>,
    ) -> CommandResult<AgentMutationResponse> {
        let mut workspace = self.resolve_workspace_with_hint(
            &request.context,
            &request.selector,
            request.workspace_hint.clone(),
        )?;
        if request.dry_run {
            return Ok(AgentMutationResponse {
                status: if workspace.status.is_running() {
                    workspace.status
                } else {
                    WorkspaceStatus::Active
                },
                workspace,
                warnings: Vec::new(),
            });
        }

        if workspace.status.is_running() {
            return Ok(AgentMutationResponse {
                workspace: workspace.clone(),
                status: workspace.status,
                warnings: Vec::new(),
            });
        }

        let launch_request = launch_request_for_workspace(
            &workspace,
            request.prompt,
            request.pre_launch_command,
            request.skip_permissions,
            request.capture_cols,
            request.capture_rows,
        );
        let start_result = execute_launch_request_with_result_for_mode(&launch_request, mode);
        if let Err(error) = start_result.result {
            if tmux_launch_error_indicates_duplicate_session(&error) {
                workspace.status = WorkspaceStatus::Active;
                return Ok(AgentMutationResponse {
                    workspace,
                    status: WorkspaceStatus::Active,
                    warnings: Vec::new(),
                });
            }

            return Err(CommandError {
                code: ErrorCode::RuntimeFailure,
                message: error,
            });
        }

        workspace.status = WorkspaceStatus::Active;
        Ok(AgentMutationResponse {
            workspace,
            status: WorkspaceStatus::Active,
            warnings: Vec::new(),
        })
    }

    pub fn agent_stop_for_mode(
        &self,
        request: AgentStopRequest,
        mode: CommandExecutionMode<'_>,
    ) -> CommandResult<AgentMutationResponse> {
        let mut workspace = self.resolve_workspace_with_hint(
            &request.context,
            &request.selector,
            request.workspace_hint.clone(),
        )?;
        let idle_status = if workspace.is_main {
            WorkspaceStatus::Main
        } else {
            WorkspaceStatus::Idle
        };

        if request.dry_run {
            return Ok(AgentMutationResponse {
                workspace,
                status: idle_status,
                warnings: Vec::new(),
            });
        }

        if !workspace.status.has_session() {
            return Ok(AgentMutationResponse {
                workspace,
                status: idle_status,
                warnings: Vec::new(),
            });
        }

        let stop_result = execute_stop_workspace_with_result_for_mode(&workspace, mode);
        if let Err(error) = stop_result.result {
            if tmux_capture_error_indicates_missing_session(&error) {
                workspace.status = idle_status;
                return Ok(AgentMutationResponse {
                    workspace,
                    status: idle_status,
                    warnings: Vec::new(),
                });
            }
            return Err(CommandError {
                code: ErrorCode::RuntimeFailure,
                message: error,
            });
        }

        workspace.status = idle_status;
        Ok(AgentMutationResponse {
            workspace,
            status: idle_status,
            warnings: Vec::new(),
        })
    }
}

impl LifecycleCommandService for InProcessLifecycleCommandService {
    fn workspace_list(
        &self,
        request: WorkspaceListRequest,
    ) -> CommandResult<WorkspaceListResponse> {
        let workspaces = self.list_workspaces_in_repo(&request.context.repo_root)?;
        Ok(WorkspaceListResponse { workspaces })
    }

    fn workspace_create(
        &self,
        request: WorkspaceCreateRequest,
    ) -> CommandResult<WorkspaceMutationResponse> {
        let setup_template =
            request
                .setup_template
                .as_ref()
                .map(|template| WorkspaceSetupTemplate {
                    auto_run_setup_commands: template.auto_run_setup_commands,
                    commands: template.commands.clone(),
                });
        let branch_mode = create_branch_mode(
            request.base_branch.as_deref(),
            request.existing_branch.as_deref(),
        )?;
        let agent = request.agent.unwrap_or(AgentType::Codex);
        let lifecycle_request = workspace_lifecycle::CreateWorkspaceRequest {
            workspace_name: request.name.clone(),
            branch_mode: branch_mode.clone(),
            agent,
        };

        if request.dry_run {
            let predicted = workspace_from_create_inputs(
                &request.context.repo_root,
                request.name.clone(),
                lifecycle_request.branch_name(),
                lifecycle_request.marker_base_branch(),
                agent,
            )?;
            return Ok(WorkspaceMutationResponse {
                workspace: predicted,
                warnings: Vec::new(),
            });
        }

        let git_runner = CommandGitRunner;
        let setup_script_runner = CommandSetupScriptRunner;
        let setup_command_runner = CommandSetupCommandRunner;
        let create_result = workspace_lifecycle::create_workspace_with_template(
            &request.context.repo_root,
            &lifecycle_request,
            setup_template.as_ref(),
            &git_runner,
            &setup_script_runner,
            &setup_command_runner,
        )
        .map_err(command_error_from_workspace_lifecycle)?;

        let mut workspace = self
            .list_workspaces_in_repo(&request.context.repo_root)?
            .into_iter()
            .find(|entry| refer_to_same_location(&entry.path, &create_result.workspace_path))
            .unwrap_or(workspace_from_create_inputs(
                &request.context.repo_root,
                request.name,
                create_result.branch.clone(),
                lifecycle_request.marker_base_branch(),
                agent,
            )?);

        let mut warnings = create_result.warnings;
        if request.start {
            let launch_request =
                launch_request_for_workspace(&workspace, None, None, false, None, None);
            let start_result = execute_launch_request_with_result_for_mode(
                &launch_request,
                CommandExecutionMode::Process,
            );
            if let Err(error) = start_result.result {
                return Err(CommandError {
                    code: ErrorCode::RuntimeFailure,
                    message: format!("workspace created but agent start failed: {error}"),
                });
            }
            workspace.status = WorkspaceStatus::Active;
        }

        if warnings.is_empty() {
            warnings = Vec::new();
        }

        Ok(WorkspaceMutationResponse {
            workspace,
            warnings,
        })
    }

    fn workspace_edit(
        &self,
        request: WorkspaceEditRequest,
    ) -> CommandResult<WorkspaceMutationResponse> {
        if request.agent.is_none() && request.base_branch.is_none() {
            return Err(CommandError {
                code: ErrorCode::InvalidArgument,
                message: "workspace edit requires --agent and/or --base".to_string(),
            });
        }

        let mut workspace = self.resolve_workspace(&request.context, &request.selector)?;
        if let Some(agent) = request.agent {
            write_workspace_agent_marker(&workspace.path, agent)
                .map_err(command_error_from_workspace_lifecycle)?;
            workspace.agent = agent;
        }

        if let Some(base_branch) = request.base_branch {
            if base_branch.trim().is_empty() {
                return Err(CommandError {
                    code: ErrorCode::InvalidArgument,
                    message: "base branch is required".to_string(),
                });
            }
            write_workspace_base_marker(&workspace.path, &base_branch)
                .map_err(command_error_from_workspace_lifecycle)?;
            workspace.base_branch = Some(base_branch);
        }

        Ok(WorkspaceMutationResponse {
            workspace,
            warnings: Vec::new(),
        })
    }

    fn workspace_delete(
        &self,
        request: WorkspaceDeleteRequest,
    ) -> CommandResult<WorkspaceMutationResponse> {
        let workspace = self.resolve_workspace(&request.context, &request.selector)?;

        if request.dry_run {
            return Ok(WorkspaceMutationResponse {
                workspace,
                warnings: Vec::new(),
            });
        }

        let lifecycle_request = workspace_lifecycle::DeleteWorkspaceRequest {
            project_name: workspace.project_name.clone(),
            project_path: Some(request.context.repo_root.clone()),
            workspace_name: workspace.name.clone(),
            branch: workspace.branch.clone(),
            workspace_path: workspace.path.clone(),
            is_missing: !workspace.path.exists(),
            delete_local_branch: request.delete_branch,
            kill_tmux_sessions: request.force_stop,
        };
        let (result, warnings) = workspace_lifecycle::delete_workspace(lifecycle_request);
        result.map_err(command_error_from_runtime_message)?;

        Ok(WorkspaceMutationResponse {
            workspace,
            warnings,
        })
    }

    fn workspace_merge(
        &self,
        request: WorkspaceMergeRequest,
    ) -> CommandResult<WorkspaceMutationResponse> {
        let workspace = self.resolve_workspace(&request.context, &request.selector)?;
        let base_branch = self.read_workspace_base_branch(&workspace)?;

        if request.dry_run {
            return Ok(WorkspaceMutationResponse {
                workspace,
                warnings: Vec::new(),
            });
        }

        let lifecycle_request = workspace_lifecycle::MergeWorkspaceRequest {
            project_name: workspace.project_name.clone(),
            project_path: Some(request.context.repo_root.clone()),
            workspace_name: workspace.name.clone(),
            workspace_branch: workspace.branch.clone(),
            workspace_path: workspace.path.clone(),
            base_branch,
            cleanup_workspace: request.cleanup_workspace,
            cleanup_local_branch: request.cleanup_branch,
        };
        let (result, warnings) = workspace_lifecycle::merge_workspace(lifecycle_request);
        result.map_err(command_error_from_runtime_message)?;

        Ok(WorkspaceMutationResponse {
            workspace,
            warnings,
        })
    }

    fn workspace_update(
        &self,
        request: WorkspaceUpdateRequest,
    ) -> CommandResult<WorkspaceMutationResponse> {
        let workspace = self.resolve_workspace(&request.context, &request.selector)?;
        let base_branch = self.read_workspace_base_branch(&workspace)?;

        if request.dry_run {
            return Ok(WorkspaceMutationResponse {
                workspace,
                warnings: Vec::new(),
            });
        }

        let lifecycle_request = workspace_lifecycle::UpdateWorkspaceFromBaseRequest {
            project_name: workspace.project_name.clone(),
            project_path: Some(request.context.repo_root.clone()),
            workspace_name: workspace.name.clone(),
            workspace_branch: workspace.branch.clone(),
            workspace_path: workspace.path.clone(),
            base_branch,
        };
        let (result, warnings) = workspace_lifecycle::update_workspace_from_base(lifecycle_request);
        result.map_err(command_error_from_runtime_message)?;

        Ok(WorkspaceMutationResponse {
            workspace,
            warnings,
        })
    }

    fn agent_start(&self, request: AgentStartRequest) -> CommandResult<AgentMutationResponse> {
        self.agent_start_for_mode(request, CommandExecutionMode::Process)
    }

    fn agent_stop(&self, request: AgentStopRequest) -> CommandResult<AgentMutationResponse> {
        self.agent_stop_for_mode(request, CommandExecutionMode::Process)
    }
}

fn resolve_workspace_from_inventory(
    workspaces: &[Workspace],
    selector: &WorkspaceSelector,
) -> CommandResult<Workspace> {
    match selector {
        WorkspaceSelector::Name(name) => workspaces
            .iter()
            .find(|workspace| workspace.name == *name)
            .cloned()
            .ok_or_else(|| CommandError {
                code: ErrorCode::NotFound,
                message: format!("workspace '{name}' was not found"),
            }),
        WorkspaceSelector::Path(path) => workspaces
            .iter()
            .find(|workspace| refer_to_same_location(&workspace.path, path))
            .cloned()
            .ok_or_else(|| CommandError {
                code: ErrorCode::NotFound,
                message: format!("workspace path '{}' was not found", path.display()),
            }),
        WorkspaceSelector::NameAndPath { name, path } => {
            let by_name = workspaces.iter().find(|workspace| workspace.name == *name);
            let by_path = workspaces
                .iter()
                .find(|workspace| refer_to_same_location(&workspace.path, path));
            match (by_name, by_path) {
                (Some(name_match), Some(path_match))
                    if refer_to_same_location(&name_match.path, &path_match.path) =>
                {
                    Ok(name_match.clone())
                }
                (Some(_), Some(_)) => Err(CommandError {
                    code: ErrorCode::InvalidArgument,
                    message: "workspace name/path selectors resolved to different workspaces"
                        .to_string(),
                }),
                _ => Err(CommandError {
                    code: ErrorCode::NotFound,
                    message: "workspace selector did not match any workspace".to_string(),
                }),
            }
        }
    }
}

fn create_branch_mode(
    base_branch: Option<&str>,
    existing_branch: Option<&str>,
) -> CommandResult<BranchMode> {
    match (base_branch, existing_branch) {
        (Some(_), Some(_)) => Err(CommandError {
            code: ErrorCode::InvalidArgument,
            message: "pass either --base or --existing-branch, not both".to_string(),
        }),
        (Some(base_branch), None) => Ok(BranchMode::NewBranch {
            base_branch: base_branch.to_string(),
        }),
        (None, Some(existing_branch)) => Ok(BranchMode::ExistingBranch {
            existing_branch: existing_branch.to_string(),
        }),
        (None, None) => Err(CommandError {
            code: ErrorCode::InvalidArgument,
            message: "workspace create requires --base or --existing-branch".to_string(),
        }),
    }
}

fn workspace_from_create_inputs(
    repo_root: &Path,
    workspace_name: String,
    branch: String,
    base_branch: String,
    agent: AgentType,
) -> CommandResult<Workspace> {
    let workspace_path = workspace_lifecycle::workspace_directory_path(repo_root, &workspace_name)
        .map_err(command_error_from_workspace_lifecycle)?;
    let mut workspace = Workspace::try_new(
        workspace_name,
        workspace_path,
        branch,
        None,
        agent,
        WorkspaceStatus::Idle,
        false,
    )
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: format!("workspace validation failed: {error:?}"),
    })?
    .with_base_branch(Some(base_branch));

    if let Some(project_name) = repo_root.file_name().and_then(|value| value.to_str()) {
        workspace =
            workspace.with_project_context(project_name.to_string(), repo_root.to_path_buf());
    }

    Ok(workspace)
}

fn command_error_from_workspace_lifecycle(error: WorkspaceLifecycleError) -> CommandError {
    let message = workspace_lifecycle_error_message(&error);
    match error {
        WorkspaceLifecycleError::EmptyWorkspaceName
        | WorkspaceLifecycleError::InvalidWorkspaceName
        | WorkspaceLifecycleError::EmptyBaseBranch
        | WorkspaceLifecycleError::EmptyExistingBranch => CommandError {
            code: ErrorCode::InvalidArgument,
            message,
        },
        WorkspaceLifecycleError::RepoNameUnavailable => CommandError {
            code: ErrorCode::NotFound,
            message,
        },
        WorkspaceLifecycleError::HomeDirectoryUnavailable
        | WorkspaceLifecycleError::GitCommandFailed(_)
        | WorkspaceLifecycleError::Io(_) => CommandError {
            code: ErrorCode::RuntimeFailure,
            message,
        },
    }
}

fn command_error_from_workspace_marker(error: WorkspaceMarkerError) -> CommandError {
    let message = match &error {
        WorkspaceMarkerError::MissingAgentMarker => "workspace agent marker is missing".to_string(),
        WorkspaceMarkerError::MissingBaseMarker => "workspace base marker is missing".to_string(),
        WorkspaceMarkerError::InvalidAgentMarker(value) => {
            format!("workspace agent marker is invalid: {value}")
        }
        WorkspaceMarkerError::EmptyBaseBranch => "workspace base marker is empty".to_string(),
        WorkspaceMarkerError::Io(details) => format!("workspace marker io error: {details}"),
    };

    match error {
        WorkspaceMarkerError::MissingAgentMarker | WorkspaceMarkerError::MissingBaseMarker => {
            CommandError {
                code: ErrorCode::NotFound,
                message,
            }
        }
        WorkspaceMarkerError::InvalidAgentMarker(_) | WorkspaceMarkerError::EmptyBaseBranch => {
            CommandError {
                code: ErrorCode::InvalidArgument,
                message,
            }
        }
        WorkspaceMarkerError::Io(_) => CommandError {
            code: ErrorCode::RuntimeFailure,
            message,
        },
    }
}

fn command_error_from_runtime_message(message: String) -> CommandError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("required") || lower.contains("matches base branch") {
        ErrorCode::InvalidArgument
    } else if lower.contains("not found")
        || lower.contains("path does not exist")
        || lower.contains("project root unavailable")
    {
        ErrorCode::NotFound
    } else if lower.contains("conflict")
        || lower.contains("uncommitted changes")
        || lower.contains("merge failed")
    {
        ErrorCode::Conflict
    } else {
        ErrorCode::RuntimeFailure
    };

    CommandError { code, message }
}

fn command_error_from_git_adapter(error: GitAdapterError) -> CommandError {
    let message = error.message();
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("not a git repository") {
        ErrorCode::NotFound
    } else {
        ErrorCode::RuntimeFailure
    };
    CommandError { code, message }
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorCode, WorkspaceSelector, create_branch_mode, resolve_workspace_from_inventory,
    };
    use crate::application::workspace_lifecycle::BranchMode;
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use std::path::PathBuf;

    fn test_workspace(name: &str, path: &str) -> Workspace {
        Workspace::try_new(
            name.to_string(),
            PathBuf::from(path),
            name.to_string(),
            None,
            AgentType::Codex,
            WorkspaceStatus::Idle,
            false,
        )
        .expect("workspace should be valid")
    }

    #[test]
    fn selector_resolution_accepts_name_and_path_for_same_workspace() {
        let workspaces = vec![
            test_workspace("feature-a", "/tmp/repo-feature-a"),
            test_workspace("feature-b", "/tmp/repo-feature-b"),
        ];
        let resolved = resolve_workspace_from_inventory(
            &workspaces,
            &WorkspaceSelector::NameAndPath {
                name: "feature-a".to_string(),
                path: PathBuf::from("/tmp/repo-feature-a"),
            },
        )
        .expect("selector should resolve");
        assert_eq!(resolved.name, "feature-a");
    }

    #[test]
    fn selector_resolution_rejects_name_and_path_mismatch() {
        let workspaces = vec![
            test_workspace("feature-a", "/tmp/repo-feature-a"),
            test_workspace("feature-b", "/tmp/repo-feature-b"),
        ];
        let error = resolve_workspace_from_inventory(
            &workspaces,
            &WorkspaceSelector::NameAndPath {
                name: "feature-a".to_string(),
                path: PathBuf::from("/tmp/repo-feature-b"),
            },
        )
        .expect_err("selector mismatch should fail");
        assert_eq!(error.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn selector_resolution_reports_not_found_for_missing_target() {
        let workspaces = vec![test_workspace("feature-a", "/tmp/repo-feature-a")];
        let error = resolve_workspace_from_inventory(
            &workspaces,
            &WorkspaceSelector::Name("feature-z".to_string()),
        )
        .expect_err("missing workspace should fail");
        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[test]
    fn create_branch_mode_requires_exactly_one_branch_strategy() {
        let both = create_branch_mode(Some("main"), Some("feature-a"))
            .expect_err("both branch strategies should fail");
        assert_eq!(both.code, ErrorCode::InvalidArgument);

        let none = create_branch_mode(None, None).expect_err("no branch strategy should fail");
        assert_eq!(none.code, ErrorCode::InvalidArgument);

        let base = create_branch_mode(Some("main"), None).expect("base branch should succeed");
        assert_eq!(
            base,
            BranchMode::NewBranch {
                base_branch: "main".to_string()
            }
        );

        let existing =
            create_branch_mode(None, Some("feature-a")).expect("existing branch should succeed");
        assert_eq!(
            existing,
            BranchMode::ExistingBranch {
                existing_branch: "feature-a".to_string()
            }
        );
    }
}
