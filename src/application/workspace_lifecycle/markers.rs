use std::fs;
use std::path::Path;

use super::{WorkspaceLifecycleError, WorkspaceMarkerError, WorkspaceMarkers};

pub(super) fn read_workspace_markers(
    workspace_path: &Path,
) -> Result<WorkspaceMarkers, WorkspaceMarkerError> {
    let base_marker_path = workspace_path.join(super::GROVE_BASE_MARKER_FILE);
    let base_marker_content = match fs::read_to_string(&base_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingBaseMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    let base_branch = base_marker_content.trim().to_string();
    if base_branch.is_empty() {
        return Err(WorkspaceMarkerError::EmptyBaseBranch);
    }

    Ok(WorkspaceMarkers { base_branch })
}

pub(super) fn write_workspace_base_marker(
    workspace_path: &Path,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    ensure_workspace_grove_dir(workspace_path)?;
    let base_marker_path = workspace_path.join(super::GROVE_BASE_MARKER_FILE);
    fs::write(base_marker_path, format!("{base_branch}\n"))
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))
}

fn ensure_workspace_grove_dir(workspace_path: &Path) -> Result<(), WorkspaceLifecycleError> {
    fs::create_dir_all(workspace_path.join(super::GROVE_DIR))
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))
}
