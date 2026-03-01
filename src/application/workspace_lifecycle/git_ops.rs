use std::path::Path;
use std::process::Command;

use crate::infrastructure::process::stderr_trimmed;

pub(super) fn run_delete_worktree_git(
    repo_root: &Path,
    workspace_path: &Path,
    is_missing: bool,
) -> Result<(), String> {
    if is_missing {
        return run_git_command(repo_root, &["worktree".to_string(), "prune".to_string()])
            .map_err(|error| format!("git worktree prune failed: {error}"));
    }

    let workspace_path_arg = workspace_path.to_string_lossy().to_string();
    let remove_args = vec![
        "worktree".to_string(),
        "remove".to_string(),
        workspace_path_arg.clone(),
    ];
    if run_git_command(repo_root, &remove_args).is_ok() {
        return Ok(());
    }

    run_git_command(
        repo_root,
        &[
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            workspace_path_arg,
        ],
    )
    .map_err(|error| format!("git worktree remove failed: {error}"))
}

pub(super) fn run_delete_local_branch_git(repo_root: &Path, branch: &str) -> Result<(), String> {
    let safe_args = vec!["branch".to_string(), "-d".to_string(), branch.to_string()];
    if run_git_command(repo_root, &safe_args).is_ok() {
        return Ok(());
    }

    run_git_command(
        repo_root,
        &["branch".to_string(), "-D".to_string(), branch.to_string()],
    )
    .map_err(|error| format!("git branch delete failed: {error}"))
}

pub(super) fn run_git_command(repo_root: &Path, args: &[String]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = stderr_trimmed(&output);
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stderr.is_empty() && stdout.is_empty() {
        return Err(format!(
            "git {}: exit status {}",
            args.join(" "),
            output.status
        ));
    }
    let details = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stderr}; {stdout}")
    };
    Err(format!("git {}: {details}", args.join(" ")))
}

pub(super) fn ensure_git_worktree_clean(worktree_path: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|error| format!("git status --porcelain: {error}"))?;

    if !output.status.success() {
        let stderr = stderr_trimmed(&output);
        if stderr.is_empty() {
            return Err(format!("git exited with status {}", output.status));
        }
        return Err(stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(());
    }

    Err("commit, stash, or discard changes first".to_string())
}
