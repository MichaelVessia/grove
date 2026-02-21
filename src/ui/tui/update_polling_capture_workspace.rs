use super::*;

impl GroveApp {
    pub(super) fn apply_workspace_status_capture(&mut self, capture: WorkspaceStatusCapture) {
        let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == capture.workspace_path)
        else {
            return;
        };

        match capture.result {
            Ok(output) => {
                let key = Self::workspace_status_tracking_key(&capture.workspace_path);
                let previous_digest = self.workspace_status_digests.get(&key);
                let changed = previous_digest
                    .is_none_or(|prev| prev.cleaned_hash != output.digest.cleaned_hash);
                self.workspace_status_digests
                    .insert(key.clone(), output.digest);
                self.workspace_output_changing.insert(key, changed);
                let workspace_path = self.state.workspaces[workspace_index].path.clone();
                let previous_status = self.state.workspaces[workspace_index].status;
                let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
                let next_status = output.resolved_status;
                let workspace = &mut self.state.workspaces[workspace_index];
                workspace.status = next_status;
                workspace.is_orphaned = false;
                self.track_workspace_status_transition(
                    &workspace_path,
                    previous_status,
                    next_status,
                    previous_orphaned,
                    false,
                );
            }
            Err(error) => {
                if tmux_capture_error_indicates_missing_session(&error) {
                    let previous_status = self.state.workspaces[workspace_index].status;
                    let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
                    let workspace_is_main = self.state.workspaces[workspace_index].is_main;
                    let previously_had_live_session = previous_status.has_session();
                    let next_status = if workspace_is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    let next_orphaned = if workspace_is_main {
                        false
                    } else {
                        previously_had_live_session || previous_orphaned
                    };
                    let workspace = &mut self.state.workspaces[workspace_index];
                    workspace.status = next_status;
                    workspace.is_orphaned = next_orphaned;
                    self.clear_status_tracking_for_workspace_path(&capture.workspace_path);
                    self.track_workspace_status_transition(
                        &capture.workspace_path,
                        previous_status,
                        next_status,
                        previous_orphaned,
                        next_orphaned,
                    );
                }
            }
        }
    }
}
