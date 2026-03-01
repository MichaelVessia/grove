use std::fs;
use std::path::Path;

use crate::domain::AgentType;

use super::GitAdapterError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MarkerMetadata {
    pub(super) agent: AgentType,
    pub(super) base_branch: Option<String>,
    pub(super) supported_agent: bool,
}

pub(super) fn marker_metadata(path: &Path) -> Result<Option<MarkerMetadata>, GitAdapterError> {
    let Some(agent_value) = read_marker_file(path, ".grove/agent")? else {
        return Ok(None);
    };
    let Some(agent) = AgentType::from_marker(agent_value.trim()) else {
        return Ok(Some(unsupported_marker_metadata()));
    };

    let Some(base_value) = read_marker_file(path, ".grove/base")? else {
        return Ok(Some(unsupported_marker_metadata()));
    };
    let base_branch = base_value.trim().to_string();
    if base_branch.is_empty() {
        return Ok(Some(unsupported_marker_metadata()));
    }

    Ok(Some(MarkerMetadata {
        agent,
        base_branch: Some(base_branch),
        supported_agent: true,
    }))
}

pub(super) fn main_workspace_metadata(path: &Path) -> Result<MarkerMetadata, GitAdapterError> {
    match read_marker_file(path, ".grove/agent")? {
        Some(value) => {
            if let Some(agent) = AgentType::from_marker(value.trim()) {
                return Ok(MarkerMetadata {
                    agent,
                    base_branch: None,
                    supported_agent: true,
                });
            }
            Ok(default_main_marker_metadata())
        }
        None => Ok(default_main_marker_metadata()),
    }
}

fn read_marker_file(path: &Path, relative_marker: &str) -> Result<Option<String>, GitAdapterError> {
    let marker_path = path.join(relative_marker);
    match fs::read_to_string(&marker_path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(GitAdapterError::ParseError(format!(
            "failed reading marker '{}' in '{}': {error}",
            relative_marker,
            path.display()
        ))),
    }
}

fn unsupported_marker_metadata() -> MarkerMetadata {
    MarkerMetadata {
        agent: AgentType::Claude,
        base_branch: None,
        supported_agent: false,
    }
}

fn default_main_marker_metadata() -> MarkerMetadata {
    MarkerMetadata {
        agent: AgentType::Claude,
        base_branch: None,
        supported_agent: true,
    }
}
