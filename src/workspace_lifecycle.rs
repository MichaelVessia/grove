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

pub(crate) fn ensure_grove_gitignore_entries(repo_root: &Path) -> Result<(), WorkspaceLifecycleError> {
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
    let agent_marker_path = workspace_path.join(GROVE_AGENT_MARKER_FILE);
    let agent_marker_content = match fs::read_to_string(&agent_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingAgentMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    let base_marker_path = workspace_path.join(GROVE_BASE_MARKER_FILE);
    let base_marker_content = match fs::read_to_string(&base_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingBaseMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    let agent = parse_agent_marker(agent_marker_content.trim())?;
    let base_branch = base_marker_content.trim().to_string();
    if base_branch.is_empty() {
        return Err(WorkspaceMarkerError::EmptyBaseBranch);
    }

    Ok(WorkspaceMarkers { agent, base_branch })
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
    let agent_marker_path = workspace_path.join(GROVE_AGENT_MARKER_FILE);
    fs::write(
        agent_marker_path,
        format!("{}\n", agent_marker_value(agent)),
    )
    .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;

    let base_marker_path = workspace_path.join(GROVE_BASE_MARKER_FILE);
    fs::write(base_marker_path, format!("{base_branch}\n"))
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
mod tests {
    use super::{
        BranchMode, CreateWorkspaceRequest, GitCommandRunner, SetupScriptContext,
        SetupScriptRunner, WorkspaceLifecycleError, WorkspaceMarkerError, copy_env_files,
        create_workspace, ensure_grove_gitignore_entries, read_workspace_markers,
        workspace_directory_path,
    };
    use crate::domain::AgentType;
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
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

    #[derive(Default)]
    struct StubGitRunner {
        calls: RefCell<Vec<Vec<String>>>,
        outcomes: RefCell<Vec<Result<(), String>>>,
    }

    impl StubGitRunner {
        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.borrow().clone()
        }
    }

    impl GitCommandRunner for StubGitRunner {
        fn run(&self, _repo_root: &Path, args: &[String]) -> Result<(), String> {
            self.calls.borrow_mut().push(args.to_vec());
            if self.outcomes.borrow().is_empty() {
                return Ok(());
            }
            self.outcomes.borrow_mut().remove(0)
        }
    }

    #[derive(Default)]
    struct StubSetupRunner {
        calls: RefCell<Vec<SetupScriptContext>>,
        outcomes: RefCell<Vec<Result<(), String>>>,
    }

    impl StubSetupRunner {
        fn with_outcomes(outcomes: Vec<Result<(), String>>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                outcomes: RefCell::new(outcomes),
            }
        }

        fn calls(&self) -> Vec<SetupScriptContext> {
            self.calls.borrow().clone()
        }
    }

    impl SetupScriptRunner for StubSetupRunner {
        fn run(&self, context: &SetupScriptContext) -> Result<(), String> {
            self.calls.borrow_mut().push(context.clone());
            if self.outcomes.borrow().is_empty() {
                return Ok(());
            }
            self.outcomes.borrow_mut().remove(0)
        }
    }

    #[test]
    fn create_request_validation_distinguishes_workspace_slug_and_existing_branch() {
        let invalid = CreateWorkspaceRequest {
            workspace_name: "feature auth".to_string(),
            branch_mode: BranchMode::NewBranch {
                base_branch: "main".to_string(),
            },
            agent: AgentType::Claude,
        };

        assert_eq!(
            invalid.validate(),
            Err(WorkspaceLifecycleError::InvalidWorkspaceName)
        );

        let valid_existing_branch = CreateWorkspaceRequest {
            workspace_name: "feature_auth".to_string(),
            branch_mode: BranchMode::ExistingBranch {
                existing_branch: "feature/auth.v2".to_string(),
            },
            agent: AgentType::Codex,
        };

        assert_eq!(valid_existing_branch.validate(), Ok(()));
        assert_eq!(valid_existing_branch.branch_name(), "feature/auth.v2");
    }

    #[test]
    fn create_workspace_new_branch_sequences_git_markers_gitignore_and_env_copy() {
        let temp = TestDir::new("create-new");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        fs::write(repo_root.join(".env"), "A=1\n").expect(".env should be written");
        fs::write(repo_root.join(".env.local"), "B=2\n").expect(".env.local should be written");

        let git = StubGitRunner::default();
        let setup = StubSetupRunner::default();
        let request = CreateWorkspaceRequest {
            workspace_name: "feature_a".to_string(),
            branch_mode: BranchMode::NewBranch {
                base_branch: "main".to_string(),
            },
            agent: AgentType::Claude,
        };

        let result =
            create_workspace(&repo_root, &request, &git, &setup).expect("create should succeed");
        let expected_workspace_path = temp.path.join("grove-feature_a");

        assert_eq!(result.workspace_path, expected_workspace_path);
        assert!(result.warnings.is_empty());
        assert_eq!(
            git.calls(),
            vec![vec![
                "worktree".to_string(),
                "add".to_string(),
                "-b".to_string(),
                "feature_a".to_string(),
                expected_workspace_path.to_string_lossy().to_string(),
                "main".to_string(),
            ]]
        );

        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".grove-agent"))
                .expect("agent marker should be readable")
                .trim(),
            "claude"
        );
        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".grove-base"))
                .expect("base marker should be readable")
                .trim(),
            "main"
        );
        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".env")).expect(".env should copy"),
            "A=1\n"
        );
        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".env.local"))
                .expect(".env.local should copy"),
            "B=2\n"
        );

        let gitignore =
            fs::read_to_string(repo_root.join(".gitignore")).expect(".gitignore should exist");
        assert!(gitignore.contains(".grove-agent"));
        assert!(gitignore.contains(".grove-base"));
        assert!(gitignore.contains(".grove-start.sh"));
        assert!(gitignore.contains(".grove-setup.sh"));
    }

    #[test]
    fn create_workspace_existing_branch_uses_attach_command_and_marker_branch() {
        let temp = TestDir::new("create-existing");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");

        let git = StubGitRunner::default();
        let setup = StubSetupRunner::default();
        let request = CreateWorkspaceRequest {
            workspace_name: "resume_auth".to_string(),
            branch_mode: BranchMode::ExistingBranch {
                existing_branch: "feature/auth.v2".to_string(),
            },
            agent: AgentType::Codex,
        };

        let result =
            create_workspace(&repo_root, &request, &git, &setup).expect("create should succeed");
        let expected_workspace_path = temp.path.join("grove-resume_auth");

        assert_eq!(
            git.calls(),
            vec![vec![
                "worktree".to_string(),
                "add".to_string(),
                expected_workspace_path.to_string_lossy().to_string(),
                "feature/auth.v2".to_string(),
            ]]
        );
        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".grove-base"))
                .expect("base marker should be readable")
                .trim(),
            "feature/auth.v2"
        );
        assert_eq!(
            fs::read_to_string(result.workspace_path.join(".grove-agent"))
                .expect("agent marker should be readable")
                .trim(),
            "codex"
        );
    }

    #[test]
    fn create_workspace_setup_script_failure_is_warning_not_failure() {
        let temp = TestDir::new("setup-warning");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        fs::write(
            repo_root.join(".grove-setup.sh"),
            "#!/usr/bin/env bash\nexit 1\n",
        )
        .expect("setup script should exist");

        let git = StubGitRunner::default();
        let setup = StubSetupRunner::with_outcomes(vec![Err("script exploded".to_string())]);
        let request = CreateWorkspaceRequest {
            workspace_name: "feature_b".to_string(),
            branch_mode: BranchMode::NewBranch {
                base_branch: "main".to_string(),
            },
            agent: AgentType::Claude,
        };

        let result =
            create_workspace(&repo_root, &request, &git, &setup).expect("create should succeed");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("script exploded"));

        let setup_calls = setup.calls();
        assert_eq!(setup_calls.len(), 1);
        assert_eq!(setup_calls[0].main_worktree_path, repo_root);
        assert_eq!(
            setup_calls[0].workspace_path,
            temp.path.join("grove-feature_b")
        );
        assert_eq!(setup_calls[0].worktree_branch, "feature_b");
    }

    #[test]
    fn ensure_gitignore_entries_is_idempotent() {
        let temp = TestDir::new("gitignore");
        let repo_root = temp.path.join("grove");
        fs::create_dir_all(&repo_root).expect("repo dir should exist");
        fs::write(repo_root.join(".gitignore"), ".grove-agent\n/target/\n")
            .expect(".gitignore should be writable");

        ensure_grove_gitignore_entries(&repo_root).expect("first ensure should succeed");
        ensure_grove_gitignore_entries(&repo_root).expect("second ensure should succeed");

        let gitignore =
            fs::read_to_string(repo_root.join(".gitignore")).expect(".gitignore should exist");
        assert_eq!(count_line(&gitignore, ".grove-agent"), 1);
        assert_eq!(count_line(&gitignore, ".grove-base"), 1);
        assert_eq!(count_line(&gitignore, ".grove-start.sh"), 1);
        assert_eq!(count_line(&gitignore, ".grove-setup.sh"), 1);
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
    fn read_workspace_markers_validates_marker_content() {
        let temp = TestDir::new("markers");
        let workspace = temp.path.join("grove-feature-z");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        fs::write(workspace.join(".grove-agent"), "unknown\n")
            .expect("agent marker should be writable");
        fs::write(workspace.join(".grove-base"), "main\n").expect("base marker should be writable");

        assert_eq!(
            read_workspace_markers(&workspace),
            Err(WorkspaceMarkerError::InvalidAgentMarker(
                "unknown".to_string()
            ))
        );
    }

    #[test]
    fn workspace_directory_path_uses_repo_prefix() {
        let repo_root = Path::new("/repos/grove");
        assert_eq!(
            workspace_directory_path(repo_root, "feature_a")
                .expect("path derivation should succeed"),
            PathBuf::from("/repos/grove-feature_a")
        );
    }

    fn count_line(content: &str, target: &str) -> usize {
        content.lines().filter(|line| line.trim() == target).count()
    }
}
