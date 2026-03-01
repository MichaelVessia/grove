use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::Workspace;

#[path = "adapters/metadata.rs"]
mod metadata;
#[path = "adapters/parser.rs"]
mod parser;
#[path = "adapters/workspace.rs"]
mod workspace;

#[cfg(test)]
use parser::{parse_branch_activity, parse_worktree_porcelain};
#[cfg(test)]
use workspace::{build_workspaces, workspace_name_from_path};

const TMUX_SESSION_PREFIX: &str = "grove-ws-";

pub trait GitAdapter {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError>;
}

pub trait MultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String>;
}

pub trait SystemAdapter {
    fn repo_name(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitAdapterError {
    CommandFailed(String),
    InvalidUtf8(String),
    ParseError(String),
}

impl GitAdapterError {
    pub fn message(&self) -> String {
        match self {
            Self::CommandFailed(message) => format!("git command failed: {message}"),
            Self::InvalidUtf8(message) => format!("git output was not valid UTF-8: {message}"),
            Self::ParseError(message) => format!("git output parse failed: {message}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiscoveryState {
    Ready,
    Empty,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootstrapData {
    pub repo_name: String,
    pub workspaces: Vec<Workspace>,
    pub discovery_state: DiscoveryState,
}

pub(crate) fn bootstrap_data(
    git: &impl GitAdapter,
    _multiplexer: &impl MultiplexerAdapter,
    system: &impl SystemAdapter,
) -> BootstrapData {
    let repo_name = system.repo_name();

    match git.list_workspaces() {
        Ok(workspaces) if workspaces.is_empty() => BootstrapData {
            repo_name,
            workspaces,
            discovery_state: DiscoveryState::Empty,
        },
        Ok(workspaces) => BootstrapData {
            repo_name,
            workspaces,
            discovery_state: DiscoveryState::Ready,
        },
        Err(error) => BootstrapData {
            repo_name,
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Error(error.message()),
        },
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandGitAdapter {
    repo_root: Option<PathBuf>,
}

impl CommandGitAdapter {
    pub fn for_repo(repo_root: PathBuf) -> Self {
        Self {
            repo_root: Some(repo_root),
        }
    }

    fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    fn run_git(&self, args: &[&str]) -> Result<String, GitAdapterError> {
        let mut command = Command::new("git");
        if let Some(repo_root) = self.repo_root() {
            command.current_dir(repo_root);
        }
        let output = command
            .args(args)
            .output()
            .map_err(|error| GitAdapterError::CommandFailed(error.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).map_err(|error| {
                GitAdapterError::InvalidUtf8(format!("stderr decode failed: {error}"))
            })?;
            return Err(GitAdapterError::CommandFailed(stderr.trim().to_string()));
        }

        String::from_utf8(output.stdout)
            .map_err(|error| GitAdapterError::InvalidUtf8(format!("stdout decode failed: {error}")))
    }
}

impl GitAdapter for CommandGitAdapter {
    fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError> {
        let repo_root_raw = self.run_git(&["rev-parse", "--show-toplevel"])?;
        let repo_root = PathBuf::from(repo_root_raw.trim());
        let repo_name = repo_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                GitAdapterError::ParseError(format!(
                    "could not derive repo name from '{}'",
                    repo_root.display()
                ))
            })?;

        let activity_raw = self.run_git(&[
            "for-each-ref",
            "--format=%(refname:short) %(committerdate:unix)",
            "refs/heads",
        ])?;
        let activity_by_branch = parser::parse_branch_activity(&activity_raw);

        let porcelain_raw = self.run_git(&["worktree", "list", "--porcelain"])?;
        let parsed_worktrees = parser::parse_worktree_porcelain(&porcelain_raw)?;

        workspace::build_workspaces(
            &parsed_worktrees,
            &repo_root,
            &repo_name,
            &activity_by_branch,
        )
    }
}

pub struct CommandMultiplexerAdapter;

impl MultiplexerAdapter for CommandMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8(output.stdout);
                match stdout {
                    Ok(content) => content
                        .lines()
                        .filter(|name| name.starts_with(TMUX_SESSION_PREFIX))
                        .map(ToOwned::to_owned)
                        .collect(),
                    Err(_) => HashSet::new(),
                }
            }
            _ => HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandSystemAdapter {
    repo_root: Option<PathBuf>,
}

impl CommandSystemAdapter {
    pub fn for_repo(repo_root: PathBuf) -> Self {
        Self {
            repo_root: Some(repo_root),
        }
    }
}

impl SystemAdapter for CommandSystemAdapter {
    fn repo_name(&self) -> String {
        if let Some(repo_root) = self.repo_root.as_ref()
            && let Some(name) = repo_root.file_name().and_then(|value| value.to_str())
        {
            return name.to_string();
        }

        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output();

        if let Ok(output) = output
            && output.status.success()
            && let Ok(stdout) = String::from_utf8(output.stdout)
        {
            let root = PathBuf::from(stdout.trim());
            if let Some(name) = root.file_name().and_then(|value| value.to_str()) {
                return name.to_string();
            }
        }

        std::env::current_dir()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .and_then(|value| value.to_str().map(str::to_string))
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{
        BootstrapData, DiscoveryState, GitAdapter, GitAdapterError, MultiplexerAdapter,
        SystemAdapter, bootstrap_data, build_workspaces, parse_branch_activity,
        parse_worktree_porcelain, workspace_name_from_path,
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
            GitAdapterError::ParseError(
                "encountered metadata before any worktree line".to_string()
            )
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
        fs::create_dir_all(managed.join(".grove")).expect(".grove should exist");

        fs::write(managed.join(".grove/agent"), "codex\n").expect("agent marker should exist");
        fs::write(managed.join(".grove/base"), "main\n").expect("base marker should exist");

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

        let workspaces =
            build_workspaces(&parsed, Path::new(&main_root), "grove", &activity_by_branch)
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
        fs::create_dir_all(main_root.join(".grove")).expect(".grove should exist");
        fs::create_dir_all(managed.join(".grove")).expect(".grove should exist");
        fs::write(main_root.join(".grove/agent"), "codex\n")
            .expect("main agent marker should exist");
        fs::write(managed.join(".grove/agent"), "claude\n").expect("agent marker should exist");
        fs::write(managed.join(".grove/base"), "main\n").expect("base marker should exist");

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
        assert_eq!(data.workspaces[1].status, WorkspaceStatus::Idle);
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
}
