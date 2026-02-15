use super::{
    BranchMode, CreateWorkspaceRequest, DeleteWorkspaceRequest, GitCommandRunner,
    SetupScriptContext, SetupScriptRunner, WorkspaceLifecycleError, WorkspaceMarkerError,
    copy_env_files, create_workspace, delete_workspace, ensure_grove_gitignore_entries,
    read_workspace_agent_marker, read_workspace_markers, workspace_directory_path,
    workspace_lifecycle_error_message, write_workspace_agent_marker,
};
use crate::config::MultiplexerKind;
use crate::domain::AgentType;
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
fn read_workspace_agent_marker_reads_valid_marker() {
    let temp = TestDir::new("agent-marker-read");
    let workspace = temp.path.join("grove-feature-z");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::write(workspace.join(".grove-agent"), "codex\n").expect("marker should be writable");

    let marker = read_workspace_agent_marker(&workspace).expect("marker should be readable");
    assert_eq!(marker, AgentType::Codex);
}

#[test]
fn write_workspace_agent_marker_writes_expected_value() {
    let temp = TestDir::new("agent-marker-write");
    let workspace = temp.path.join("grove");
    fs::create_dir_all(&workspace).expect("workspace should exist");

    write_workspace_agent_marker(&workspace, AgentType::Claude).expect("write should succeed");
    assert_eq!(
        fs::read_to_string(workspace.join(".grove-agent"))
            .expect("marker should be readable")
            .trim(),
        "claude"
    );

    write_workspace_agent_marker(&workspace, AgentType::Codex).expect("write should succeed");
    assert_eq!(
        fs::read_to_string(workspace.join(".grove-agent"))
            .expect("marker should be readable")
            .trim(),
        "codex"
    );
}

#[test]
fn workspace_directory_path_uses_repo_prefix() {
    let repo_root = Path::new("/repos/grove");
    assert_eq!(
        workspace_directory_path(repo_root, "feature_a").expect("path derivation should succeed"),
        PathBuf::from("/repos/grove-feature_a")
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
        project_name: None,
        project_path: Some(repo_root.clone()),
        workspace_name: "feature-x".to_string(),
        branch: "feature-x".to_string(),
        workspace_path: temp.path.join("grove-feature-x"),
        is_missing: true,
        delete_local_branch: false,
    };

    let (result, warnings) = delete_workspace(request, MultiplexerKind::Tmux);
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
        project_name: None,
        project_path: Some(repo_root.clone()),
        workspace_name: "feature-y".to_string(),
        branch: "missing-branch".to_string(),
        workspace_path: temp.path.join("grove-feature-y"),
        is_missing: true,
        delete_local_branch: true,
    };

    let (result, warnings) = delete_workspace(request, MultiplexerKind::Tmux);
    assert_eq!(result, Ok(()));
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].contains("local branch: git branch delete failed:"),
        "unexpected warning: {}",
        warnings[0]
    );
}

fn init_git_repo(repo_root: &Path) {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["init"])
        .output()
        .expect("git init should run");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn count_line(content: &str, target: &str) -> usize {
    content.lines().filter(|line| line.trim() == target).count()
}
