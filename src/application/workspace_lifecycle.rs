use crate::application::agent_runtime::kill_workspace_session_command;
use crate::config::MultiplexerKind;
use crate::domain::AgentType;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const GROVE_AGENT_MARKER_FILE: &str = ".grove-agent";
const GROVE_BASE_MARKER_FILE: &str = ".grove-base";
const GROVE_SETUP_SCRIPT_FILE: &str = ".grove-setup.sh";
const GROVE_GITIGNORE_ENTRIES: [&str; 4] = [
    ".grove-agent",
    ".grove-base",
    ".grove-start.sh",
    ".grove-setup.sh",
];
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
    RepoNameUnavailable,
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
        WorkspaceLifecycleError::RepoNameUnavailable => "repo name unavailable".to_string(),
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
        if !workspace_name_is_valid(&self.workspace_name) {
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
        }

        Ok(())
    }

    pub fn branch_name(&self) -> String {
        match &self.branch_mode {
            BranchMode::NewBranch { .. } => self.workspace_name.clone(),
            BranchMode::ExistingBranch { existing_branch } => existing_branch.clone(),
        }
    }

    pub fn marker_base_branch(&self) -> String {
        match &self.branch_mode {
            BranchMode::NewBranch { base_branch } => base_branch.clone(),
            BranchMode::ExistingBranch { existing_branch } => existing_branch.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupScriptContext {
    pub script_path: PathBuf,
    pub main_worktree_path: PathBuf,
    pub workspace_path: PathBuf,
    pub worktree_branch: String,
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

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git exited with status {}", output.status)
        } else {
            stderr
        };
        Err(message)
    }
}

pub struct CommandSetupScriptRunner;

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

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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

pub fn create_workspace(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    request.validate()?;

    let workspace_path = workspace_directory_path(repo_root, &request.workspace_name)?;
    let branch = request.branch_name();

    run_create_worktree_command(repo_root, &workspace_path, request, git_runner)?;

    fs::create_dir_all(&workspace_path)
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    write_workspace_markers(
        &workspace_path,
        request.agent,
        &request.marker_base_branch(),
    )?;
    ensure_grove_gitignore_entries(repo_root)?;
    copy_env_files(repo_root, &workspace_path)?;

    let mut warnings = Vec::new();
    let setup_script_path = repo_root.join(GROVE_SETUP_SCRIPT_FILE);
    if setup_script_path.exists() {
        let context = SetupScriptContext {
            script_path: setup_script_path,
            main_worktree_path: repo_root.to_path_buf(),
            workspace_path: workspace_path.clone(),
            worktree_branch: branch.clone(),
        };
        if let Err(error) = setup_script_runner.run(&context) {
            warnings.push(format!("setup script failed: {error}"));
        }
    }

    Ok(CreateWorkspaceResult {
        workspace_path,
        branch,
        warnings,
    })
}

pub fn delete_workspace(
    request: DeleteWorkspaceRequest,
    multiplexer: MultiplexerKind,
) -> (Result<(), String>, Vec<String>) {
    let mut warnings = Vec::new();
    let stop_session_command = kill_workspace_session_command(
        request.project_name.as_deref(),
        &request.workspace_name,
        multiplexer,
    );
    let _ = run_command(&stop_session_command);

    let repo_root = if let Some(project_path) = request.project_path {
        project_path
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd
    } else {
        return (
            Err("workspace project root unavailable".to_string()),
            warnings,
        );
    };

    if let Err(error) =
        run_delete_worktree_git(&repo_root, &request.workspace_path, request.is_missing)
    {
        return (Err(error), warnings);
    }

    if request.delete_local_branch
        && let Err(error) = run_delete_local_branch_git(&repo_root, &request.branch)
    {
        warnings.push(format!("local branch: {error}"));
    }

    (Ok(()), warnings)
}

pub(crate) fn workspace_directory_path(
    repo_root: &Path,
    workspace_name: &str,
) -> Result<PathBuf, WorkspaceLifecycleError> {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(WorkspaceLifecycleError::RepoNameUnavailable)?;

    let parent = repo_root.parent().unwrap_or(repo_root);
    Ok(parent.join(format!("{repo_name}-{workspace_name}")))
}

fn run_command(args: &[String]) -> Result<(), String> {
    let Some(program) = args.first() else {
        return Err("command is empty".to_string());
    };
    let output = Command::new(program)
        .args(&args[1..])
        .output()
        .map_err(|error| format!("{}: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        return Err(format!("{}: exit status {}", args.join(" "), output.status));
    }
    Err(format!("{}: {stderr}", args.join(" ")))
}

fn run_delete_worktree_git(
    repo_root: &Path,
    workspace_path: &Path,
    is_missing: bool,
) -> Result<(), String> {
    if is_missing {
        return run_git_command(repo_root, &["worktree".to_string(), "prune".to_string()])
            .map_err(|error| format!("git worktree prune failed: {error}"));
    }

    let workspace_path_arg = workspace_path.to_string_lossy().to_string();
    let remove_args = vec![
        "worktree".to_string(),
        "remove".to_string(),
        workspace_path_arg.clone(),
    ];
    if run_git_command(repo_root, &remove_args).is_ok() {
        return Ok(());
    }

    run_git_command(
        repo_root,
        &[
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            workspace_path_arg,
        ],
    )
    .map_err(|error| format!("git worktree remove failed: {error}"))
}

fn run_delete_local_branch_git(repo_root: &Path, branch: &str) -> Result<(), String> {
    let safe_args = vec!["branch".to_string(), "-d".to_string(), branch.to_string()];
    if run_git_command(repo_root, &safe_args).is_ok() {
        return Ok(());
    }

    run_git_command(
        repo_root,
        &["branch".to_string(), "-D".to_string(), branch.to_string()],
    )
    .map_err(|error| format!("git branch delete failed: {error}"))
}

fn run_git_command(repo_root: &Path, args: &[String]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        return Err(format!(
            "git {}: exit status {}",
            args.join(" "),
            output.status
        ));
    }
    Err(format!("git {}: {stderr}", args.join(" ")))
}

pub(crate) fn ensure_grove_gitignore_entries(
    repo_root: &Path,
) -> Result<(), WorkspaceLifecycleError> {
    let gitignore_path = repo_root.join(".gitignore");
    let existing_content = match fs::read_to_string(&gitignore_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(WorkspaceLifecycleError::Io(error.to_string())),
    };

    let mut missing_entries = Vec::new();
    for entry in GROVE_GITIGNORE_ENTRIES {
        if !existing_content.lines().any(|line| line.trim() == entry) {
            missing_entries.push(entry);
        }
    }

    if missing_entries.is_empty() {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)
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
    let agent = read_workspace_agent_marker(workspace_path)?;

    let base_marker_path = workspace_path.join(GROVE_BASE_MARKER_FILE);
    let base_marker_content = match fs::read_to_string(&base_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingBaseMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    let base_branch = base_marker_content.trim().to_string();
    if base_branch.is_empty() {
        return Err(WorkspaceMarkerError::EmptyBaseBranch);
    }

    Ok(WorkspaceMarkers { agent, base_branch })
}

pub fn read_workspace_agent_marker(
    workspace_path: &Path,
) -> Result<AgentType, WorkspaceMarkerError> {
    let agent_marker_path = workspace_path.join(GROVE_AGENT_MARKER_FILE);
    let agent_marker_content = match fs::read_to_string(&agent_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingAgentMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    parse_agent_marker(agent_marker_content.trim())
}

fn run_create_worktree_command(
    repo_root: &Path,
    workspace_path: &Path,
    request: &CreateWorkspaceRequest,
    git_runner: &impl GitCommandRunner,
) -> Result<(), WorkspaceLifecycleError> {
    let workspace_path_arg = workspace_path.to_string_lossy().to_string();
    let args = match &request.branch_mode {
        BranchMode::NewBranch { base_branch } => vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            request.branch_name(),
            workspace_path_arg,
            base_branch.clone(),
        ],
        BranchMode::ExistingBranch { existing_branch } => vec![
            "worktree".to_string(),
            "add".to_string(),
            workspace_path_arg,
            existing_branch.clone(),
        ],
    };

    git_runner
        .run(repo_root, &args)
        .map_err(WorkspaceLifecycleError::GitCommandFailed)
}

fn write_workspace_markers(
    workspace_path: &Path,
    agent: AgentType,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    write_workspace_agent_marker(workspace_path, agent)?;

    let base_marker_path = workspace_path.join(GROVE_BASE_MARKER_FILE);
    fs::write(base_marker_path, format!("{base_branch}\n"))
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))
}

pub fn write_workspace_agent_marker(
    workspace_path: &Path,
    agent: AgentType,
) -> Result<(), WorkspaceLifecycleError> {
    let agent_marker_path = workspace_path.join(GROVE_AGENT_MARKER_FILE);
    fs::write(
        agent_marker_path,
        format!("{}\n", agent_marker_value(agent)),
    )
    .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))
}

fn parse_agent_marker(value: &str) -> Result<AgentType, WorkspaceMarkerError> {
    match value {
        "claude" => Ok(AgentType::Claude),
        "codex" => Ok(AgentType::Codex),
        invalid => Err(WorkspaceMarkerError::InvalidAgentMarker(
            invalid.to_string(),
        )),
    }
}

fn agent_marker_value(agent: AgentType) -> &'static str {
    match agent {
        AgentType::Claude => "claude",
        AgentType::Codex => "codex",
    }
}

fn workspace_name_is_valid(name: &str) -> bool {
    name.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
}

#[cfg(test)]
mod tests;
