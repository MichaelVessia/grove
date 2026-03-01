use std::path::PathBuf;

use super::{MergeWorkspaceRequest, UpdateWorkspaceFromBaseRequest};

pub(super) fn workspace_name_is_valid(name: &str) -> bool {
    name.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
}

pub(super) fn resolve_repo_root(project_path: Option<&PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = project_path {
        return Ok(path.clone());
    }

    std::env::current_dir().map_err(|_| "workspace project root unavailable".to_string())
}

pub(super) fn validate_merge_request(request: &MergeWorkspaceRequest) -> Result<(), String> {
    if request.workspace_name.trim().is_empty() {
        return Err("workspace name is required".to_string());
    }
    if request.workspace_branch.trim().is_empty() {
        return Err("workspace branch is required".to_string());
    }
    if request.base_branch.trim().is_empty() {
        return Err("base branch is required".to_string());
    }
    if request.workspace_branch == request.base_branch {
        return Err("workspace branch matches base branch".to_string());
    }
    if !request.workspace_path.exists() {
        return Err("workspace path does not exist on disk".to_string());
    }

    Ok(())
}

pub(super) fn validate_update_request(
    request: &UpdateWorkspaceFromBaseRequest,
) -> Result<(), String> {
    if request.workspace_name.trim().is_empty() {
        return Err("workspace name is required".to_string());
    }
    if request.workspace_branch.trim().is_empty() {
        return Err("workspace branch is required".to_string());
    }
    if request.base_branch.trim().is_empty() {
        return Err("base branch is required".to_string());
    }
    if !request.workspace_path.exists() {
        return Err("workspace path does not exist on disk".to_string());
    }

    Ok(())
}
