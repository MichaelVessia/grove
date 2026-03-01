use std::time::Duration;

use crate::domain::{Workspace, WorkspaceStatus};

use super::sessions::session_name_for_workspace_ref;
use super::{LivePreviewTarget, WorkspaceStatusTarget};

pub fn workspace_should_poll_status(workspace: &Workspace) -> bool {
    if !workspace.supported_agent {
        return false;
    }

    workspace.status.has_session()
}

pub fn workspace_status_session_target(
    workspace: &Workspace,
    selected_live_session: Option<&str>,
) -> Option<String> {
    if !workspace_should_poll_status(workspace) {
        return None;
    }

    let session_name = session_name_for_workspace_ref(workspace);
    if selected_live_session == Some(session_name.as_str()) {
        return None;
    }

    Some(session_name)
}

pub fn workspace_status_targets_for_polling(
    workspaces: &[Workspace],
    selected_live_session: Option<&str>,
) -> Vec<WorkspaceStatusTarget> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            let session_name = workspace_status_session_target(workspace, selected_live_session)?;
            Some(WorkspaceStatusTarget {
                workspace_name: workspace.name.clone(),
                workspace_path: workspace.path.clone(),
                session_name,
                supported_agent: workspace.supported_agent,
            })
        })
        .collect()
}

pub fn workspace_status_targets_for_polling_with_live_preview(
    workspaces: &[Workspace],
    live_preview: Option<&LivePreviewTarget>,
) -> Vec<WorkspaceStatusTarget> {
    workspace_status_targets_for_polling(
        workspaces,
        live_preview.map(|target| target.session_name.as_str()),
    )
}

pub fn poll_interval(
    status: WorkspaceStatus,
    is_selected: bool,
    is_preview_focused: bool,
    interactive_mode: bool,
    since_last_key: Duration,
    output_changing: bool,
) -> Duration {
    if interactive_mode && is_selected {
        if since_last_key < Duration::from_secs(2) {
            return Duration::from_millis(50);
        }
        if since_last_key < Duration::from_secs(10) {
            return Duration::from_millis(200);
        }
        return Duration::from_millis(500);
    }

    if !is_selected {
        return Duration::from_secs(10);
    }

    if output_changing {
        return Duration::from_millis(200);
    }

    if is_preview_focused {
        return Duration::from_millis(500);
    }

    match status {
        WorkspaceStatus::Active | WorkspaceStatus::Thinking => Duration::from_millis(200),
        WorkspaceStatus::Waiting | WorkspaceStatus::Idle => Duration::from_secs(2),
        WorkspaceStatus::Done | WorkspaceStatus::Error => Duration::from_secs(20),
        WorkspaceStatus::Main | WorkspaceStatus::Unknown | WorkspaceStatus::Unsupported => {
            Duration::from_secs(2)
        }
    }
}
