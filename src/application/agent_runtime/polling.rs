use std::time::Duration;

use crate::domain::{Workspace, WorkspaceStatus};

use super::sessions::session_name_for_workspace_ref;
use super::{LivePreviewTarget, WorkspaceStatusTarget};

const FOCUSED_PREVIEW_POLL_INTERVAL_MS: u64 = 100;
const SELECTED_TERMINAL_POLL_INTERVAL_MS: u64 = 500;

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

    if is_preview_focused {
        return match status {
            WorkspaceStatus::Done | WorkspaceStatus::Error => {
                Duration::from_millis(SELECTED_TERMINAL_POLL_INTERVAL_MS)
            }
            _ => Duration::from_millis(FOCUSED_PREVIEW_POLL_INTERVAL_MS),
        };
    }

    if output_changing {
        return Duration::from_millis(200);
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use crate::domain::{AgentType, WorkspaceStatus};

    use super::super::LivePreviewTarget;
    use super::{
        poll_interval, workspace_should_poll_status, workspace_status_session_target,
        workspace_status_targets_for_polling,
        workspace_status_targets_for_polling_with_live_preview,
    };

    fn fixture_workspace(name: &str, is_main: bool) -> crate::domain::Workspace {
        crate::domain::Workspace::try_new(
            name.to_string(),
            PathBuf::from(format!("/repos/grove-{name}")),
            if is_main {
                "main".to_string()
            } else {
                name.to_string()
            },
            Some(1_700_000_100),
            AgentType::Claude,
            if is_main {
                WorkspaceStatus::Main
            } else {
                WorkspaceStatus::Idle
            },
            is_main,
        )
        .expect("workspace should be valid")
    }

    #[test]
    fn workspace_status_poll_policy_requires_supported_agent() {
        let workspace = fixture_workspace("feature", false).with_supported_agent(false);
        assert!(!workspace_should_poll_status(&workspace));
    }

    #[test]
    fn workspace_status_session_target_skips_selected_live_session() {
        let mut workspace = fixture_workspace("feature", false);
        workspace.status = WorkspaceStatus::Active;

        assert_eq!(
            workspace_status_session_target(&workspace, None),
            Some("grove-ws-feature".to_string())
        );
        assert_eq!(
            workspace_status_session_target(&workspace, Some("grove-ws-feature")),
            None
        );
    }

    #[test]
    fn workspace_status_targets_for_polling_skip_selected_session() {
        let mut selected = fixture_workspace("selected", false);
        selected.status = WorkspaceStatus::Active;
        let mut other = fixture_workspace("other", false);
        other.status = WorkspaceStatus::Active;
        let workspaces = vec![selected, other];

        let targets = workspace_status_targets_for_polling(&workspaces, Some("grove-ws-selected"));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].workspace_name, "other");
        assert_eq!(targets[0].session_name, "grove-ws-other");
    }

    #[test]
    fn workspace_status_targets_for_polling_with_live_preview_skips_selected_session() {
        let mut selected = fixture_workspace("selected", false);
        selected.status = WorkspaceStatus::Active;
        let mut other = fixture_workspace("other", false);
        other.status = WorkspaceStatus::Active;
        let workspaces = vec![selected, other];

        let live_preview = LivePreviewTarget {
            session_name: "grove-ws-selected".to_string(),
            include_escape_sequences: true,
        };
        let targets = workspace_status_targets_for_polling_with_live_preview(
            &workspaces,
            Some(&live_preview),
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].workspace_name, "other");
    }

    #[test]
    fn poll_intervals_follow_preview_and_interactive_rules() {
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_millis(100),
                true
            ),
            Duration::from_millis(50)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(5),
                true
            ),
            Duration::from_millis(200)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(15),
                false
            ),
            Duration::from_millis(500)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Waiting,
                true,
                true,
                false,
                Duration::from_secs(30),
                false
            ),
            Duration::from_millis(100)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                true,
                false,
                Duration::from_secs(30),
                true
            ),
            Duration::from_millis(100)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                false,
                false,
                false,
                Duration::from_secs(30),
                true
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Done,
                true,
                false,
                false,
                Duration::from_secs(30),
                false
            ),
            Duration::from_secs(20)
        );
    }
}
