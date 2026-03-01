use crate::infrastructure::paths::refer_to_same_location;

use super::UpdateWorkspaceFromBaseRequest;

pub(super) fn update_workspace_from_base_with_session_stopper(
    request: UpdateWorkspaceFromBaseRequest,
    _stop_sessions: impl Fn(Option<&str>, &str),
) -> (Result<(), String>, Vec<String>) {
    let warnings = Vec::new();

    if let Err(error) = super::requests::validate_update_request(&request) {
        return (Err(error), warnings);
    }
    let repo_root = match super::requests::resolve_repo_root(request.project_path.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return (Err(error), warnings);
        }
    };

    let is_base_workspace_update = request.workspace_branch == request.base_branch
        && refer_to_same_location(&request.workspace_path, &repo_root);

    if request.workspace_branch == request.base_branch && !is_base_workspace_update {
        return (
            Err("workspace branch matches base branch".to_string()),
            warnings,
        );
    }

    if let Err(error) = super::git_ops::run_git_command(
        &repo_root,
        &[
            "rev-parse".to_string(),
            "--verify".to_string(),
            request.base_branch.clone(),
        ],
    ) {
        return (
            Err(format!(
                "base branch '{}' is not available: {error}",
                request.base_branch
            )),
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
        &request.workspace_path,
        &["switch".to_string(), request.workspace_branch.clone()],
    ) {
        return (Err(format!("git switch failed: {error}")), warnings);
    }

    if is_base_workspace_update {
        if let Err(error) = super::git_ops::run_git_command(
            &request.workspace_path,
            &[
                "pull".to_string(),
                "--ff-only".to_string(),
                "origin".to_string(),
                request.base_branch.clone(),
            ],
        ) {
            return (Err(format!("git pull failed: {error}")), warnings);
        }
        return (Ok(()), warnings);
    }

    if let Err(error) = super::git_ops::run_git_command(
        &request.workspace_path,
        &[
            "merge".to_string(),
            "--no-ff".to_string(),
            request.base_branch.clone(),
        ],
    ) {
        let _ = super::git_ops::run_git_command(
            &request.workspace_path,
            &["merge".to_string(), "--abort".to_string()],
        );
        return (Err(format!("git merge failed: {error}")), warnings);
    }

    (Ok(()), warnings)
}
