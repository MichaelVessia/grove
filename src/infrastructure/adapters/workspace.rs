use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use crate::domain::{Workspace, WorkspaceStatus};

use super::GitAdapterError;
use super::metadata::{main_workspace_metadata, marker_metadata};
use super::parser::ParsedWorktree;

pub(super) fn workspace_name_from_path(path: &Path, repo_name: &str, is_main: bool) -> String {
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

pub(super) fn build_workspaces(
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
