use std::path::{Path, PathBuf};

use super::WorkspaceLifecycleError;

pub(super) fn workspace_directory_path(
    repo_root: &Path,
    workspace_name: &str,
) -> Result<PathBuf, WorkspaceLifecycleError> {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(WorkspaceLifecycleError::RepoNameUnavailable)?;
    let home_directory =
        dirs::home_dir().ok_or(WorkspaceLifecycleError::HomeDirectoryUnavailable)?;
    let workspaces_root = home_directory.join(".grove").join("workspaces");
    let repo_bucket = format!("{repo_name}-{}", stable_repo_path_hash(repo_root));
    Ok(workspaces_root
        .join(repo_bucket)
        .join(format!("{repo_name}-{workspace_name}")))
}

fn stable_repo_path_hash(repo_root: &Path) -> String {
    const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    let normalized = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let mut hash = FNV_OFFSET_BASIS;
    for byte in normalized.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("{hash:016x}")
}
