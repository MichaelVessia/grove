use std::path::PathBuf;

use crate::domain::{AgentType, Workspace, WorkspaceStatus};

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
    pub prompt: Option<String>,
    pub pre_launch_command: Option<String>,
    pub skip_permissions: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStopRequest {
    pub context: RepoContext,
    pub selector: WorkspaceSelector,
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
