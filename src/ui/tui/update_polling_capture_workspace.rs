use super::*;

impl GroveApp {
    pub(super) fn apply_workspace_status_capture(&mut self, capture: WorkspaceStatusCapture) {
        let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == capture.workspace_path)
        else {
            self.log_event_with_fields(
                "workspace_status",
                "capture_dropped_missing_workspace",
                [
                    (
                        "workspace_path".to_string(),
                        Value::from(capture.workspace_path.display().to_string()),
                    ),
                    (
                        "session".to_string(),
                        Value::from(capture.session_name.clone()),
                    ),
                ],
            );
            return;
        };

        match capture.result {
            Ok(output) => {
                let key = Self::workspace_status_tracking_key(&capture.workspace_path);
                let previous_digest = self.workspace_status_digests.get(&key);
                let changed_cleaned = previous_digest
                    .is_none_or(|prev| prev.cleaned_hash != output.digest.cleaned_hash);
                let changed_raw = previous_digest.is_none_or(|prev| {
                    prev.raw_hash != output.digest.raw_hash || prev.raw_len != output.digest.raw_len
                });
                self.workspace_status_digests
                    .insert(key.clone(), output.digest.clone());
                self.workspace_output_changing.insert(key, changed_cleaned);
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
                self.log_event_with_fields(
                    "workspace_status",
                    "capture_completed",
                    [
                        (
                            "workspace_name".to_string(),
                            Value::from(capture.workspace_name),
                        ),
                        (
                            "workspace_path".to_string(),
                            Value::from(capture.workspace_path.display().to_string()),
                        ),
                        ("session".to_string(), Value::from(capture.session_name)),
                        (
                            "supported_agent".to_string(),
                            Value::from(capture.supported_agent),
                        ),
                        ("capture_ms".to_string(), Value::from(capture.capture_ms)),
                        ("changed_raw".to_string(), Value::from(changed_raw)),
                        ("changed_cleaned".to_string(), Value::from(changed_cleaned)),
                        ("raw_hash".to_string(), Value::from(output.digest.raw_hash)),
                        (
                            "raw_len".to_string(),
                            Value::from(usize_to_u64(output.digest.raw_len)),
                        ),
                        (
                            "cleaned_hash".to_string(),
                            Value::from(output.digest.cleaned_hash),
                        ),
                        (
                            "cleaned_len".to_string(),
                            Value::from(usize_to_u64(output.cleaned_output.len())),
                        ),
                        (
                            "cleaned_output".to_string(),
                            Value::from(output.cleaned_output),
                        ),
                        (
                            "resolved_status".to_string(),
                            Value::from(Self::workspace_status_name(output.resolved_status)),
                        ),
                    ],
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
                self.log_event_with_fields(
                    "workspace_status",
                    "capture_failed",
                    [
                        (
                            "workspace_name".to_string(),
                            Value::from(capture.workspace_name),
                        ),
                        (
                            "workspace_path".to_string(),
                            Value::from(capture.workspace_path.display().to_string()),
                        ),
                        ("session".to_string(), Value::from(capture.session_name)),
                        (
                            "supported_agent".to_string(),
                            Value::from(capture.supported_agent),
                        ),
                        ("capture_ms".to_string(), Value::from(capture.capture_ms)),
                        ("error".to_string(), Value::from(error)),
                    ],
                );
            }
        }
    }
}
