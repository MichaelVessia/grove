use crate::application::agent_runtime::{
    kill_workspace_session_commands, kill_workspace_session_commands_for_existing_sessions,
};
use crate::application::session_cleanup::list_tmux_sessions;
use crate::infrastructure::process::{execute_command, stderr_trimmed};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "workspace_lifecycle/delete.rs"]
mod delete;
#[path = "workspace_lifecycle/git_ops.rs"]
mod git_ops;
#[path = "workspace_lifecycle/markers.rs"]
mod markers;
#[path = "workspace_lifecycle/merge.rs"]
mod merge;
#[path = "workspace_lifecycle/requests.rs"]
mod requests;
#[path = "workspace_lifecycle/update.rs"]
mod update;

const GROVE_DIR: &str = ".grove";
const GROVE_BASE_MARKER_FILE: &str = ".grove/base";
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
        WorkspaceLifecycleError::GitCommandFailed(message) => {
            format!("git command failed: {message}")
        }
        WorkspaceLifecycleError::Io(message) => format!("io error: {message}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMarkerError {
    MissingBaseMarker,
    EmptyBaseBranch,
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteWorkspaceRequest {
    pub task_slug: Option<String>,
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
    pub task_slug: Option<String>,
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
    pub task_slug: Option<String>,
    pub project_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub workspace_name: String,
    pub workspace_branch: String,
    pub workspace_path: PathBuf,
    pub base_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMarkers {
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

pub trait SessionTerminator {
    fn stop_workspace_sessions(
        &self,
        task_slug: Option<&str>,
        project_name: Option<&str>,
        workspace_name: &str,
    );
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

impl SessionTerminator for NoopSessionTerminator {
    fn stop_workspace_sessions(
        &self,
        _task_slug: Option<&str>,
        _project_name: Option<&str>,
        _workspace_name: &str,
    ) {
    }
}

/// Session terminator that kills tmux sessions via real commands.
/// Used by runtime callers that need actual session teardown.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeSessionTerminator;

impl SessionTerminator for RuntimeSessionTerminator {
    fn stop_workspace_sessions(
        &self,
        task_slug: Option<&str>,
        project_name: Option<&str>,
        workspace_name: &str,
    ) {
        let commands = match list_tmux_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    return;
                }
                let names: Vec<String> = sessions.into_iter().map(|session| session.name).collect();
                kill_workspace_session_commands_for_existing_sessions(
                    task_slug,
                    project_name,
                    workspace_name,
                    names.as_slice(),
                )
            }
            Err(_) => kill_workspace_session_commands(task_slug, project_name, workspace_name),
        };

        for command in commands {
            if command.is_empty() {
                continue;
            }
            let _ = execute_command(&command);
        }
    }
}

pub fn delete_workspace(request: DeleteWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    delete_workspace_with_terminator(request, &NoopSessionTerminator)
}

pub fn delete_workspace_with_terminator(
    request: DeleteWorkspaceRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    delete_workspace_with_session_stopper(request, |task_slug, project_name, workspace_name| {
        session_terminator.stop_workspace_sessions(task_slug, project_name, workspace_name);
    })
}

pub(crate) fn delete_workspace_with_session_stopper(
    request: DeleteWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    delete::delete_workspace_with_session_stopper(request, stop_sessions)
}

pub fn merge_workspace(request: MergeWorkspaceRequest) -> (Result<(), String>, Vec<String>) {
    merge_workspace_with_terminator(request, &NoopSessionTerminator)
}

pub fn merge_workspace_with_terminator(
    request: MergeWorkspaceRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    merge_workspace_with_session_stopper(request, |task_slug, project_name, workspace_name| {
        session_terminator.stop_workspace_sessions(task_slug, project_name, workspace_name);
    })
}

pub(crate) fn merge_workspace_with_session_stopper(
    request: MergeWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    merge::merge_workspace_with_session_stopper(request, stop_sessions)
}

pub fn update_workspace_from_base(
    request: UpdateWorkspaceFromBaseRequest,
) -> (Result<(), String>, Vec<String>) {
    update_workspace_from_base_with_terminator(request, &NoopSessionTerminator)
}

pub fn update_workspace_from_base_with_terminator(
    request: UpdateWorkspaceFromBaseRequest,
    session_terminator: &impl SessionTerminator,
) -> (Result<(), String>, Vec<String>) {
    update_workspace_from_base_with_session_stopper(
        request,
        |task_slug, project_name, workspace_name| {
            session_terminator.stop_workspace_sessions(task_slug, project_name, workspace_name);
        },
    )
}

pub(crate) fn update_workspace_from_base_with_session_stopper(
    request: UpdateWorkspaceFromBaseRequest,
    stop_sessions: impl Fn(Option<&str>, Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    update::update_workspace_from_base_with_session_stopper(request, stop_sessions)
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

pub fn write_workspace_base_marker(
    workspace_path: &Path,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    markers::write_workspace_base_marker(workspace_path, base_branch)
}

#[cfg(test)]
mod tests {
    use super::{
        DeleteWorkspaceRequest, MergeWorkspaceRequest, UpdateWorkspaceFromBaseRequest,
        WorkspaceLifecycleError, WorkspaceMarkerError, copy_env_files, delete_workspace,
        ensure_grove_git_exclude_entries, merge_workspace, merge_workspace_with_session_stopper,
        read_workspace_markers, update_workspace_from_base,
        update_workspace_from_base_with_session_stopper, workspace_lifecycle_error_message,
        write_workspace_base_marker,
    };
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    type SessionStopCall = (Option<String>, Option<String>, String);

    #[derive(Debug)]
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "grove-phase3-{label}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test dir should be created");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn ensure_git_exclude_entries_is_idempotent() {
        let temp = TestDir::new("git-exclude");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(repo_root.join(".git/info")).expect("git info dir should exist");
        fs::write(
            repo_root.join(".git/info/exclude"),
            ".grove/base\n/target/\n",
        )
        .expect("git exclude should be writable");

        ensure_grove_git_exclude_entries(&repo_root).expect("first ensure should succeed");
        ensure_grove_git_exclude_entries(&repo_root).expect("second ensure should succeed");

        let git_exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))
            .expect("git exclude should exist");
        assert_eq!(count_line(&git_exclude, ".grove/"), 1);
    }

    #[test]
    fn ensure_git_exclude_entries_does_not_modify_gitignore() {
        let temp = TestDir::new("gitignore-untouched");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        fs::write(repo_root.join(".gitignore"), "/dist/\n").expect(".gitignore should be writable");

        ensure_grove_git_exclude_entries(&repo_root).expect("ensure should succeed");

        let gitignore = fs::read_to_string(repo_root.join(".gitignore"))
            .expect(".gitignore should still exist");
        assert_eq!(gitignore, "/dist/\n");

        let git_exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))
            .expect("git exclude should exist");
        assert_eq!(count_line(&git_exclude, ".grove/"), 1);
    }

    #[test]
    fn ensure_git_exclude_entries_supports_gitdir_pointer_file() {
        let temp = TestDir::new("gitdir-file");
        let repo_root = temp.path.join("grove");
        let git_dir = temp.path.join("actual-git");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        fs::create_dir_all(git_dir.join("info")).expect("git info dir should exist");
        fs::write(repo_root.join(".git"), "gitdir: ../actual-git\n")
            .expect(".git pointer file should be writable");

        ensure_grove_git_exclude_entries(&repo_root).expect("ensure should succeed");

        let git_exclude =
            fs::read_to_string(git_dir.join("info/exclude")).expect("git exclude should exist");
        assert_eq!(count_line(&git_exclude, ".grove/"), 1);
    }

    #[test]
    fn copy_env_files_only_copies_known_env_files() {
        let temp = TestDir::new("copy-env");
        let main_worktree = temp.path.join("grove");
        let workspace = temp.path.join("grove-feature-x");
        fs::create_dir_all(&main_worktree).expect("main worktree should exist");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        fs::write(main_worktree.join(".env"), "ROOT=1\n").expect(".env should be writable");
        fs::write(main_worktree.join(".env.development.local"), "DEV=1\n")
            .expect(".env.development.local should be writable");
        fs::write(main_worktree.join(".env.production"), "PROD=1\n")
            .expect(".env.production should be writable");

        copy_env_files(&main_worktree, &workspace).expect("copy should succeed");

        assert_eq!(
            fs::read_to_string(workspace.join(".env")).expect(".env should copy"),
            "ROOT=1\n"
        );
        assert_eq!(
            fs::read_to_string(workspace.join(".env.development.local"))
                .expect(".env.development.local should copy"),
            "DEV=1\n"
        );
        assert!(!workspace.join(".env.production").exists());
    }

    #[test]
    fn read_workspace_markers_requires_base_marker() {
        let temp = TestDir::new("markers-missing-base");
        let workspace = temp.path.join("grove-feature-z");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        fs::create_dir_all(workspace.join(".grove")).expect(".grove should exist");

        assert_eq!(
            read_workspace_markers(&workspace),
            Err(WorkspaceMarkerError::MissingBaseMarker)
        );
    }

    #[test]
    fn read_workspace_markers_rejects_empty_base_marker() {
        let temp = TestDir::new("markers-empty-base");
        let workspace = temp.path.join("grove-feature-z");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        fs::create_dir_all(workspace.join(".grove")).expect(".grove should exist");
        fs::write(workspace.join(".grove/base"), "\n").expect("base marker should be writable");

        assert_eq!(
            read_workspace_markers(&workspace),
            Err(WorkspaceMarkerError::EmptyBaseBranch)
        );
    }

    #[test]
    fn write_workspace_base_marker_writes_expected_value() {
        let temp = TestDir::new("base-marker-write");
        let workspace = temp.path.join("grove");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        write_workspace_base_marker(&workspace, "main").expect("write should succeed");
        assert_eq!(
            fs::read_to_string(workspace.join(".grove/base"))
                .expect("marker should be readable")
                .trim(),
            "main"
        );

        write_workspace_base_marker(&workspace, "develop").expect("write should succeed");
        assert_eq!(
            fs::read_to_string(workspace.join(".grove/base"))
                .expect("marker should be readable")
                .trim(),
            "develop"
        );
    }

    #[test]
    fn workspace_lifecycle_error_messages_are_user_friendly() {
        assert_eq!(
            workspace_lifecycle_error_message(&WorkspaceLifecycleError::InvalidWorkspaceName),
            "workspace name must be [A-Za-z0-9_-]"
        );
        assert_eq!(
            workspace_lifecycle_error_message(&WorkspaceLifecycleError::GitCommandFailed(
                "boom".to_string()
            )),
            "git command failed: boom"
        );
    }

    #[test]
    fn delete_workspace_prunes_missing_worktree() {
        let temp = TestDir::new("delete-prune");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let request = DeleteWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "feature-x".to_string(),
            branch: "feature-x".to_string(),
            workspace_path: temp.path.join("grove-feature-x"),
            is_missing: true,
            delete_local_branch: false,
            kill_tmux_sessions: false,
        };

        let (result, warnings) = delete_workspace(request);
        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty());
    }

    #[test]
    fn delete_workspace_records_branch_delete_failure_as_warning() {
        let temp = TestDir::new("delete-branch-warning");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let request = DeleteWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "feature-y".to_string(),
            branch: "missing-branch".to_string(),
            workspace_path: temp.path.join("grove-feature-y"),
            is_missing: true,
            delete_local_branch: true,
            kill_tmux_sessions: false,
        };

        let (result, warnings) = delete_workspace(request);
        assert_eq!(result, Ok(()));
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("local branch: git branch delete failed:"),
            "unexpected warning: {}",
            warnings[0]
        );
    }

    #[test]
    fn delete_workspace_without_project_path_uses_current_directory() {
        let temp = TestDir::new("delete-cwd-fallback");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let workspace_path = temp.path.join("grove-feature-z");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-z",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );

        let original_cwd = std::env::current_dir().expect("cwd should be available");
        std::env::set_current_dir(&repo_root).expect("should switch cwd to repo");
        let request = DeleteWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: None,
            workspace_name: "feature-z".to_string(),
            branch: "feature-z".to_string(),
            workspace_path: workspace_path.clone(),
            is_missing: false,
            delete_local_branch: true,
            kill_tmux_sessions: false,
        };
        let (result, warnings) = delete_workspace(request);
        std::env::set_current_dir(&original_cwd).expect("should restore cwd");

        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert!(!workspace_path.exists(), "workspace path should be removed");
    }

    #[test]
    fn merge_workspace_merges_branch_and_cleans_up_when_requested() {
        let temp = TestDir::new("merge-cleanup");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-merge");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-merge",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("feature.txt"), "merged\n")
            .expect("feature file should exist");
        run_git(&workspace_path, &["add", "feature.txt"]);
        run_git(&workspace_path, &["commit", "-m", "add feature"]);

        let request = MergeWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "feature-merge".to_string(),
            workspace_branch: "feature-merge".to_string(),
            workspace_path: workspace_path.clone(),
            base_branch: base_branch.clone(),
            cleanup_workspace: true,
            cleanup_local_branch: true,
        };

        let (result, warnings) = merge_workspace(request);
        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert!(
            !workspace_path.exists(),
            "workspace path should be removed after cleanup"
        );

        let merged_file = repo_root.join("feature.txt");
        assert!(
            merged_file.exists(),
            "merged file should exist in base worktree after merge"
        );
        let branch_list = git_stdout(&repo_root, &["branch", "--list", "feature-merge"]);
        assert!(
            branch_list.trim().is_empty(),
            "feature branch should be removed from local refs"
        );
    }

    #[test]
    fn merge_workspace_requires_clean_base_and_workspace_worktrees() {
        let temp = TestDir::new("merge-dirty");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-dirty");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-dirty",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("dirty.txt"), "dirty\n").expect("dirty file should exist");

        let workspace_dirty_request = MergeWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "feature-dirty".to_string(),
            workspace_branch: "feature-dirty".to_string(),
            workspace_path: workspace_path.clone(),
            base_branch: base_branch.clone(),
            cleanup_workspace: false,
            cleanup_local_branch: false,
        };

        let (workspace_result, workspace_warnings) = merge_workspace(workspace_dirty_request);
        assert!(workspace_warnings.is_empty());
        let workspace_error = workspace_result.expect_err("dirty workspace should block merge");
        assert!(
            workspace_error.contains("workspace worktree has uncommitted changes"),
            "unexpected workspace error: {workspace_error}"
        );

        fs::write(repo_root.join("base-dirty.txt"), "dirty\n")
            .expect("base dirty file should exist");
        run_git(&workspace_path, &["add", "dirty.txt"]);
        run_git(&workspace_path, &["commit", "-m", "clean workspace branch"]);

        let base_dirty_request = MergeWorkspaceRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "feature-dirty".to_string(),
            workspace_branch: "feature-dirty".to_string(),
            workspace_path: workspace_path.clone(),
            base_branch,
            cleanup_workspace: false,
            cleanup_local_branch: false,
        };

        let (base_result, base_warnings) = merge_workspace(base_dirty_request);
        assert!(base_warnings.is_empty());
        let base_error = base_result.expect_err("dirty base should block merge");
        assert!(
            base_error.contains("base worktree has uncommitted changes"),
            "unexpected base error: {base_error}"
        );
    }

    #[test]
    fn merge_workspace_failure_does_not_stop_sessions() {
        let temp = TestDir::new("merge-failure-no-session-stop");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-no-stop");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-no-stop",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("dirty.txt"), "dirty\n").expect("dirty file should exist");

        let stop_calls: RefCell<Vec<SessionStopCall>> = RefCell::new(Vec::new());
        let request = MergeWorkspaceRequest {
            task_slug: Some("feature-no-stop".to_string()),
            project_name: Some("grove-project".to_string()),
            project_path: Some(repo_root),
            workspace_name: "feature-no-stop".to_string(),
            workspace_branch: "feature-no-stop".to_string(),
            workspace_path,
            base_branch,
            cleanup_workspace: true,
            cleanup_local_branch: false,
        };

        let (result, warnings) =
            merge_workspace_with_session_stopper(request, |task_slug, project, workspace| {
                stop_calls.borrow_mut().push((
                    task_slug.map(ToOwned::to_owned),
                    project.map(ToOwned::to_owned),
                    workspace.to_string(),
                ));
            });

        assert!(warnings.is_empty());
        let error = result.expect_err("dirty workspace should block merge");
        assert!(
            error.contains("workspace worktree has uncommitted changes"),
            "unexpected error: {error}"
        );
        assert!(
            stop_calls.borrow().is_empty(),
            "merge failure should not stop sessions"
        );
    }

    #[test]
    fn merge_workspace_cleanup_stops_sessions_after_success() {
        let temp = TestDir::new("merge-cleanup-session-stop");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-cleanup-stop");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-cleanup-stop",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("feature.txt"), "merged\n")
            .expect("feature file should exist");
        run_git(&workspace_path, &["add", "feature.txt"]);
        run_git(&workspace_path, &["commit", "-m", "add feature"]);

        let stop_calls: RefCell<Vec<SessionStopCall>> = RefCell::new(Vec::new());
        let request = MergeWorkspaceRequest {
            task_slug: Some("feature-cleanup-stop".to_string()),
            project_name: Some("grove-project".to_string()),
            project_path: Some(repo_root),
            workspace_name: "feature-cleanup-stop".to_string(),
            workspace_branch: "feature-cleanup-stop".to_string(),
            workspace_path: workspace_path.clone(),
            base_branch,
            cleanup_workspace: true,
            cleanup_local_branch: false,
        };

        let (result, warnings) =
            merge_workspace_with_session_stopper(request, |task_slug, project, workspace| {
                stop_calls.borrow_mut().push((
                    task_slug.map(ToOwned::to_owned),
                    project.map(ToOwned::to_owned),
                    workspace.to_string(),
                ));
            });

        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert!(
            !workspace_path.exists(),
            "workspace path should be removed after cleanup"
        );
        assert_eq!(
            *stop_calls.borrow(),
            vec![(
                Some("feature-cleanup-stop".to_string()),
                Some("grove-project".to_string()),
                "feature-cleanup-stop".to_string()
            )]
        );
    }

    #[test]
    fn update_workspace_from_base_never_stops_sessions() {
        let temp = TestDir::new("update-no-session-stop");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-update-no-stop");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-update-no-stop",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("dirty.txt"), "dirty\n").expect("dirty file should exist");

        let stop_calls: RefCell<Vec<SessionStopCall>> = RefCell::new(Vec::new());
        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: Some("feature-update-no-stop".to_string()),
            project_name: Some("grove-project".to_string()),
            project_path: Some(repo_root),
            workspace_name: "feature-update-no-stop".to_string(),
            workspace_branch: "feature-update-no-stop".to_string(),
            workspace_path,
            base_branch,
        };

        let (result, warnings) = update_workspace_from_base_with_session_stopper(
            request,
            |task_slug, project, workspace| {
                stop_calls.borrow_mut().push((
                    task_slug.map(ToOwned::to_owned),
                    project.map(ToOwned::to_owned),
                    workspace.to_string(),
                ));
            },
        );
        assert!(warnings.is_empty());
        let error = result.expect_err("dirty workspace should block update");
        assert!(
            error.contains("workspace worktree has uncommitted changes"),
            "unexpected error: {error}"
        );
        assert!(
            stop_calls.borrow().is_empty(),
            "workspace update should never stop sessions"
        );
    }

    #[test]
    fn update_workspace_from_base_merges_base_into_workspace_branch() {
        let temp = TestDir::new("update-from-base");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-sync");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-sync",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );

        fs::write(repo_root.join("base-change.txt"), "from base\n")
            .expect("base file should exist");
        run_git(&repo_root, &["add", "base-change.txt"]);
        run_git(&repo_root, &["commit", "-m", "base change"]);

        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root),
            workspace_name: "feature-sync".to_string(),
            workspace_branch: "feature-sync".to_string(),
            workspace_path: workspace_path.clone(),
            base_branch,
        };

        let (result, warnings) = update_workspace_from_base(request);
        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty());
        assert!(
            workspace_path.join("base-change.txt").exists(),
            "workspace should receive base branch changes"
        );
    }

    #[test]
    fn update_workspace_from_base_requires_clean_workspace_worktree() {
        let temp = TestDir::new("update-from-base-dirty");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let workspace_path = temp.path.join("grove-feature-sync-dirty");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-sync-dirty",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );
        fs::write(workspace_path.join("dirty.txt"), "dirty\n").expect("dirty file should exist");

        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root),
            workspace_name: "feature-sync-dirty".to_string(),
            workspace_branch: "feature-sync-dirty".to_string(),
            workspace_path,
            base_branch,
        };

        let (result, warnings) = update_workspace_from_base(request);
        assert!(warnings.is_empty());
        let error = result.expect_err("dirty workspace should block update");
        assert!(
            error.contains("workspace worktree has uncommitted changes"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn update_workspace_from_base_pulls_main_workspace_from_origin() {
        let temp = TestDir::new("update-base-from-origin");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let base_branch = current_branch(&repo_root);
        let origin = temp.path.join("origin.git");
        let origin_arg = origin.to_string_lossy().to_string();
        run_git(&temp.path, &["init", "--bare", origin_arg.as_str()]);
        run_git(
            &repo_root,
            &["remote", "add", "origin", origin_arg.as_str()],
        );
        run_git(&repo_root, &["push", "-u", "origin", base_branch.as_str()]);

        let upstream_clone = temp.path.join("upstream");
        let upstream_clone_arg = upstream_clone.to_string_lossy().to_string();
        run_git(
            &temp.path,
            &["clone", origin_arg.as_str(), upstream_clone_arg.as_str()],
        );
        run_git(
            &upstream_clone,
            &["config", "user.email", "grove-tests@example.com"],
        );
        run_git(&upstream_clone, &["config", "user.name", "Grove Tests"]);

        fs::write(upstream_clone.join("from-upstream.txt"), "upstream\n")
            .expect("upstream file should be writable");
        run_git(&upstream_clone, &["add", "from-upstream.txt"]);
        run_git(&upstream_clone, &["commit", "-m", "upstream change"]);
        run_git(&upstream_clone, &["push", "origin", base_branch.as_str()]);
        assert!(
            !repo_root.join("from-upstream.txt").exists(),
            "base workspace should not have upstream file before pull"
        );

        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root.clone()),
            workspace_name: "grove".to_string(),
            workspace_branch: base_branch.clone(),
            workspace_path: repo_root.clone(),
            base_branch: base_branch.clone(),
        };

        let (result, warnings) = update_workspace_from_base(request);
        assert_eq!(result, Ok(()));
        assert!(warnings.is_empty());
        assert!(
            repo_root.join("from-upstream.txt").exists(),
            "base workspace should receive upstream changes"
        );
    }

    #[test]
    fn update_workspace_from_base_rejects_matching_branch_for_non_main_workspace() {
        let temp = TestDir::new("update-from-base-matching-branch");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        init_git_repo(&repo_root);

        let workspace_path = temp.path.join("grove-feature-sync");
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                "feature-sync",
                workspace_path.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );

        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: None,
            project_name: None,
            project_path: Some(repo_root),
            workspace_name: "feature-sync".to_string(),
            workspace_branch: "feature-sync".to_string(),
            workspace_path,
            base_branch: "feature-sync".to_string(),
        };

        let (result, warnings) = update_workspace_from_base(request);
        assert!(warnings.is_empty());
        assert_eq!(
            result,
            Err("workspace branch matches base branch".to_string())
        );
    }

    fn init_git_repo(repo_root: &Path) {
        run_git(repo_root, &["init"]);
        run_git(
            repo_root,
            &["config", "user.email", "grove-tests@example.com"],
        );
        run_git(repo_root, &["config", "user.name", "Grove Tests"]);
        fs::write(repo_root.join("README.md"), "hello\n").expect("README should be writable");
        run_git(repo_root, &["add", "README.md"]);
        run_git(repo_root, &["commit", "-m", "initial commit"]);
    }

    fn current_branch(repo_root: &Path) -> String {
        git_stdout(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
            .trim()
            .to_string()
    }

    fn git_stdout(repo_root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout should be utf8")
    }

    fn run_git(repo_root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn count_line(content: &str, target: &str) -> usize {
        content.lines().filter(|line| line.trim() == target).count()
    }
}
