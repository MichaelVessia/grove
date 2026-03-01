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
mod tests;
