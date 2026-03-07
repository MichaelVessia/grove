use std::fs;
use std::path::{Path, PathBuf};

use crate::application::workspace_lifecycle::GitCommandRunner;
use crate::domain::Task;
use crate::infrastructure::paths::refer_to_same_location;

use super::DeleteTaskRequest;

pub(super) fn delete_task_with_runner(
    request: DeleteTaskRequest,
    git_runner: &impl GitCommandRunner,
    stop_sessions: impl Fn(&Task),
    manifest_tasks_root: Option<&Path>,
) -> (Result<(), String>, Vec<String>) {
    let DeleteTaskRequest {
        task,
        delete_local_branch,
        kill_tmux_sessions,
    } = request;
    let task_root = task.root_path.clone();
    let manifest_task_root =
        manifest_task_root(manifest_tasks_root, task.slug.as_str(), task_root.as_path());
    let mut warnings = Vec::new();

    if kill_tmux_sessions {
        stop_sessions(&task);
    }

    for worktree in &task.worktrees {
        let is_missing = !worktree.path.exists();
        if let Err(error) = run_delete_worktree_git(
            git_runner,
            worktree.repository_path.as_path(),
            worktree.path.as_path(),
            is_missing,
        ) {
            return (Err(error), warnings);
        }

        if delete_local_branch
            && let Err(error) = run_delete_local_branch_git(
                git_runner,
                worktree.repository_path.as_path(),
                worktree.branch.as_str(),
            )
        {
            warnings.push(format!(
                "{} branch cleanup: {error}",
                worktree.repository_name
            ));
        }
    }

    if let Err(error) = remove_task_root(task_root.as_path()) {
        return (Err(error), warnings);
    }

    if let Some(manifest_task_root) = manifest_task_root
        && let Err(error) = remove_task_root(manifest_task_root.as_path())
    {
        return (Err(error), warnings);
    }

    (Ok(()), warnings)
}

fn manifest_task_root(
    manifest_tasks_root: Option<&Path>,
    task_slug: &str,
    task_root: &Path,
) -> Option<PathBuf> {
    let manifest_task_root = manifest_tasks_root?.join(task_slug);
    if refer_to_same_location(manifest_task_root.as_path(), task_root) {
        return None;
    }

    Some(manifest_task_root)
}

fn remove_task_root(task_root: &Path) -> Result<(), String> {
    if !task_root.exists() {
        return Ok(());
    }

    fs::remove_dir_all(task_root)
        .map_err(|error| format!("remove task root '{}': {error}", task_root.display()))
}

fn run_delete_worktree_git(
    git_runner: &impl GitCommandRunner,
    repo_root: &Path,
    worktree_path: &Path,
    is_missing: bool,
) -> Result<(), String> {
    if is_missing {
        return git_runner
            .run(repo_root, &["worktree".to_string(), "prune".to_string()])
            .map_err(|error| format!("git worktree prune failed: {error}"));
    }

    let worktree_path_arg = worktree_path.to_string_lossy().to_string();
    let remove_args = vec![
        "worktree".to_string(),
        "remove".to_string(),
        worktree_path_arg.clone(),
    ];
    if git_runner.run(repo_root, &remove_args).is_ok() {
        return Ok(());
    }

    git_runner
        .run(
            repo_root,
            &[
                "worktree".to_string(),
                "remove".to_string(),
                "--force".to_string(),
                worktree_path_arg,
            ],
        )
        .map_err(|error| format!("git worktree remove failed: {error}"))
}

fn run_delete_local_branch_git(
    git_runner: &impl GitCommandRunner,
    repo_root: &Path,
    branch: &str,
) -> Result<(), String> {
    let safe_args = vec!["branch".to_string(), "-d".to_string(), branch.to_string()];
    if git_runner.run(repo_root, &safe_args).is_ok() {
        return Ok(());
    }

    git_runner
        .run(
            repo_root,
            &["branch".to_string(), "-D".to_string(), branch.to_string()],
        )
        .map_err(|error| format!("git branch delete failed: {error}"))
}
