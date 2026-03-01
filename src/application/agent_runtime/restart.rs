use crate::domain::Workspace;

use super::SessionExecutionResult;

pub fn restart_workspace(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    super::execute_restart_workspace_in_pane_with_result(workspace, skip_permissions, agent_env)
}
