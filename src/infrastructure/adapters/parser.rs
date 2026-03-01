use std::collections::HashMap;
use std::path::PathBuf;

use super::GitAdapterError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedWorktree {
    pub(super) path: PathBuf,
    pub(super) branch: Option<String>,
    pub(super) is_detached: bool,
}

pub(super) fn parse_worktree_porcelain(
    input: &str,
) -> Result<Vec<ParsedWorktree>, GitAdapterError> {
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

pub(super) fn parse_branch_activity(input: &str) -> HashMap<String, i64> {
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
