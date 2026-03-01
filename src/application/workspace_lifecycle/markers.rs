use std::fs;
use std::path::Path;

use crate::domain::AgentType;

use super::{WorkspaceLifecycleError, WorkspaceMarkerError, WorkspaceMarkers};

pub(super) fn read_workspace_markers(
    workspace_path: &Path,
) -> Result<WorkspaceMarkers, WorkspaceMarkerError> {
    let agent = read_workspace_agent_marker(workspace_path)?;

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

    Ok(WorkspaceMarkers { agent, base_branch })
}

pub(super) fn read_workspace_agent_marker(
    workspace_path: &Path,
) -> Result<AgentType, WorkspaceMarkerError> {
    let agent_marker_path = workspace_path.join(super::GROVE_AGENT_MARKER_FILE);
    let agent_marker_content = match fs::read_to_string(&agent_marker_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WorkspaceMarkerError::MissingAgentMarker);
        }
        Err(error) => return Err(WorkspaceMarkerError::Io(error.to_string())),
    };

    parse_agent_marker(agent_marker_content.trim())
}

pub(super) fn write_workspace_agent_marker(
    workspace_path: &Path,
    agent: AgentType,
) -> Result<(), WorkspaceLifecycleError> {
    ensure_workspace_grove_dir(workspace_path)?;
    let agent_marker_path = workspace_path.join(super::GROVE_AGENT_MARKER_FILE);
    fs::write(
        agent_marker_path,
        format!("{}\n", agent_marker_value(agent)),
    )
    .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))
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

fn parse_agent_marker(value: &str) -> Result<AgentType, WorkspaceMarkerError> {
    AgentType::from_marker(value)
        .ok_or_else(|| WorkspaceMarkerError::InvalidAgentMarker(value.to_string()))
}

fn agent_marker_value(agent: AgentType) -> &'static str {
    agent.marker()
}
