use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::infrastructure::config::{GroveConfig, ProjectConfig};
use crate::infrastructure::paths::refer_to_same_location;
use crate::infrastructure::process::stderr_trimmed;

use super::*;

pub(super) struct AppDependencies {
    pub(super) tmux_input: Box<dyn TmuxInput>,
    pub(super) clipboard: Box<dyn ClipboardAccess>,
    pub(super) config_path: PathBuf,
    pub(super) event_log: Box<dyn EventLogger>,
    pub(super) debug_record_start_ts: Option<u64>,
    pub(super) daemon_socket_path: Option<PathBuf>,
}

fn default_config_path() -> PathBuf {
    crate::infrastructure::config::config_path()
        .unwrap_or_else(|| PathBuf::from(".config/grove/config.toml"))
}

fn current_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?;
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.canonicalize().ok().or(Some(path))
}

pub(super) fn project_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn ensure_current_repo_project(config: &mut GroveConfig, config_path: &Path) -> Option<String> {
    let repo_root = current_repo_root()?;

    let already_present = config
        .projects
        .iter()
        .any(|project| refer_to_same_location(&project.path, &repo_root));
    if already_present {
        return None;
    }

    config.projects.push(ProjectConfig {
        name: project_display_name(&repo_root),
        path: repo_root,
        defaults: Default::default(),
    });
    crate::infrastructure::config::save_to_path(config_path, config).err()
}

pub(super) fn load_runtime_config() -> (GroveConfig, PathBuf, Option<String>) {
    let (mut config, config_path, load_error) = match crate::infrastructure::config::load() {
        Ok(loaded) => (loaded.config, loaded.path, None),
        Err(error) => (GroveConfig::default(), default_config_path(), Some(error)),
    };
    let startup_error = ensure_current_repo_project(&mut config, &config_path);
    let error = match (load_error, startup_error) {
        (Some(load_error), Some(startup_error)) => Some(format!(
            "{load_error}; startup project add failed: {startup_error}"
        )),
        (Some(load_error), None) => Some(load_error),
        (None, Some(startup_error)) => Some(format!("startup project add failed: {startup_error}")),
        (None, None) => None,
    };

    (config, config_path, error)
}

pub(super) fn read_workspace_launch_prompt(workspace_path: &Path) -> Option<String> {
    let raw = fs::read_to_string(workspace_path.join(WORKSPACE_LAUNCH_PROMPT_FILENAME)).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

pub(super) fn load_local_branches(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|error| format!("git branch failed: {error}"))?;
    if !output.status.success() {
        let stderr = stderr_trimmed(&output);
        return Err(format!("git branch failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("git branch output decode failed: {error}"))?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

pub(super) fn filter_branches(query: &str, all_branches: &[String]) -> Vec<String> {
    if query.is_empty() {
        return all_branches.to_vec();
    }

    let query_lower = query.to_lowercase();
    all_branches
        .iter()
        .filter(|branch| branch.to_lowercase().contains(&query_lower))
        .cloned()
        .collect()
}
