use std::collections::HashSet;

use crate::domain::Workspace;

use super::sessions::session_name_for_workspace_in_project;
use super::status::detect_status;
use super::{ReconciliationResult, SessionActivity};

pub fn reconcile_with_sessions(
    mut workspaces: Vec<Workspace>,
    running_sessions: &HashSet<String>,
    previously_running_workspace_names: &HashSet<String>,
) -> ReconciliationResult {
    let mut matched_sessions = HashSet::with_capacity(running_sessions.len());

    for workspace in &mut workspaces {
        let session_name = session_name_for_workspace_in_project(
            workspace.project_name.as_deref(),
            &workspace.name,
        );
        let has_live_session = running_sessions.contains(&session_name);
        if has_live_session {
            matched_sessions.insert(session_name);
            workspace.status = detect_status(
                "",
                SessionActivity::Active,
                workspace.is_main,
                true,
                workspace.supported_agent,
            );
            workspace.is_orphaned = false;
        } else {
            workspace.status = detect_status(
                "",
                SessionActivity::Idle,
                workspace.is_main,
                false,
                workspace.supported_agent,
            );
            workspace.is_orphaned = if workspace.is_main {
                false
            } else {
                previously_running_workspace_names.contains(&workspace.name)
            };
        }
    }

    let mut orphaned_sessions: Vec<String> = running_sessions
        .iter()
        .filter(|session_name| !matched_sessions.contains(*session_name))
        .cloned()
        .collect();
    orphaned_sessions.sort();

    ReconciliationResult {
        workspaces,
        orphaned_sessions,
    }
}
