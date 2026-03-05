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

pub(super) fn marker_metadata(path: &Path) -> Result<MarkerMetadata, GitAdapterError> {
    let base_branch = read_marker_file(path, ".grove/base")?.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    });
    Ok(MarkerMetadata {
        agent: AgentType::Claude,
        base_branch,
        supported_agent: true,
    })
}

pub(super) fn main_workspace_metadata(path: &Path) -> Result<MarkerMetadata, GitAdapterError> {
    let _ = path;
    Ok(default_main_marker_metadata())
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

fn default_main_marker_metadata() -> MarkerMetadata {
    MarkerMetadata {
        agent: AgentType::Claude,
        base_branch: None,
        supported_agent: true,
    }
}
