use super::update_prelude::*;

impl GroveApp {
    pub(super) fn apply_live_preview_capture(
        &mut self,
        session_name: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
        capture_ms: u64,
        base_total_ms: u64,
        result: Result<String, String>,
    ) {
        match result {
            Ok(output) => {
                let processing_started_at = Instant::now();
                let apply_started_at = Instant::now();
                let update = self.preview.apply_capture(&output);
                let apply_capture_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(apply_started_at),
                );
                let drain_started_at = Instant::now();
                let consumed_inputs = if update.changed_cleaned {
                    self.drain_pending_inputs_for_session(session_name)
                } else {
                    Vec::new()
                };
                let drain_pending_inputs_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(drain_started_at),
                );
                self.polling.output_changing = update.changed_cleaned;
                self.polling.agent_output_changing =
                    update.changed_cleaned && consumed_inputs.is_empty();
                self.push_agent_activity_frame(self.polling.agent_output_changing);
                let mut workspace_status_eval_ms = 0;
                let mut workspace_status_changed = false;
                let mut status_detect_ms = 0;
                let selected_workspace_index = self
                    .state
                    .selected_workspace()
                    .filter(|workspace| session_name_for_workspace_ref(workspace) == session_name)
                    .map(|_| self.state.selected_index);
                if let Some(index) = selected_workspace_index {
                    let supported_agent = self.state.workspaces[index].supported_agent;
                    let workspace_path = self.state.workspaces[index].path.clone();
                    let workspace_agent = self.state.workspaces[index].agent;
                    let workspace_is_main = self.state.workspaces[index].is_main;
                    let previous_status = self.state.workspaces[index].status;
                    let previous_orphaned = self.state.workspaces[index].is_orphaned;
                    let workspace_status_started_at = Instant::now();
                    let (changed_cleaned, cleaned_output) = self
                        .capture_changed_cleaned_for_workspace(&workspace_path, output.as_str());
                    workspace_status_eval_ms = Self::duration_millis(
                        Instant::now().saturating_duration_since(workspace_status_started_at),
                    );
                    workspace_status_changed = changed_cleaned;
                    let status_detect_started_at = Instant::now();
                    let resolved_status = detect_status_with_session_override(
                        cleaned_output.as_str(),
                        SessionActivity::Active,
                        workspace_is_main,
                        true,
                        supported_agent,
                        workspace_agent,
                        &workspace_path,
                    );
                    status_detect_ms = Self::duration_millis(
                        Instant::now().saturating_duration_since(status_detect_started_at),
                    );
                    let workspace = &mut self.state.workspaces[index];
                    workspace.status = resolved_status;
                    workspace.is_orphaned = false;
                    self.track_workspace_status_transition(
                        &workspace_path,
                        previous_status,
                        resolved_status,
                        previous_orphaned,
                        false,
                    );
                }
                self.session.last_tmux_error = None;
                let pipeline_process_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(processing_started_at),
                );
                self.telemetry.event_log.log(
                    LogEvent::new("preview_poll", "capture_completed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data(
                            "scrollback_lines",
                            Value::from(usize_to_u64(scrollback_lines)),
                        )
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data("apply_capture_ms", Value::from(apply_capture_ms))
                        .with_data(
                            "drain_pending_inputs_ms",
                            Value::from(drain_pending_inputs_ms),
                        )
                        .with_data(
                            "workspace_status_eval_ms",
                            Value::from(workspace_status_eval_ms),
                        )
                        .with_data(
                            "workspace_status_changed",
                            Value::from(workspace_status_changed),
                        )
                        .with_data("status_detect_ms", Value::from(status_detect_ms))
                        .with_data("pipeline_process_ms", Value::from(pipeline_process_ms))
                        .with_data(
                            "pipeline_total_ms",
                            Value::from(base_total_ms.saturating_add(pipeline_process_ms)),
                        )
                        .with_data(
                            "total_ms",
                            Value::from(base_total_ms.saturating_add(apply_capture_ms)),
                        )
                        .with_data("output_bytes", Value::from(usize_to_u64(output.len())))
                        .with_data("changed", Value::from(update.changed_cleaned))
                        .with_data("changed_raw", Value::from(update.changed_raw))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        ),
                );
                if update.changed_cleaned {
                    let line_count = usize_to_u64(self.preview.lines.len());
                    let now = Instant::now();
                    let mut output_event = LogEvent::new("preview_update", "output_changed")
                        .with_data("line_count", Value::from(line_count))
                        .with_data("session", Value::from(session_name.to_string()));
                    if let Some(first_input) = consumed_inputs.first() {
                        let last_input = consumed_inputs.last().unwrap_or(first_input);
                        let oldest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.received_at),
                        );
                        let newest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.received_at),
                        );
                        let oldest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.forwarded_at),
                        );
                        let newest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.forwarded_at),
                        );
                        let consumed_count = usize_to_u64(consumed_inputs.len());
                        let consumed_seq_first = first_input.seq;
                        let consumed_seq_last = last_input.seq;

                        output_event = output_event
                            .with_data("input_seq", Value::from(consumed_seq_first))
                            .with_data(
                                "input_to_preview_ms",
                                Value::from(oldest_input_to_preview_ms),
                            )
                            .with_data("tmux_to_preview_ms", Value::from(oldest_tmux_to_preview_ms))
                            .with_data("consumed_input_count", Value::from(consumed_count))
                            .with_data("consumed_input_seq_first", Value::from(consumed_seq_first))
                            .with_data("consumed_input_seq_last", Value::from(consumed_seq_last))
                            .with_data(
                                "newest_input_to_preview_ms",
                                Value::from(newest_input_to_preview_ms),
                            )
                            .with_data(
                                "newest_tmux_to_preview_ms",
                                Value::from(newest_tmux_to_preview_ms),
                            );

                        self.log_input_event_with_fields(
                            "interactive_input_to_preview",
                            consumed_seq_first,
                            vec![
                                ("session".to_string(), Value::from(session_name.to_string())),
                                (
                                    "input_to_preview_ms".to_string(),
                                    Value::from(oldest_input_to_preview_ms),
                                ),
                                (
                                    "tmux_to_preview_ms".to_string(),
                                    Value::from(oldest_tmux_to_preview_ms),
                                ),
                                (
                                    "newest_input_to_preview_ms".to_string(),
                                    Value::from(newest_input_to_preview_ms),
                                ),
                                (
                                    "newest_tmux_to_preview_ms".to_string(),
                                    Value::from(newest_tmux_to_preview_ms),
                                ),
                                (
                                    "consumed_input_count".to_string(),
                                    Value::from(consumed_count),
                                ),
                                (
                                    "consumed_input_seq_first".to_string(),
                                    Value::from(consumed_seq_first),
                                ),
                                (
                                    "consumed_input_seq_last".to_string(),
                                    Value::from(consumed_seq_last),
                                ),
                                (
                                    "queue_depth".to_string(),
                                    Value::from(self.pending_input_depth()),
                                ),
                            ],
                        );
                        if consumed_inputs.len() > 1 {
                            self.log_input_event_with_fields(
                                "interactive_inputs_coalesced",
                                consumed_seq_first,
                                vec![
                                    ("session".to_string(), Value::from(session_name.to_string())),
                                    (
                                        "consumed_input_count".to_string(),
                                        Value::from(consumed_count),
                                    ),
                                    (
                                        "consumed_input_seq_last".to_string(),
                                        Value::from(consumed_seq_last),
                                    ),
                                ],
                            );
                        }
                    }
                    self.telemetry.event_log.log(output_event);
                }
            }
            Err(message) => {
                self.clear_agent_activity_tracking();
                let capture_error_indicates_missing_session =
                    tmux_capture_error_indicates_missing_session(&message);
                if capture_error_indicates_missing_session {
                    self.session.lazygit_sessions.remove_ready(session_name);
                    self.session.shell_sessions.remove_ready(session_name);
                    let selected_workspace_index = self
                        .state
                        .selected_workspace()
                        .filter(|workspace| {
                            session_name_for_workspace_ref(workspace) == session_name
                        })
                        .map(|_| self.state.selected_index);
                    if let Some(index) = selected_workspace_index {
                        let workspace_path = self.state.workspaces[index].path.clone();
                        let previous_status = self.state.workspaces[index].status;
                        let previous_orphaned = self.state.workspaces[index].is_orphaned;
                        let next_status = if self.state.workspaces[index].is_main {
                            WorkspaceStatus::Main
                        } else {
                            WorkspaceStatus::Idle
                        };
                        let next_orphaned = !self.state.workspaces[index].is_main;
                        let workspace = &mut self.state.workspaces[index];
                        workspace.status = next_status;
                        workspace.is_orphaned = next_orphaned;
                        self.track_workspace_status_transition(
                            &workspace_path,
                            previous_status,
                            next_status,
                            previous_orphaned,
                            next_orphaned,
                        );
                        self.clear_status_tracking_for_workspace_path(&workspace_path);
                    }
                    if self
                        .session
                        .interactive
                        .as_ref()
                        .is_some_and(|interactive| interactive.target_session == session_name)
                    {
                        self.session.interactive = None;
                    }
                }
                self.telemetry.event_log.log(
                    LogEvent::new("preview_poll", "capture_failed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data(
                            "scrollback_lines",
                            Value::from(usize_to_u64(scrollback_lines)),
                        )
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        )
                        .with_data("error", Value::from(message.clone())),
                );
                if capture_error_indicates_missing_session {
                    self.session.last_tmux_error = None;
                } else {
                    self.session.last_tmux_error = Some(message.clone());
                    self.log_tmux_error(message);
                    self.show_error_toast("preview capture failed");
                }
                self.refresh_preview_summary();
            }
        }
    }

    pub(super) fn scroll_preview(&mut self, delta: i32) {
        let viewport_height = self
            .preview_output_dimensions()
            .map_or(1, |(_, height)| usize::from(height));
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        let changed = self.preview.scroll(delta, Instant::now(), viewport_height);
        if changed {
            let offset = usize_to_u64(self.preview.offset);
            self.telemetry.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from(i64::from(delta)))
                    .with_data("offset", Value::from(offset)),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.telemetry.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data("offset", Value::from(usize_to_u64(self.preview.offset)))
                    .with_data("previous_offset", Value::from(usize_to_u64(old_offset))),
            );
        }
    }

    pub(super) fn jump_preview_to_bottom(&mut self) {
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        self.preview.jump_to_bottom();
        if old_offset != self.preview.offset {
            self.telemetry.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from("jump_bottom"))
                    .with_data("offset", Value::from(usize_to_u64(self.preview.offset)))
                    .with_data("previous_offset", Value::from(usize_to_u64(old_offset))),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.telemetry.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data("offset", Value::from(usize_to_u64(self.preview.offset))),
            );
        }
    }
}
