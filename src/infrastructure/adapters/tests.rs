use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    BootstrapData, DiscoveryState, GitAdapter, GitAdapterError, MultiplexerAdapter, SystemAdapter,
    bootstrap_data, build_workspaces, parse_branch_activity, parse_worktree_porcelain,
    parse_zellij_running_sessions, workspace_name_from_path,
};

use crate::domain::{AgentType, Workspace, WorkspaceStatus};

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
            "grove-adapter-{label}-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test dir should exist");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct FakeMultiplexerAdapter {
    running: HashSet<String>,
}

impl MultiplexerAdapter for FakeMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        self.running.clone()
    }
}

struct FakeSystemAdapter;

impl SystemAdapter for FakeSystemAdapter {
    fn repo_name(&self) -> String {
        "grove".to_string()
    }
}

struct FakeGitSuccess;

impl GitAdapter for FakeGitSuccess {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError> {
        Ok(vec![
            Workspace::try_new(
                "grove".to_string(),
                PathBuf::from("/repos/grove"),
                "main".to_string(),
                Some(1_700_000_300),
                AgentType::Claude,
                WorkspaceStatus::Main,
                true,
            )
            .expect("workspace should be valid"),
            Workspace::try_new(
                "feature-a".to_string(),
                PathBuf::from("/repos/grove-feature-a"),
                "feature-a".to_string(),
                Some(1_700_000_200),
                AgentType::Codex,
                WorkspaceStatus::Idle,
                false,
            )
            .expect("workspace should be valid"),
        ])
    }
}

struct FakeGitEmpty;

impl GitAdapter for FakeGitEmpty {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError> {
        Ok(Vec::new())
    }
}

struct FakeGitError;

impl GitAdapter for FakeGitError {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError> {
        Err(GitAdapterError::CommandFailed(
            "fatal: not a git repository".to_string(),
        ))
    }
}

#[test]
fn parse_worktree_porcelain_supports_branch_and_detached_entries() {
    let output = "worktree /repos/grove\nHEAD 123\nbranch refs/heads/main\n\nworktree /repos/grove-feature-a\nHEAD 456\nbranch refs/heads/feature-a\n\nworktree /repos/grove-detached\nHEAD 789\ndetached\n";

    let parsed = parse_worktree_porcelain(output).expect("porcelain should parse");

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].path, PathBuf::from("/repos/grove"));
    assert_eq!(parsed[0].branch, Some("main".to_string()));
    assert!(!parsed[0].is_detached);

    assert_eq!(parsed[2].path, PathBuf::from("/repos/grove-detached"));
    assert_eq!(parsed[2].branch, None);
    assert!(parsed[2].is_detached);
}

#[test]
fn parse_worktree_porcelain_rejects_metadata_before_worktree() {
    let output = "branch refs/heads/main\nworktree /repos/grove\n";

    let error = parse_worktree_porcelain(output).expect_err("parser should fail");

    assert_eq!(
        error,
        GitAdapterError::ParseError("encountered metadata before any worktree line".to_string())
    );
}

#[test]
fn parse_branch_activity_collects_unix_timestamps() {
    let output = "main 1700000300\nfeature-a 1700000200\ninvalid not-a-number\n";
    let activity = parse_branch_activity(output);

    assert_eq!(activity.get("main"), Some(&1_700_000_300));
    assert_eq!(activity.get("feature-a"), Some(&1_700_000_200));
    assert!(!activity.contains_key("invalid"));
}

#[test]
fn build_workspaces_includes_main_and_only_marker_managed_worktrees() {
    let temp = TestDir::new("build");
    let main_root = temp.path.join("grove");
    let managed = temp.path.join("grove-feature-a");
    let unmanaged = temp.path.join("grove-unmanaged");

    fs::create_dir_all(&main_root).expect("main should exist");
    fs::create_dir_all(&managed).expect("managed should exist");
    fs::create_dir_all(&unmanaged).expect("unmanaged should exist");

    fs::write(managed.join(".grove-agent"), "codex\n").expect("agent marker should exist");
    fs::write(managed.join(".grove-base"), "main\n").expect("base marker should exist");

    let parsed = parse_worktree_porcelain(&format!(
            "worktree {}\nHEAD 1\nbranch refs/heads/main\n\nworktree {}\nHEAD 2\nbranch refs/heads/feature-a\n\nworktree {}\nHEAD 3\nbranch refs/heads/unmanaged\n",
            main_root.display(),
            managed.display(),
            unmanaged.display(),
        ))
        .expect("porcelain should parse");

    let activity_by_branch = HashMap::from([
        ("main".to_string(), 1_700_000_400),
        ("feature-a".to_string(), 1_700_000_300),
        ("unmanaged".to_string(), 1_700_000_100),
    ]);

    let workspaces = build_workspaces(&parsed, Path::new(&main_root), "grove", &activity_by_branch)
        .expect("workspace build should succeed");

    assert_eq!(workspaces.len(), 2);
    assert_eq!(workspaces[0].name, "grove");
    assert_eq!(workspaces[0].status, WorkspaceStatus::Main);
    assert_eq!(workspaces[0].agent, AgentType::Claude);

    assert_eq!(workspaces[1].name, "feature-a");
    assert_eq!(workspaces[1].agent, AgentType::Codex);
    assert_eq!(workspaces[1].base_branch.as_deref(), Some("main"));
}

#[test]
fn build_workspaces_main_uses_agent_marker_when_present() {
    let temp = TestDir::new("build-main-agent");
    let main_root = temp.path.join("grove");
    let managed = temp.path.join("grove-feature-a");

    fs::create_dir_all(&main_root).expect("main should exist");
    fs::create_dir_all(&managed).expect("managed should exist");
    fs::write(main_root.join(".grove-agent"), "codex\n").expect("main agent marker should exist");
    fs::write(managed.join(".grove-agent"), "claude\n").expect("agent marker should exist");
    fs::write(managed.join(".grove-base"), "main\n").expect("base marker should exist");

    let parsed = parse_worktree_porcelain(&format!(
            "worktree {}\nHEAD 1\nbranch refs/heads/main\n\nworktree {}\nHEAD 2\nbranch refs/heads/feature-a\n",
            main_root.display(),
            managed.display(),
        ))
        .expect("porcelain should parse");

    let workspaces = build_workspaces(&parsed, Path::new(&main_root), "grove", &HashMap::new())
        .expect("workspace build should succeed");

    assert_eq!(workspaces.len(), 2);
    assert_eq!(workspaces[0].name, "grove");
    assert_eq!(workspaces[0].agent, AgentType::Codex);
    assert_eq!(workspaces[0].status, WorkspaceStatus::Main);
}

#[test]
fn workspace_name_from_path_strips_repo_prefix_for_non_main_worktrees() {
    let derived = workspace_name_from_path(Path::new("/repos/grove-feature-a"), "grove", false);
    assert_eq!(derived, "feature-a");

    let main = workspace_name_from_path(Path::new("/repos/grove"), "grove", true);
    assert_eq!(main, "grove");
}

#[test]
fn bootstrap_data_reports_ready_for_successful_discovery() {
    let data: BootstrapData = bootstrap_data(
        &FakeGitSuccess,
        &FakeMultiplexerAdapter {
            running: HashSet::from(["grove-ws-feature-a".to_string()]),
        },
        &FakeSystemAdapter,
    );

    assert_eq!(data.repo_name, "grove");
    assert_eq!(data.workspaces.len(), 2);
    assert_eq!(data.discovery_state, DiscoveryState::Ready);
    assert_eq!(data.workspaces[1].status, WorkspaceStatus::Active);
}

#[test]
fn bootstrap_data_reports_empty_state() {
    let data = bootstrap_data(
        &FakeGitEmpty,
        &FakeMultiplexerAdapter {
            running: HashSet::new(),
        },
        &FakeSystemAdapter,
    );
    assert_eq!(data.discovery_state, DiscoveryState::Empty);
}

#[test]
fn bootstrap_data_reports_error_state() {
    let data = bootstrap_data(
        &FakeGitError,
        &FakeMultiplexerAdapter {
            running: HashSet::new(),
        },
        &FakeSystemAdapter,
    );

    match data.discovery_state {
        DiscoveryState::Error(message) => {
            assert!(message.contains("not a git repository"));
        }
        other => panic!("expected error state, got: {other:?}"),
    }
}

#[test]
fn parse_zellij_running_sessions_ignores_exited_sessions() {
    let parsed = parse_zellij_running_sessions(
        "grove-ws-alpha [Created 1m ago]\n\
             grove-ws-beta [Created 2m ago] (EXITED - attach to resurrect)\n\
             unrelated [Created 1m ago]\n\
             grove-ws-gamma\n",
    );

    assert_eq!(
        parsed,
        HashSet::from(["grove-ws-alpha".to_string(), "grove-ws-gamma".to_string()])
    );
}
