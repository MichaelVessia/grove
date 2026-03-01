use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::{AgentType, Workspace, WorkspaceStatus};

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
        let activity_by_branch = parse_branch_activity(&activity_raw);

        let porcelain_raw = self.run_git(&["worktree", "list", "--porcelain"])?;
        let parsed_worktrees = parse_worktree_porcelain(&porcelain_raw)?;

        build_workspaces(
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedWorktree {
    path: PathBuf,
    branch: Option<String>,
    is_detached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkerMetadata {
    agent: AgentType,
    base_branch: Option<String>,
    supported_agent: bool,
}

fn parse_worktree_porcelain(input: &str) -> Result<Vec<ParsedWorktree>, GitAdapterError> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut current_is_detached = false;

    for line in input.lines() {
        if line.trim().is_empty() {
            push_current_worktree(
                &mut worktrees,
                &mut current_path,
                &mut current_branch,
                &mut current_is_detached,
            )?;
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            push_current_worktree(
                &mut worktrees,
                &mut current_path,
                &mut current_branch,
                &mut current_is_detached,
            )?;
            current_path = Some(PathBuf::from(path));
            continue;
        }

        if current_path.is_none() {
            return Err(GitAdapterError::ParseError(
                "encountered metadata before any worktree line".to_string(),
            ));
        }

        if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = Some(short_branch_name(branch_ref));
            current_is_detached = false;
            continue;
        }

        if line == "detached" {
            current_branch = None;
            current_is_detached = true;
        }
    }

    push_current_worktree(
        &mut worktrees,
        &mut current_path,
        &mut current_branch,
        &mut current_is_detached,
    )?;

    Ok(worktrees)
}

fn push_current_worktree(
    worktrees: &mut Vec<ParsedWorktree>,
    current_path: &mut Option<PathBuf>,
    current_branch: &mut Option<String>,
    current_is_detached: &mut bool,
) -> Result<(), GitAdapterError> {
    let path = match current_path.take() {
        Some(path) => path,
        None => {
            if current_branch.is_some() || *current_is_detached {
                return Err(GitAdapterError::ParseError(
                    "worktree metadata was present without a path".to_string(),
                ));
            }
            return Ok(());
        }
    };

    worktrees.push(ParsedWorktree {
        path,
        branch: current_branch.take(),
        is_detached: *current_is_detached,
    });
    *current_is_detached = false;

    Ok(())
}

fn short_branch_name(branch_ref: &str) -> String {
    branch_ref
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_ref)
        .to_string()
}

fn parse_branch_activity(input: &str) -> HashMap<String, i64> {
    let mut activity = HashMap::new();

    for line in input.lines() {
        if let Some((branch, timestamp)) = line.rsplit_once(' ')
            && let Ok(unix_secs) = timestamp.parse::<i64>()
        {
            activity.insert(branch.to_string(), unix_secs);
        }
    }

    activity
}

fn workspace_name_from_path(path: &Path, repo_name: &str, is_main: bool) -> String {
    let directory_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string());

    if is_main {
        return repo_name.to_string();
    }

    let repo_prefix = format!("{repo_name}-");
    directory_name
        .strip_prefix(&repo_prefix)
        .unwrap_or(&directory_name)
        .to_string()
}

fn workspace_status(is_main: bool, branch: &Option<String>, is_detached: bool) -> WorkspaceStatus {
    if is_main {
        return WorkspaceStatus::Main;
    }
    if is_detached || branch.is_none() {
        return WorkspaceStatus::Unknown;
    }

    WorkspaceStatus::Idle
}

fn workspace_sort(left: &Workspace, right: &Workspace) -> Ordering {
    match (left.is_main, right.is_main) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    let activity_order = right
        .last_activity_unix_secs
        .cmp(&left.last_activity_unix_secs);
    if activity_order != Ordering::Equal {
        return activity_order;
    }

    left.name.cmp(&right.name)
}

fn marker_metadata(path: &Path) -> Result<Option<MarkerMetadata>, GitAdapterError> {
    let Some(agent_value) = read_marker_file(path, ".grove/agent")? else {
        return Ok(None);
    };
    let Some(agent) = AgentType::from_marker(agent_value.trim()) else {
        return Ok(Some(unsupported_marker_metadata()));
    };

    let Some(base_value) = read_marker_file(path, ".grove/base")? else {
        return Ok(Some(unsupported_marker_metadata()));
    };
    let base_branch = base_value.trim().to_string();
    if base_branch.is_empty() {
        return Ok(Some(unsupported_marker_metadata()));
    }

    Ok(Some(MarkerMetadata {
        agent,
        base_branch: Some(base_branch),
        supported_agent: true,
    }))
}

fn main_workspace_metadata(path: &Path) -> Result<MarkerMetadata, GitAdapterError> {
    match read_marker_file(path, ".grove/agent")? {
        Some(value) => {
            if let Some(agent) = AgentType::from_marker(value.trim()) {
                return Ok(MarkerMetadata {
                    agent,
                    base_branch: None,
                    supported_agent: true,
                });
            }
            Ok(default_main_marker_metadata())
        }
        None => Ok(default_main_marker_metadata()),
    }
}

fn read_marker_file(path: &Path, relative_marker: &str) -> Result<Option<String>, GitAdapterError> {
    let marker_path = path.join(relative_marker);
    match fs::read_to_string(&marker_path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(GitAdapterError::ParseError(format!(
            "failed reading marker '{}' in '{}': {error}",
            relative_marker,
            path.display()
        ))),
    }
}

fn unsupported_marker_metadata() -> MarkerMetadata {
    MarkerMetadata {
        agent: AgentType::Claude,
        base_branch: None,
        supported_agent: false,
    }
}

fn default_main_marker_metadata() -> MarkerMetadata {
    MarkerMetadata {
        agent: AgentType::Claude,
        base_branch: None,
        supported_agent: true,
    }
}

fn build_workspaces(
    parsed_worktrees: &[ParsedWorktree],
    repo_root: &Path,
    repo_name: &str,
    activity_by_branch: &HashMap<String, i64>,
) -> Result<Vec<Workspace>, GitAdapterError> {
    let mut workspaces = Vec::new();

    for entry in parsed_worktrees {
        let is_main = entry.path == repo_root;
        let branch = entry
            .branch
            .clone()
            .unwrap_or_else(|| "(detached)".to_string());
        let last_activity_unix_secs = entry
            .branch
            .as_ref()
            .and_then(|branch_name| activity_by_branch.get(branch_name).copied());

        let metadata = if is_main {
            Some(main_workspace_metadata(&entry.path)?)
        } else {
            marker_metadata(&entry.path)?
        };

        let Some(metadata) = metadata else {
            continue;
        };

        let status = if metadata.supported_agent {
            workspace_status(is_main, &entry.branch, entry.is_detached)
        } else {
            WorkspaceStatus::Unsupported
        };

        let workspace = Workspace::try_new(
            workspace_name_from_path(&entry.path, repo_name, is_main),
            entry.path.clone(),
            branch,
            last_activity_unix_secs,
            metadata.agent,
            status,
            is_main,
        )
        .map_err(|error| {
            GitAdapterError::ParseError(format!(
                "worktree '{}' failed validation: {error:?}",
                entry.path.display()
            ))
        })?
        .with_project_context(repo_name.to_string(), repo_root.to_path_buf())
        .with_base_branch(metadata.base_branch)
        .with_supported_agent(metadata.supported_agent);

        workspaces.push(workspace);
    }

    workspaces.sort_by(workspace_sort);

    Ok(workspaces)
}

#[cfg(test)]
mod tests;
