use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::application::agent_runtime::kill_task_session_commands;
use crate::domain::{AgentType, Task, Worktree};
use crate::infrastructure::config::RepositoryConfig;
use crate::infrastructure::paths::tasks_root;
use crate::infrastructure::process::{execute_command, stderr_trimmed};
use crate::infrastructure::task_manifest::encode_task_manifest;

use crate::application::workspace_lifecycle::{
    CommandGitRunner, GitCommandRunner, SetupCommandRunner, SetupScriptRunner,
};

#[path = "task_lifecycle/create.rs"]
mod create;
#[path = "task_lifecycle/delete.rs"]
mod delete;

const GROVE_SETUP_SCRIPT_FILE: &str = ".grove/setup.sh";
const TASK_MANIFEST_FILE: &str = ".grove/task.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskLifecycleError {
    EmptyTaskName,
    InvalidTaskName,
    EmptyRepositories,
    HomeDirectoryUnavailable,
    RepositoryNameUnavailable,
    BaseBranchDetectionFailed(String),
    TaskInvalid(String),
    TaskManifest(String),
    GitCommandFailed(String),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskBranchSource {
    BaseBranch,
    PullRequest { number: u64 },
}

pub fn task_lifecycle_error_message(error: &TaskLifecycleError) -> String {
    match error {
        TaskLifecycleError::EmptyTaskName => "task name is required".to_string(),
        TaskLifecycleError::InvalidTaskName => "task name must be [A-Za-z0-9_-]".to_string(),
        TaskLifecycleError::EmptyRepositories => "at least one repository is required".to_string(),
        TaskLifecycleError::HomeDirectoryUnavailable => "home directory unavailable".to_string(),
        TaskLifecycleError::RepositoryNameUnavailable => "repository name unavailable".to_string(),
        TaskLifecycleError::BaseBranchDetectionFailed(message) => {
            format!("base branch detection failed: {message}")
        }
        TaskLifecycleError::TaskInvalid(message) => format!("task invalid: {message}"),
        TaskLifecycleError::TaskManifest(message) => format!("task manifest error: {message}"),
        TaskLifecycleError::GitCommandFailed(message) => {
            format!("git command failed: {message}")
        }
        TaskLifecycleError::Io(message) => format!("io error: {message}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskRequest {
    pub task_name: String,
    pub repositories: Vec<RepositoryConfig>,
    pub agent: AgentType,
    pub branch_source: TaskBranchSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskResult {
    pub task_root: PathBuf,
    pub task: Task,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTaskRequest {
    pub task: Task,
    pub delete_local_branch: bool,
    pub kill_tmux_sessions: bool,
}

impl CreateTaskRequest {
    pub fn validate(&self) -> Result<(), TaskLifecycleError> {
        if self.task_name.trim().is_empty() {
            return Err(TaskLifecycleError::EmptyTaskName);
        }
        if !task_name_is_valid(self.task_name.as_str()) {
            return Err(TaskLifecycleError::InvalidTaskName);
        }
        if self.repositories.is_empty() {
            return Err(TaskLifecycleError::EmptyRepositories);
        }
        Ok(())
    }
}

pub fn create_task(
    request: &CreateTaskRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    let home_directory = dirs::home_dir().ok_or(TaskLifecycleError::HomeDirectoryUnavailable)?;
    let tasks_root = home_directory.join(".grove").join("tasks");
    create_task_in_root(
        tasks_root.as_path(),
        request,
        git_runner,
        setup_script_runner,
        setup_command_runner,
    )
}

pub fn create_task_in_root(
    tasks_root: &Path,
    request: &CreateTaskRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    create::create_task_in_root(
        tasks_root,
        request,
        git_runner,
        setup_script_runner,
        setup_command_runner,
    )
}

pub fn delete_task(request: DeleteTaskRequest) -> (Result<(), String>, Vec<String>) {
    let git_runner = CommandGitRunner;
    delete_task_with_runner(request, &git_runner)
}

pub fn delete_task_with_runner(
    request: DeleteTaskRequest,
    git_runner: &impl GitCommandRunner,
) -> (Result<(), String>, Vec<String>) {
    let manifest_tasks_root = tasks_root();
    delete_task_with_runner_in_manifest_root(request, git_runner, manifest_tasks_root.as_deref())
}

fn delete_task_with_runner_in_manifest_root(
    request: DeleteTaskRequest,
    git_runner: &impl GitCommandRunner,
    manifest_tasks_root: Option<&Path>,
) -> (Result<(), String>, Vec<String>) {
    delete::delete_task_with_runner(request, git_runner, stop_task_sessions, manifest_tasks_root)
}

fn task_name_is_valid(name: &str) -> bool {
    name.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
}

fn task_manifest_path(task_root: &Path) -> PathBuf {
    task_root.join(TASK_MANIFEST_FILE)
}

fn write_task_manifest(task_root: &Path, task: &Task) -> Result<(), TaskLifecycleError> {
    let manifest_path = task_manifest_path(task_root);
    let Some(manifest_parent) = manifest_path.parent() else {
        return Err(TaskLifecycleError::TaskManifest(
            "task manifest path missing parent".to_string(),
        ));
    };
    fs::create_dir_all(manifest_parent)
        .map_err(|error| TaskLifecycleError::Io(error.to_string()))?;
    let encoded = encode_task_manifest(task).map_err(TaskLifecycleError::TaskManifest)?;
    fs::write(manifest_path, encoded).map_err(|error| TaskLifecycleError::Io(error.to_string()))
}

fn repo_directory_name(repository: &RepositoryConfig) -> Result<String, TaskLifecycleError> {
    repository
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or(TaskLifecycleError::RepositoryNameUnavailable)
}

fn resolve_repository_base_branch(
    repository: &RepositoryConfig,
) -> Result<String, TaskLifecycleError> {
    let configured_base_branch = repository.defaults.base_branch.trim();
    if !configured_base_branch.is_empty() {
        return Ok(configured_base_branch.to_string());
    }

    detect_repository_base_branch(repository.path.as_path())?.ok_or_else(|| {
        TaskLifecycleError::BaseBranchDetectionFailed(format!(
            "could not resolve base branch for project '{}'",
            repository.name
        ))
    })
}

fn detect_repository_base_branch(repo_root: &Path) -> Result<Option<String>, TaskLifecycleError> {
    if let Some(remote_head) = git_optional_stdout(
        repo_root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )? {
        let branch = remote_head
            .strip_prefix("origin/")
            .unwrap_or(remote_head.as_str())
            .trim();
        if !branch.is_empty() {
            return Ok(Some(branch.to_string()));
        }
    }

    if let Some(current_branch) = git_optional_stdout(repo_root, &["branch", "--show-current"])? {
        let branch = current_branch.trim();
        if !branch.is_empty() {
            return Ok(Some(branch.to_string()));
        }
    }

    if git_branch_exists(repo_root, "main")? {
        return Ok(Some("main".to_string()));
    }
    if git_branch_exists(repo_root, "master")? {
        return Ok(Some("master".to_string()));
    }

    Ok(None)
}

fn git_branch_exists(repo_root: &Path, branch: &str) -> Result<bool, TaskLifecycleError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()
        .map_err(|error| TaskLifecycleError::BaseBranchDetectionFailed(error.to_string()))?;
    if output.status.success() {
        return Ok(true);
    }
    let stderr = stderr_trimmed(&output);
    if stderr.contains("not a git repository") {
        return Err(TaskLifecycleError::BaseBranchDetectionFailed(stderr));
    }
    Ok(false)
}

fn git_optional_stdout(
    repo_root: &Path,
    args: &[&str],
) -> Result<Option<String>, TaskLifecycleError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|error| TaskLifecycleError::BaseBranchDetectionFailed(error.to_string()))?;
    if !output.status.success() {
        let stderr = stderr_trimmed(&output);
        if stderr.contains("not a git repository") {
            return Err(TaskLifecycleError::BaseBranchDetectionFailed(stderr));
        }
        return Ok(None);
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| TaskLifecycleError::BaseBranchDetectionFailed(error.to_string()))?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed.to_string()))
}

fn create_task_domain(
    task_name: &str,
    task_root: &Path,
    worktrees: Vec<Worktree>,
) -> Result<Task, TaskLifecycleError> {
    Task::try_new(
        task_name.to_string(),
        task_name.to_string(),
        task_root.to_path_buf(),
        task_name.to_string(),
        worktrees,
    )
    .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))
}

fn stop_task_sessions(task: &Task) {
    for command in kill_task_session_commands(task) {
        let _ = execute_command(command.as_slice());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CreateTaskRequest, DeleteTaskRequest, TaskBranchSource, create_task_in_root,
        delete_task_with_runner_in_manifest_root, detect_repository_base_branch,
        repo_directory_name, task_manifest_path,
    };
    use crate::application::workspace_lifecycle::{
        GitCommandRunner, SetupCommandContext, SetupCommandRunner, SetupScriptContext,
        SetupScriptRunner,
    };
    use crate::domain::AgentType;
    use crate::infrastructure::config::{ProjectDefaults, RepositoryConfig};
    use crate::infrastructure::process::stderr_trimmed;
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
                "grove-task-lifecycle-{label}-{}-{timestamp}",
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

    #[derive(Default)]
    struct StubGitRunner {
        calls: RefCell<Vec<(PathBuf, Vec<String>)>>,
    }

    impl StubGitRunner {
        fn calls(&self) -> Vec<(PathBuf, Vec<String>)> {
            self.calls.borrow().clone()
        }
    }

    impl GitCommandRunner for StubGitRunner {
        fn run(&self, repo_root: &Path, args: &[String]) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push((repo_root.to_path_buf(), args.to_vec()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct StubSetupRunner;

    impl SetupScriptRunner for StubSetupRunner {
        fn run(&self, _context: &SetupScriptContext) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct StubSetupCommandRunner;

    impl SetupCommandRunner for StubSetupCommandRunner {
        fn run(&self, _context: &SetupCommandContext, _command: &str) -> Result<(), String> {
            Ok(())
        }
    }

    fn repository(path: PathBuf) -> RepositoryConfig {
        let mut defaults = ProjectDefaults::default();
        defaults.base_branch = "main".to_string();
        RepositoryConfig {
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| "repo".to_string()),
            path,
            defaults,
        }
    }

    #[test]
    fn repo_directory_name_uses_repository_path_basename() {
        let repository = repository(PathBuf::from("/repos/terraform-fastly"));
        assert_eq!(
            repo_directory_name(&repository).expect("repo directory should resolve"),
            "terraform-fastly"
        );
    }

    #[test]
    fn create_task_builds_one_worktree_per_repository_under_task_root() {
        let temp = TestDir::new("create");
        let tasks_root = temp.path.join("tasks");
        let flohome = temp.path.join("repos").join("flohome");
        let fastly = temp.path.join("repos").join("terraform-fastly");
        fs::create_dir_all(&flohome).expect("flohome repo should exist");
        fs::create_dir_all(&fastly).expect("fastly repo should exist");
        fs::write(flohome.join(".env"), "APP=1\n").expect("env should write");
        fs::write(fastly.join(".env.local"), "FASTLY=1\n").expect("env local should write");

        let request = CreateTaskRequest {
            task_name: "flohome-launch".to_string(),
            repositories: vec![repository(flohome.clone()), repository(fastly.clone())],
            agent: AgentType::Codex,
            branch_source: TaskBranchSource::BaseBranch,
        };
        let git = StubGitRunner::default();
        let setup = StubSetupRunner;
        let setup_command = StubSetupCommandRunner;

        let result =
            create_task_in_root(tasks_root.as_path(), &request, &git, &setup, &setup_command)
                .expect("task should create");

        assert_eq!(result.task_root, tasks_root.join("flohome-launch"));
        assert_eq!(result.task.worktrees.len(), 2);
        assert!(result.task.worktrees[0].path.starts_with(&result.task_root));
        assert!(result.task.worktrees[1].path.starts_with(&result.task_root));
        assert!(task_manifest_path(&result.task_root).exists());
        assert_eq!(
            fs::read_to_string(result.task.worktrees[0].path.join(".grove/base"))
                .expect("base marker should exist")
                .trim(),
            "main"
        );
        assert_eq!(
            fs::read_to_string(result.task.worktrees[0].path.join(".env"))
                .expect("env should copy"),
            "APP=1\n"
        );
        assert_eq!(
            fs::read_to_string(result.task.worktrees[1].path.join(".env.local"))
                .expect("env local should copy"),
            "FASTLY=1\n"
        );
        assert_eq!(
            git.calls(),
            vec![
                (
                    flohome.clone(),
                    vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        "flohome-launch".to_string(),
                        tasks_root
                            .join("flohome-launch")
                            .join("flohome")
                            .to_string_lossy()
                            .to_string(),
                        "main".to_string(),
                    ],
                ),
                (
                    fastly.clone(),
                    vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        "flohome-launch".to_string(),
                        tasks_root
                            .join("flohome-launch")
                            .join("terraform-fastly")
                            .to_string_lossy()
                            .to_string(),
                        "main".to_string(),
                    ],
                ),
            ]
        );
    }

    #[test]
    fn create_task_resolves_base_branch_per_repository() {
        let temp = TestDir::new("create-per-project-base");
        let tasks_root = temp.path.join("tasks");
        let flohome = temp.path.join("repos").join("flohome");
        let fastly = temp.path.join("repos").join("terraform-fastly");
        fs::create_dir_all(&flohome).expect("flohome repo should exist");
        fs::create_dir_all(&fastly).expect("fastly repo should exist");

        let mut flohome_defaults = ProjectDefaults::default();
        flohome_defaults.base_branch = "develop".to_string();
        let mut fastly_defaults = ProjectDefaults::default();
        fastly_defaults.base_branch = "master".to_string();

        let request = CreateTaskRequest {
            task_name: "flohome-launch".to_string(),
            repositories: vec![
                RepositoryConfig {
                    name: "flohome".to_string(),
                    path: flohome.clone(),
                    defaults: flohome_defaults,
                },
                RepositoryConfig {
                    name: "terraform-fastly".to_string(),
                    path: fastly.clone(),
                    defaults: fastly_defaults,
                },
            ],
            agent: AgentType::Codex,
            branch_source: TaskBranchSource::BaseBranch,
        };
        let git = StubGitRunner::default();
        let setup = StubSetupRunner;
        let setup_command = StubSetupCommandRunner;

        let result =
            create_task_in_root(tasks_root.as_path(), &request, &git, &setup, &setup_command)
                .expect("task should create");

        assert_eq!(
            result.task.worktrees[0].base_branch.as_deref(),
            Some("develop")
        );
        assert_eq!(
            result.task.worktrees[1].base_branch.as_deref(),
            Some("master")
        );
        assert_eq!(
            fs::read_to_string(result.task.worktrees[0].path.join(".grove/base"))
                .expect("base marker should exist")
                .trim(),
            "develop"
        );
        assert_eq!(
            fs::read_to_string(result.task.worktrees[1].path.join(".grove/base"))
                .expect("base marker should exist")
                .trim(),
            "master"
        );
        assert_eq!(
            git.calls(),
            vec![
                (
                    flohome.clone(),
                    vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        "flohome-launch".to_string(),
                        tasks_root
                            .join("flohome-launch")
                            .join("flohome")
                            .to_string_lossy()
                            .to_string(),
                        "develop".to_string(),
                    ],
                ),
                (
                    fastly.clone(),
                    vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        "flohome-launch".to_string(),
                        tasks_root
                            .join("flohome-launch")
                            .join("terraform-fastly")
                            .to_string_lossy()
                            .to_string(),
                        "master".to_string(),
                    ],
                ),
            ]
        );
    }

    #[test]
    fn create_task_request_accepts_pull_request_source() {
        let request = CreateTaskRequest {
            task_name: "pr-123".to_string(),
            repositories: vec![repository(PathBuf::from("/repos/flohome"))],
            agent: AgentType::Codex,
            branch_source: TaskBranchSource::PullRequest { number: 123 },
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn create_task_in_root_fetches_pull_request_head_before_worktree_add() {
        let temp = TestDir::new("create-pr-head");
        let tasks_root = temp.path.join("tasks");
        let flohome = temp.path.join("repos").join("flohome");
        fs::create_dir_all(&flohome).expect("flohome repo should exist");

        let request = CreateTaskRequest {
            task_name: "pr-123".to_string(),
            repositories: vec![repository(flohome.clone())],
            agent: AgentType::Codex,
            branch_source: TaskBranchSource::PullRequest { number: 123 },
        };
        let git = StubGitRunner::default();
        let setup = StubSetupRunner;
        let setup_command = StubSetupCommandRunner;

        let result =
            create_task_in_root(tasks_root.as_path(), &request, &git, &setup, &setup_command)
                .expect("task should create");

        assert_eq!(result.task.worktrees.len(), 1);
        assert_eq!(
            fs::read_to_string(result.task.worktrees[0].path.join(".grove/base"))
                .expect("base marker should exist")
                .trim(),
            "main"
        );
        assert_eq!(
            git.calls(),
            vec![
                (
                    flohome.clone(),
                    vec![
                        "fetch".to_string(),
                        "origin".to_string(),
                        "pull/123/head".to_string(),
                    ],
                ),
                (
                    flohome,
                    vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        "pr-123".to_string(),
                        tasks_root
                            .join("pr-123")
                            .join("flohome")
                            .to_string_lossy()
                            .to_string(),
                        "FETCH_HEAD".to_string(),
                    ],
                ),
            ]
        );
    }

    #[test]
    fn detect_repository_base_branch_prefers_current_then_common_names() {
        let temp = TestDir::new("detect-base-branch");
        let repo = temp.path.join("repo");
        fs::create_dir_all(&repo).expect("repo should exist");

        run_git(&repo, &["init", "--initial-branch=master"]);
        run_git(&repo, &["config", "user.name", "Grove Tests"]);
        run_git(&repo, &["config", "user.email", "grove@example.com"]);
        fs::write(repo.join("README.md"), "hello\n").expect("readme should write");
        run_git(&repo, &["add", "README.md"]);
        run_git(&repo, &["commit", "-m", "init"]);

        assert_eq!(
            detect_repository_base_branch(&repo).expect("branch should resolve"),
            Some("master".to_string())
        );

        run_git(&repo, &["checkout", "-b", "feature"]);
        assert_eq!(
            detect_repository_base_branch(&repo).expect("branch should resolve"),
            Some("feature".to_string())
        );
    }

    fn run_git(repo_root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            stderr_trimmed(&output)
        );
    }

    #[test]
    fn delete_task_removes_all_worktrees_and_task_root() {
        let temp = TestDir::new("delete");
        let task_root = temp.path.join("tasks").join("flohome-launch");
        let flohome_repo = temp.path.join("repos").join("flohome");
        let fastly_repo = temp.path.join("repos").join("terraform-fastly");
        let flohome_path = task_root.join("flohome");
        let fastly_path = task_root.join("terraform-fastly");

        fs::create_dir_all(task_root.join(".grove")).expect("task manifest dir should exist");
        fs::create_dir_all(&flohome_repo).expect("flohome repo should exist");
        fs::create_dir_all(&fastly_repo).expect("fastly repo should exist");
        fs::create_dir_all(&flohome_path).expect("flohome worktree should exist");
        fs::create_dir_all(&fastly_path).expect("fastly worktree should exist");
        fs::write(task_manifest_path(&task_root), "name = 'flohome-launch'\n")
            .expect("task manifest should exist");

        let task = crate::domain::Task::try_new(
            "flohome-launch".to_string(),
            "flohome-launch".to_string(),
            task_root.clone(),
            "flohome-launch".to_string(),
            vec![
                crate::domain::Worktree::try_new(
                    "flohome".to_string(),
                    flohome_repo.clone(),
                    flohome_path.clone(),
                    "flohome-launch".to_string(),
                    AgentType::Codex,
                    crate::domain::WorkspaceStatus::Idle,
                )
                .expect("flohome worktree should be valid"),
                crate::domain::Worktree::try_new(
                    "terraform-fastly".to_string(),
                    fastly_repo.clone(),
                    fastly_path.clone(),
                    "flohome-launch".to_string(),
                    AgentType::Codex,
                    crate::domain::WorkspaceStatus::Idle,
                )
                .expect("fastly worktree should be valid"),
            ],
        )
        .expect("task should be valid");
        let git = StubGitRunner::default();

        let result = delete_task_with_runner_in_manifest_root(
            DeleteTaskRequest {
                task,
                delete_local_branch: true,
                kill_tmux_sessions: false,
            },
            &git,
            None,
        );

        assert_eq!(result.0, Ok(()));
        assert!(!task_root.exists());
        assert_eq!(
            git.calls(),
            vec![
                (
                    flohome_repo,
                    vec![
                        "worktree".to_string(),
                        "remove".to_string(),
                        flohome_path.to_string_lossy().to_string(),
                    ],
                ),
                (
                    temp.path.join("repos").join("flohome"),
                    vec![
                        "branch".to_string(),
                        "-d".to_string(),
                        "flohome-launch".to_string(),
                    ],
                ),
                (
                    fastly_repo,
                    vec![
                        "worktree".to_string(),
                        "remove".to_string(),
                        fastly_path.to_string_lossy().to_string(),
                    ],
                ),
                (
                    temp.path.join("repos").join("terraform-fastly"),
                    vec![
                        "branch".to_string(),
                        "-d".to_string(),
                        "flohome-launch".to_string(),
                    ],
                ),
            ]
        );
    }

    #[test]
    fn delete_task_removes_manifest_directory_for_migrated_task() {
        let temp = TestDir::new("delete-migrated");
        let manifest_tasks_root = temp.path.join("tasks");
        let manifest_task_root = manifest_tasks_root.join("web-monorepo-reviws-and-conflicts");
        let workspace_root = temp
            .path
            .join("workspaces")
            .join("web-monorepo-7b30ef98a8a6861d")
            .join("web-monorepo-reviws-and-conflicts");
        let repo_root = temp.path.join("repos").join("web-monorepo");

        fs::create_dir_all(manifest_task_root.join(".grove"))
            .expect("manifest task dir should exist");
        fs::create_dir_all(&workspace_root).expect("workspace root should exist");
        fs::create_dir_all(&repo_root).expect("repo root should exist");
        fs::write(
            task_manifest_path(&manifest_task_root),
            "name = 'migrated'\n",
        )
        .expect("task manifest should exist");

        let task = crate::domain::Task::try_new(
            "web-monorepo-reviws-and-conflicts".to_string(),
            "web-monorepo-reviws-and-conflicts".to_string(),
            workspace_root.clone(),
            "slug-issue".to_string(),
            vec![
                crate::domain::Worktree::try_new(
                    "monorepo".to_string(),
                    repo_root.clone(),
                    workspace_root.clone(),
                    "slug-issue".to_string(),
                    AgentType::Codex,
                    crate::domain::WorkspaceStatus::Idle,
                )
                .expect("worktree should be valid"),
            ],
        )
        .expect("task should be valid");
        let git = StubGitRunner::default();

        let result = delete_task_with_runner_in_manifest_root(
            DeleteTaskRequest {
                task,
                delete_local_branch: false,
                kill_tmux_sessions: false,
            },
            &git,
            Some(manifest_tasks_root.as_path()),
        );

        assert_eq!(result.0, Ok(()));
        assert!(!workspace_root.exists());
        assert!(!manifest_task_root.exists());
    }
}
