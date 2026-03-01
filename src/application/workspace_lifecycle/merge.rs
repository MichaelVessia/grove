use super::MergeWorkspaceRequest;

pub(super) fn merge_workspace_with_session_stopper(
    request: MergeWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    let mut warnings = Vec::new();

    if let Err(error) = super::requests::validate_merge_request(&request) {
        return (Err(error), warnings);
    }
    let repo_root = match super::requests::resolve_repo_root(request.project_path.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return (Err(error), warnings);
        }
    };

    if let Err(error) = super::git_ops::ensure_git_worktree_clean(&repo_root) {
        return (
            Err(format!("base worktree has uncommitted changes: {error}")),
            warnings,
        );
    }
    if let Err(error) = super::git_ops::ensure_git_worktree_clean(&request.workspace_path) {
        return (
            Err(format!(
                "workspace worktree has uncommitted changes: {error}"
            )),
            warnings,
        );
    }

    if let Err(error) = super::git_ops::run_git_command(
        &repo_root,
        &["switch".to_string(), request.base_branch.clone()],
    ) {
        return (Err(format!("git switch failed: {error}")), warnings);
    }

    if let Err(error) = super::git_ops::run_git_command(
        &repo_root,
        &[
            "merge".to_string(),
            "--no-ff".to_string(),
            request.workspace_branch.clone(),
        ],
    ) {
        let _ = super::git_ops::run_git_command(
            &repo_root,
            &["merge".to_string(), "--abort".to_string()],
        );
        return (Err(format!("git merge failed: {error}")), warnings);
    }

    if request.cleanup_workspace {
        stop_sessions(
            request.project_name.as_deref(),
            request.workspace_name.as_str(),
        );
        if let Err(error) =
            super::git_ops::run_delete_worktree_git(&repo_root, &request.workspace_path, false)
        {
            warnings.push(format!("workspace cleanup: {error}"));
        }
    }

    if request.cleanup_local_branch
        && let Err(error) =
            super::git_ops::run_delete_local_branch_git(&repo_root, &request.workspace_branch)
    {
        warnings.push(format!("local branch cleanup: {error}"));
    }

    (Ok(()), warnings)
}
