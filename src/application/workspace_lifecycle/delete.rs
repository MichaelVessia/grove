use super::DeleteWorkspaceRequest;

pub(super) fn delete_workspace_with_session_stopper(
    request: DeleteWorkspaceRequest,
    stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    let mut warnings = Vec::new();
    if request.kill_tmux_sessions {
        stop_sessions(
            request.project_name.as_deref(),
            request.workspace_name.as_str(),
        );
    }

    let repo_root = match super::requests::resolve_repo_root(request.project_path.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return (Err(error), warnings);
        }
    };

    if let Err(error) = super::git_ops::run_delete_worktree_git(
        &repo_root,
        &request.workspace_path,
        request.is_missing,
    ) {
        return (Err(error), warnings);
    }

    if request.delete_local_branch
        && let Err(error) = super::git_ops::run_delete_local_branch_git(&repo_root, &request.branch)
    {
        warnings.push(format!("local branch: {error}"));
    }

    (Ok(()), warnings)
}
