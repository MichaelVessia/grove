use crate::domain::AgentType;
use crate::infrastructure::process::stderr_trimmed;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "workspace_lifecycle/create.rs"]
mod create;
#[path = "workspace_lifecycle/delete.rs"]
mod delete;
#[path = "workspace_lifecycle/facade.rs"]
pub mod facade;
#[path = "workspace_lifecycle/git_ops.rs"]
mod git_ops;
#[path = "workspace_lifecycle/markers.rs"]
mod markers;
#[path = "workspace_lifecycle/merge.rs"]
mod merge;
#[path = "workspace_lifecycle/paths.rs"]
mod paths;
#[path = "workspace_lifecycle/requests.rs"]
mod requests;
#[path = "workspace_lifecycle/update.rs"]
mod update;

const GROVE_DIR: &str = ".grove";
const GROVE_AGENT_MARKER_FILE: &str = ".grove/agent";
const GROVE_BASE_MARKER_FILE: &str = ".grove/base";
const GROVE_SETUP_SCRIPT_FILE: &str = ".grove/setup.sh";
const GROVE_GIT_EXCLUDE_ENTRIES: [&str; 1] = [".grove/"];
const ENV_FILES_TO_COPY: [&str; 4] = [
    ".env",
    ".env.local",
    ".env.development",
    ".env.development.local",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceLifecycleError {
    EmptyWorkspaceName,
    InvalidWorkspaceName,
    EmptyBaseBranch,
    EmptyExistingBranch,
    InvalidPullRequestNumber,
    RepoNameUnavailable,
    HomeDirectoryUnavailable,
    GitCommandFailed(String),
    Io(String),
}

pub fn workspace_lifecycle_error_message(error: &WorkspaceLifecycleError) -> String {
    match error {
        WorkspaceLifecycleError::EmptyWorkspaceName => "workspace name is required".to_string(),
        WorkspaceLifecycleError::InvalidWorkspaceName => {
            "workspace name must be [A-Za-z0-9_-]".to_string()
        }
        WorkspaceLifecycleError::EmptyBaseBranch => "base branch is required".to_string(),
        WorkspaceLifecycleError::EmptyExistingBranch => "existing branch is required".to_string(),
        WorkspaceLifecycleError::InvalidPullRequestNumber => {
            "pull request number is required".to_string()
        }
        WorkspaceLifecycleError::RepoNameUnavailable => "repo name unavailable".to_string(),
        WorkspaceLifecycleError::HomeDirectoryUnavailable => {
            "home directory unavailable".to_string()
        }
        WorkspaceLifecycleError::GitCommandFailed(message) => {
            format!("git command failed: {message}")
        }
        WorkspaceLifecycleError::Io(message) => format!("io error: {message}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMarkerError {
    MissingAgentMarker,
    MissingBaseMarker,
    InvalidAgentMarker(String),
    EmptyBaseBranch,
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchMode {
    NewBranch { base_branch: String },
    ExistingBranch { existing_branch: String },
    PullRequest { number: u64, base_branch: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceRequest {
    pub workspace_name: String,
    pub branch_mode: BranchMode,
    pub agent: AgentType,
}

impl CreateWorkspaceRequest {
    pub fn validate(&self) -> Result<(), WorkspaceLifecycleError> {
        if self.workspace_name.is_empty() {
            return Err(WorkspaceLifecycleError::EmptyWorkspaceName);
        }
        if !requests::workspace_name_is_valid(&self.workspace_name) {
            return Err(WorkspaceLifecycleError::InvalidWorkspaceName);
        }

        match &self.branch_mode {
            BranchMode::NewBranch { base_branch } => {
                if base_branch.trim().is_empty() {
                    return Err(WorkspaceLifecycleError::EmptyBaseBranch);
                }
            }
            BranchMode::ExistingBranch { existing_branch } => {
                if existing_branch.trim().is_empty() {
                    return Err(WorkspaceLifecycleError::EmptyExistingBranch);
                }
            }
            BranchMode::PullRequest {
                number,
                base_branch,
            } => {
                if *number == 0 {
                    return Err(WorkspaceLifecycleError::InvalidPullRequestNumber);
                }
                if base_branch.trim().is_empty() {
                    return Err(WorkspaceLifecycleError::EmptyBaseBranch);
                }
            }
        }

        Ok(())
    }

    pub fn branch_name(&self) -> String {
        match &self.branch_mode {
            BranchMode::NewBranch { .. } => self.workspace_name.clone(),
            BranchMode::ExistingBranch { existing_branch } => existing_branch.clone(),
            BranchMode::PullRequest { .. } => self.workspace_name.clone(),
        }
    }

    pub fn marker_base_branch(&self) -> String {
        match &self.branch_mode {
            BranchMode::NewBranch { base_branch } => base_branch.clone(),
            BranchMode::ExistingBranch { existing_branch } => existing_branch.clone(),
            BranchMode::PullRequest { base_branch, .. } => base_branch.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceResult {
    pub workspace_path: PathBuf,
    pub branch: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteWorkspaceRequest {
    pub project_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub workspace_name: String,
    pub branch: String,
    pub workspace_path: PathBuf,
    pub is_missing: bool,
    pub delete_local_branch: bool,
    pub kill_tmux_sessions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeWorkspaceRequest {
    pub project_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub workspace_name: String,
    pub workspace_branch: String,
    pub workspace_path: PathBuf,
    pub base_branch: String,
    pub cleanup_workspace: bool,
    pub cleanup_local_branch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateWorkspaceFromBaseRequest {
    pub project_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub workspace_name: String,
    pub workspace_branch: String,
    pub workspace_path: PathBuf,
    pub base_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMarkers {
    pub agent: AgentType,
    pub base_branch: String,
}

pub trait GitCommandRunner {
    fn run(&self, repo_root: &Path, args: &[String]) -> Result<(), String>;
}

pub trait SetupScriptRunner {
    fn run(&self, context: &SetupScriptContext) -> Result<(), String>;
}

pub trait SetupCommandRunner {
    fn run(&self, context: &SetupCommandContext, command: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupScriptContext {
    pub script_path: PathBuf,
    pub main_worktree_path: PathBuf,
    pub workspace_path: PathBuf,
    pub worktree_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupCommandContext {
    pub main_worktree_path: PathBuf,
    pub workspace_path: PathBuf,
    pub worktree_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSetupTemplate {
    pub auto_run_setup_commands: bool,
    pub commands: Vec<String>,
}

impl Default for WorkspaceSetupTemplate {
    fn default() -> Self {
        Self {
            auto_run_setup_commands: true,
            commands: Vec::new(),
        }
    }
}

pub struct CommandGitRunner;

impl GitCommandRunner for CommandGitRunner {
    fn run(&self, repo_root: &Path, args: &[String]) -> Result<(), String> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .output()
            .map_err(|error| error.to_string())?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = stderr_trimmed(&output);
        let message = if stderr.is_empty() {
            format!("git exited with status {}", output.status)
        } else {
            stderr
        };
        Err(message)
    }
}

pub struct CommandSetupScriptRunner;
pub struct CommandSetupCommandRunner;

impl SetupScriptRunner for CommandSetupScriptRunner {
    fn run(&self, context: &SetupScriptContext) -> Result<(), String> {
        let output = Command::new("bash")
            .arg(&context.script_path)
            .current_dir(&context.workspace_path)
            .env("MAIN_WORKTREE", &context.main_worktree_path)
            .env("WORKTREE_BRANCH", &context.worktree_branch)
            .env("WORKTREE_PATH", &context.workspace_path)
            .output()
            .map_err(|error| error.to_string())?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = stderr_trimmed(&output);
        let message = if stderr.is_empty() {
            format!(
                "setup script '{}' exited with status {}",
                context.script_path.display(),
                output.status
            )
        } else {
            stderr
        };
        Err(message)
    }
}

impl SetupCommandRunner for CommandSetupCommandRunner {
    fn run(&self, context: &SetupCommandContext, command: &str) -> Result<(), String> {
        let output = Command::new("bash")
            .arg("-lc")
            .arg(command)
            .current_dir(&context.workspace_path)
            .env("MAIN_WORKTREE", &context.main_worktree_path)
            .env("WORKTREE_BRANCH", &context.worktree_branch)
            .env("WORKTREE_PATH", &context.workspace_path)
            .output()
            .map_err(|error| error.to_string())?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = stderr_trimmed(&output);
        let message = if stderr.is_empty() {
            format!("setup command exited with status {}", output.status)
        } else {
            stderr
        };
        Err(message)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct NoopSessionTerminator;

impl facade::SessionTerminator for NoopSessionTerminator {
    fn stop_workspace_sessions(&self, _project_name: Option<&str>, _workspace_name: &str) {}
}

pub fn create_workspace(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    create::create_workspace(repo_root, request, git_runner, setup_script_runner)
}

pub fn create_workspace_with_template(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    setup_template: Option<&WorkspaceSetupTemplate>,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    create::create_workspace_with_template(
        repo_root,
        request,
        setup_template,
        git_runner,
        setup_script_runner,
        setup_command_runner,
    )
}

pub fn delete_workspace(request: DeleteWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    facade::delete_workspace(request, &NoopSessionTerminator)
}

pub(crate) fn delete_workspace_with_session_stopper(
    request: DeleteWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    delete::delete_workspace_with_session_stopper(request, stop_sessions)
}

pub fn merge_workspace(request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    facade::merge_workspace(request, &NoopSessionTerminator)
}

pub(crate) fn merge_workspace_with_session_stopper(
    request: MergeWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    merge::merge_workspace_with_session_stopper(request, stop_sessions)
}

pub fn update_workspace_from_base(
    request: UpdateWorkspaceFromBaseRequest,
) -> (Result<(), String>, Vec<String>) {
    facade::update_workspace_from_base(request, &NoopSessionTerminator)
}

pub(crate) fn update_workspace_from_base_with_session_stopper(
    request: UpdateWorkspaceFromBaseRequest,
    stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    update::update_workspace_from_base_with_session_stopper(request, stop_sessions)
}

pub(crate) fn workspace_directory_path(
    repo_root: &Path,
    workspace_name: &str,
) -> Result<PathBuf, WorkspaceLifecycleError> {
    paths::workspace_directory_path(repo_root, workspace_name)
}

pub(crate) fn ensure_grove_git_exclude_entries(
    repo_root: &Path,
) -> Result<(), WorkspaceLifecycleError> {
    let exclude_path = git_exclude_path(repo_root)?;
    let existing_content = match fs::read_to_string(&exclude_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(WorkspaceLifecycleError::Io(error.to_string())),
    };

    let mut missing_entries = Vec::new();
    for entry in GROVE_GIT_EXCLUDE_ENTRIES {
        if !existing_content.lines().any(|line| line.trim() == entry) {
            missing_entries.push(entry);
        }
    }

    if missing_entries.is_empty() {
        return Ok(());
    }

    let parent = exclude_path
        .parent()
        .ok_or_else(|| WorkspaceLifecycleError::Io("exclude path parent missing".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_path)
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;

    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file).map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    }

    for entry in missing_entries {
        writeln!(file, "{entry}")
            .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    }

    Ok(())
}

fn git_exclude_path(repo_root: &Path) -> Result<PathBuf, WorkspaceLifecycleError> {
    let dot_git = repo_root.join(".git");
    match fs::metadata(&dot_git) {
        Ok(metadata) if metadata.is_dir() => Ok(dot_git.join("info").join("exclude")),
        Ok(metadata) if metadata.is_file() => resolve_gitdir_file_exclude_path(repo_root, &dot_git),
        Ok(_) => Err(WorkspaceLifecycleError::Io(format!(
            "{} is neither file nor directory",
            dot_git.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(dot_git.join("info").join("exclude"))
        }
        Err(error) => Err(WorkspaceLifecycleError::Io(error.to_string())),
    }
}

fn resolve_gitdir_file_exclude_path(
    repo_root: &Path,
    dot_git_file: &Path,
) -> Result<PathBuf, WorkspaceLifecycleError> {
    let dot_git_content = fs::read_to_string(dot_git_file)
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    let gitdir_value = dot_git_content
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))
        .ok_or_else(|| {
            WorkspaceLifecycleError::Io(format!(
                "{} missing gitdir pointer",
                dot_git_file.display()
            ))
        })?;

    if gitdir_value.is_empty() {
        return Err(WorkspaceLifecycleError::Io(format!(
            "{} has empty gitdir pointer",
            dot_git_file.display()
        )));
    }

    let gitdir_path = PathBuf::from(gitdir_value);
    let resolved_gitdir = if gitdir_path.is_absolute() {
        gitdir_path
    } else {
        repo_root.join(gitdir_path)
    };
    Ok(resolved_gitdir.join("info").join("exclude"))
}

pub(crate) fn copy_env_files(
    main_worktree: &Path,
    workspace_path: &Path,
) -> Result<(), WorkspaceLifecycleError> {
    for file_name in ENV_FILES_TO_COPY {
        let source = main_worktree.join(file_name);
        if source.exists() {
            let target = workspace_path.join(file_name);
            fs::copy(&source, &target)
                .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
        }
    }
    Ok(())
}

pub fn read_workspace_markers(
    workspace_path: &Path,
) -> Result<WorkspaceMarkers, WorkspaceMarkerError> {
    markers::read_workspace_markers(workspace_path)
}

pub fn read_workspace_agent_marker(
    workspace_path: &Path,
) -> Result<AgentType, WorkspaceMarkerError> {
    markers::read_workspace_agent_marker(workspace_path)
}

pub fn write_workspace_agent_marker(
    workspace_path: &Path,
    agent: AgentType,
) -> Result<(), WorkspaceLifecycleError> {
    markers::write_workspace_agent_marker(workspace_path, agent)
}

pub fn write_workspace_base_marker(
    workspace_path: &Path,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    markers::write_workspace_base_marker(workspace_path, base_branch)
}

#[cfg(test)]
mod tests;
