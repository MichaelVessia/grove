use std::time::Duration;

use crate::domain::{Workspace, WorkspaceStatus};

use super::{LivePreviewTarget, WorkspaceStatusTarget};

pub fn workspace_status_targets_for_polling_with_live_preview(
    workspaces: &[Workspace],
    live_preview: Option<&LivePreviewTarget>,
) -> Vec<WorkspaceStatusTarget> {
    super::workspace_status_targets_for_polling_with_live_preview(workspaces, live_preview)
}

pub fn poll_interval(
    status: WorkspaceStatus,
    is_selected: bool,
    is_preview_focused: bool,
    interactive_mode: bool,
    since_last_key: Duration,
    output_changing: bool,
) -> Duration {
    super::poll_interval(
        status,
        is_selected,
        is_preview_focused,
        interactive_mode,
        since_last_key,
        output_changing,
    )
}
