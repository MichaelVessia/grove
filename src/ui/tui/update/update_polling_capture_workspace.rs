use super::update_prelude::*;

impl GroveApp {
    pub(super) fn apply_workspace_status_capture(&mut self, capture: WorkspaceStatusCapture) {
        let processing_started_at = Instant::now();
        let supported_agent = capture.supported_agent;
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
                let status_eval_started_at = Instant::now();
                let (changed, cleaned_output) =
                    self.capture_changed_cleaned_for_workspace(&capture.workspace_path, &output);
                let status_eval_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(status_eval_started_at),
                );
                let workspace_path = self.state.workspaces[workspace_index].path.clone();
                let previous_status = self.state.workspaces[workspace_index].status;
                let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
                let workspace_agent = self.state.workspaces[workspace_index].agent;
                let workspace_is_main = self.state.workspaces[workspace_index].is_main;
                let status_detect_started_at = Instant::now();
                let next_status = detect_status_with_session_override(
                    cleaned_output.as_str(),
                    SessionActivity::Active,
                    workspace_is_main,
                    true,
                    supported_agent,
                    workspace_agent,
                    &workspace_path,
                );
                let status_detect_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(status_detect_started_at),
                );
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
                let process_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(processing_started_at),
                );
                self.telemetry.event_log.log(
                    LogEvent::new("preview_poll", "status_capture_completed")
                        .with_data("workspace", Value::from(capture.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(capture.workspace_path.to_string_lossy().to_string()),
                        )
                        .with_data("session", Value::from(capture.session_name))
                        .with_data("supported_agent", Value::from(supported_agent))
                        .with_data("capture_ms", Value::from(capture.capture_ms))
                        .with_data("status_eval_ms", Value::from(status_eval_ms))
                        .with_data("status_detect_ms", Value::from(status_detect_ms))
                        .with_data("process_ms", Value::from(process_ms))
                        .with_data(
                            "total_ms",
                            Value::from(capture.capture_ms.saturating_add(process_ms)),
                        )
                        .with_data("output_bytes", Value::from(usize_to_u64(output.len())))
                        .with_data("changed", Value::from(changed)),
                );
            }
            Err(error) => {
                let missing_session = tmux_capture_error_indicates_missing_session(&error);
                if missing_session {
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
                let process_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(processing_started_at),
                );
                self.telemetry.event_log.log(
                    LogEvent::new("preview_poll", "status_capture_failed")
                        .with_data("workspace", Value::from(capture.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(capture.workspace_path.to_string_lossy().to_string()),
                        )
                        .with_data("session", Value::from(capture.session_name))
                        .with_data("supported_agent", Value::from(supported_agent))
                        .with_data("capture_ms", Value::from(capture.capture_ms))
                        .with_data("process_ms", Value::from(process_ms))
                        .with_data(
                            "total_ms",
                            Value::from(capture.capture_ms.saturating_add(process_ms)),
                        )
                        .with_data("missing_session", Value::from(missing_session))
                        .with_data("error", Value::from(error)),
                );
            }
        }
    }
}
