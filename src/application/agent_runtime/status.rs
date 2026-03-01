use std::path::Path;

use crate::domain::{AgentType, WorkspaceStatus};

use super::SessionActivity;

pub(crate) fn detect_status_with_session_override(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &Path,
) -> WorkspaceStatus {
    super::detect_status_with_session_override(
        output,
        session_activity,
        is_main,
        has_live_session,
        supported_agent,
        agent,
        workspace_path,
    )
}

pub(crate) fn latest_assistant_attention_marker(
    agent: AgentType,
    workspace_path: &Path,
) -> Option<String> {
    super::latest_assistant_attention_marker(agent, workspace_path)
}
