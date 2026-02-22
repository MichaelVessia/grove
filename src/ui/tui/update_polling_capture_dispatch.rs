use super::*;

impl GroveApp {
    pub(super) fn workspace_status_poll_targets_for_background(
        &self,
    ) -> Vec<WorkspaceStatusTarget> {
        let live_preview_session = self.selected_live_preview_session_if_ready();
        let mut targets = workspace_status_targets_for_polling(
            &self.state.workspaces,
            live_preview_session.as_deref(),
        );
        for target in &mut targets {
            target.daemon_socket_path =
                self.remote_session_socket_for_workspace_path(&target.workspace_path);
        }
        targets
    }

    pub(super) fn capped_workspace_status_poll_targets(
        &mut self,
        targets: Vec<WorkspaceStatusTarget>,
    ) -> Vec<WorkspaceStatusTarget> {
        if targets.is_empty() {
            self.workspace_status_poll_cursor = 0;
            return Vec::new();
        }

        let total_targets = targets.len();
        let start_index = self.workspace_status_poll_cursor % total_targets;
        let cycle_targets = WORKSPACE_STATUS_POLL_MAX_TARGETS_PER_CYCLE.min(total_targets);
        let mut selected = Vec::with_capacity(cycle_targets);
        for offset in 0..cycle_targets {
            let index = (start_index + offset) % total_targets;
            selected.push(targets[index].clone());
        }
        self.workspace_status_poll_cursor = (start_index + cycle_targets) % total_targets;
        selected
    }

    pub(super) fn selected_live_preview_session_if_ready(&self) -> Option<String> {
        match self.preview_tab {
            PreviewTab::Git => {
                let workspace = self.state.selected_workspace()?;
                let session_name = git_session_name_for_workspace(workspace);
                self.lazygit_sessions
                    .is_ready(&session_name)
                    .then_some(session_name)
            }
            PreviewTab::Shell => self.selected_shell_preview_session_if_ready(),
            PreviewTab::Agent => self.selected_agent_preview_session_if_ready(),
        }
    }

    fn selected_live_preview_session_for_completion(&self) -> Option<String> {
        if matches!(self.preview_tab, PreviewTab::Git | PreviewTab::Shell) {
            return self.selected_live_preview_session_if_ready();
        }

        self.selected_live_preview_session_if_ready().or_else(|| {
            let workspace = self.state.selected_workspace()?;
            if !workspace.supported_agent {
                return None;
            }
            Some(session_name_for_workspace_ref(workspace))
        })
    }

    pub(super) fn poll_preview(&mut self) {
        if !self.tmux_input.supports_background_poll() {
            self.poll_preview_sync();
            return;
        }
        if self.preview_poll_in_flight {
            self.preview_poll_requested = true;
            self.log_event_with_fields(
                "preview_poll",
                "requested_while_in_flight",
                [("generation".to_string(), Value::from(self.poll_generation))],
            );
            return;
        }

        let live_preview = self.prepare_live_preview_session();
        let cursor_session = self.interactive_target_session();
        let cursor_daemon_socket_path = self.interactive_daemon_socket_path();

        if live_preview.is_none() && cursor_session.is_none() {
            self.preview_poll_requested = false;
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
            self.log_event_with_fields(
                "preview_poll",
                "skipped_no_targets",
                [("generation".to_string(), Value::from(self.poll_generation))],
            );
            return;
        }

        let previous_live_digest = if live_preview.is_some() {
            self.preview.last_digest().cloned()
        } else {
            None
        };
        self.poll_generation = self.poll_generation.saturating_add(1);
        self.preview_poll_in_flight = true;
        self.preview_poll_requested = false;
        self.preview_poll_started_at = Some(Instant::now());
        self.log_event_with_fields(
            "preview_poll",
            "cycle_started",
            [
                ("generation".to_string(), Value::from(self.poll_generation)),
                (
                    "live_capture_targeted".to_string(),
                    Value::from(live_preview.is_some()),
                ),
                (
                    "cursor_capture_targeted".to_string(),
                    Value::from(cursor_session.is_some()),
                ),
                ("workspace_status_targets".to_string(), Value::from(0)),
                ("source".to_string(), Value::from("normal")),
            ],
        );
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            previous_live_digest,
            cursor_session,
            cursor_daemon_socket_path,
        ));
    }

    pub(super) fn poll_preview_prioritized(&mut self) {
        if !self.tmux_input.supports_background_poll() || !self.preview_poll_in_flight {
            self.poll_preview();
            return;
        }

        let live_preview = self.prepare_live_preview_session().or_else(|| {
            if self.preview_tab != PreviewTab::Agent {
                return None;
            }
            let workspace = self.state.selected_workspace()?;
            if !workspace.supported_agent {
                return None;
            }
            Some(LivePreviewTarget {
                session_name: session_name_for_workspace_ref(workspace),
                include_escape_sequences: true,
                daemon_socket_path: self.remote_session_socket_for_workspace(workspace),
                status_context: Some(LivePreviewStatusContext {
                    workspace_path: workspace.path.clone(),
                    is_main: workspace.is_main,
                    supported_agent: workspace.supported_agent,
                    agent: workspace.agent,
                }),
            })
        });
        let cursor_session = self.interactive_target_session();
        let cursor_daemon_socket_path = self.interactive_daemon_socket_path();

        if live_preview.is_none() && cursor_session.is_none() {
            self.preview_poll_requested = false;
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
            return;
        }

        let previous_live_digest = if live_preview.is_some() {
            self.preview.last_digest().cloned()
        } else {
            None
        };
        self.poll_generation = self.poll_generation.saturating_add(1);
        self.preview_poll_requested = false;
        self.preview_poll_started_at = Some(Instant::now());
        self.log_event_with_fields(
            "preview_poll",
            "cycle_started",
            [
                ("generation".to_string(), Value::from(self.poll_generation)),
                (
                    "live_capture_targeted".to_string(),
                    Value::from(live_preview.is_some()),
                ),
                (
                    "cursor_capture_targeted".to_string(),
                    Value::from(cursor_session.is_some()),
                ),
                ("workspace_status_targets".to_string(), Value::from(0)),
                ("source".to_string(), Value::from("prioritized")),
            ],
        );
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            previous_live_digest,
            cursor_session,
            cursor_daemon_socket_path,
        ));
    }

    pub(super) fn poll_workspace_statuses_background(&mut self) {
        if !self.tmux_input.supports_background_poll() {
            return;
        }
        if self.workspace_status_poll_in_flight {
            self.workspace_status_poll_requested = true;
            self.log_event_with_fields(
                "workspace_status_poll",
                "requested_while_in_flight",
                [(
                    "pending_depth".to_string(),
                    Value::from(self.pending_input_depth()),
                )],
            );
            return;
        }

        let all_targets = self.workspace_status_poll_targets_for_background();
        if all_targets.is_empty() {
            self.workspace_status_poll_requested = false;
            self.log_event_with_fields(
                "workspace_status_poll",
                "skipped_no_targets",
                [(
                    "workspace_status_poll_cursor".to_string(),
                    Value::from(usize_to_u64(self.workspace_status_poll_cursor)),
                )],
            );
            return;
        }

        let total_targets = all_targets.len();
        let cycle_targets = self.capped_workspace_status_poll_targets(all_targets);
        self.workspace_status_poll_in_flight = true;
        self.workspace_status_poll_requested = false;
        self.workspace_status_poll_started_at = Some(Instant::now());
        self.log_event_with_fields(
            "workspace_status_poll",
            "cycle_started",
            [
                (
                    "cycle_targets".to_string(),
                    Value::from(usize_to_u64(cycle_targets.len())),
                ),
                (
                    "total_targets".to_string(),
                    Value::from(usize_to_u64(total_targets)),
                ),
                (
                    "workspace_status_poll_cursor".to_string(),
                    Value::from(usize_to_u64(self.workspace_status_poll_cursor)),
                ),
            ],
        );
        self.queue_cmd(self.schedule_async_workspace_status_poll(cycle_targets));
    }

    pub(super) fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
        let completion_generation = completion.generation;
        let live_capture_present = completion.live_capture.is_some();
        let cursor_capture_present = completion.cursor_capture.is_some();
        let status_capture_count = completion.workspace_status_captures.len();
        let attention_marker_count = completion.attention_markers.len();
        let mut attention_polled_paths = completion
            .workspace_status_captures
            .iter()
            .filter(|capture| capture.supported_agent)
            .map(|capture| capture.workspace_path.clone())
            .collect::<Vec<PathBuf>>();
        if let Some(live_capture) = completion.live_capture.as_ref()
            && let Ok(output) = live_capture.result.as_ref()
            && let Some(resolved) = output.resolved_status.as_ref()
            && resolved.supported_agent
        {
            attention_polled_paths.push(resolved.workspace_path.clone());
        }
        if completion.generation < self.poll_generation {
            self.emit_event(
                LogEvent::new("preview_poll", "stale_result_dropped")
                    .with_data("generation", Value::from(completion.generation))
                    .with_data("latest_generation", Value::from(self.poll_generation)),
            );
            return;
        }

        self.preview_poll_in_flight = false;
        if completion.generation > self.poll_generation {
            self.poll_generation = completion.generation;
        }

        let mut had_live_capture = false;
        if let Some(live_capture) = completion.live_capture {
            let selected_live_session = self.selected_live_preview_session_for_completion();
            if selected_live_session.as_deref() == Some(live_capture.session.as_str()) {
                had_live_capture = true;
                self.apply_live_preview_capture(
                    &live_capture.session,
                    live_capture.include_escape_sequences,
                    live_capture.capture_ms,
                    live_capture.total_ms,
                    live_capture.result,
                );
            } else {
                let mut event = LogEvent::new("preview_poll", "session_mismatch_dropped")
                    .with_data("captured_session", Value::from(live_capture.session));
                if let Some(selected_session) = selected_live_session {
                    event = event.with_data("selected_session", Value::from(selected_session));
                }
                self.emit_event(event);
                self.clear_agent_activity_tracking();
                if self
                    .selected_live_preview_session_for_completion()
                    .is_none()
                {
                    self.refresh_preview_summary();
                }
            }
        } else {
            self.clear_agent_activity_tracking();
            if self
                .selected_live_preview_session_for_completion()
                .is_none()
            {
                self.refresh_preview_summary();
            }
        }

        for status_capture in completion.workspace_status_captures {
            self.apply_workspace_status_capture(status_capture);
        }
        if !had_live_capture
            && self
                .selected_live_preview_session_for_completion()
                .is_none()
        {
            self.refresh_preview_summary();
        }

        self.reconcile_workspace_attention_with_marker_updates(
            attention_polled_paths,
            completion.attention_markers,
        );

        if let Some(cursor_capture) = completion.cursor_capture {
            self.apply_cursor_capture_result(cursor_capture);
        }

        if self.preview_poll_requested {
            self.preview_poll_requested = false;
            self.poll_preview();
        }
        let cycle_duration_ms = self
            .preview_poll_started_at
            .take()
            .map(|started_at| {
                Self::duration_millis(Instant::now().saturating_duration_since(started_at))
            })
            .unwrap_or(0);
        self.log_event_with_fields(
            "preview_poll",
            "cycle_completed",
            [
                ("generation".to_string(), Value::from(completion_generation)),
                ("duration_ms".to_string(), Value::from(cycle_duration_ms)),
                (
                    "live_capture_present".to_string(),
                    Value::from(live_capture_present),
                ),
                (
                    "cursor_capture_present".to_string(),
                    Value::from(cursor_capture_present),
                ),
                (
                    "workspace_status_capture_count".to_string(),
                    Value::from(usize_to_u64(status_capture_count)),
                ),
                (
                    "attention_marker_count".to_string(),
                    Value::from(usize_to_u64(attention_marker_count)),
                ),
            ],
        );
    }

    pub(super) fn handle_workspace_status_poll_completed(
        &mut self,
        completion: WorkspaceStatusPollCompletion,
    ) {
        let status_capture_count = completion.workspace_status_captures.len();
        let attention_marker_count = completion.attention_markers.len();
        let attention_polled_paths = completion
            .workspace_status_captures
            .iter()
            .filter(|capture| capture.supported_agent)
            .map(|capture| capture.workspace_path.clone())
            .collect::<Vec<PathBuf>>();

        self.workspace_status_poll_in_flight = false;
        for status_capture in completion.workspace_status_captures {
            self.apply_workspace_status_capture(status_capture);
        }
        self.reconcile_workspace_attention_with_marker_updates(
            attention_polled_paths,
            completion.attention_markers,
        );
        if self.workspace_status_poll_requested {
            self.workspace_status_poll_requested = false;
            self.poll_workspace_statuses_background();
        }
        let cycle_duration_ms = self
            .workspace_status_poll_started_at
            .take()
            .map(|started_at| {
                Self::duration_millis(Instant::now().saturating_duration_since(started_at))
            })
            .unwrap_or(0);
        self.log_event_with_fields(
            "workspace_status_poll",
            "cycle_completed",
            [
                ("duration_ms".to_string(), Value::from(cycle_duration_ms)),
                (
                    "workspace_status_capture_count".to_string(),
                    Value::from(usize_to_u64(status_capture_count)),
                ),
                (
                    "attention_marker_count".to_string(),
                    Value::from(usize_to_u64(attention_marker_count)),
                ),
            ],
        );
    }
}
