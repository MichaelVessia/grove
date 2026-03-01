use std::collections::HashSet;

use crate::domain::Workspace;

use super::ReconciliationResult;

pub fn reconcile_with_sessions(
    workspaces: &[Workspace],
    running_sessions: &HashSet<String>,
    previously_running_workspace_names: &HashSet<String>,
) -> ReconciliationResult {
    super::reconcile_with_sessions(
        workspaces,
        running_sessions,
        previously_running_workspace_names,
    )
}
