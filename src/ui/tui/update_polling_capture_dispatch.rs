use super::*;

impl GroveApp {
    pub(super) fn status_poll_targets_for_async_preview(
        &self,
        live_preview: Option<&LivePreviewTarget>,
    ) -> Vec<WorkspaceStatusTarget> {
        let mut targets = workspace_status_targets_for_polling_with_live_preview(
            &self.state.workspaces,
            live_preview,
        );
        for target in &mut targets {
            target.daemon_socket_path =
                self.remote_session_socket_for_workspace_path(&target.workspace_path);
        }
        targets
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
        let status_poll_targets = self.status_poll_targets_for_async_preview(live_preview.as_ref());

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
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
                (
                    "workspace_status_targets".to_string(),
                    Value::from(usize_to_u64(status_poll_targets.len())),
                ),
                ("source".to_string(), Value::from("normal")),
            ],
        );
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            previous_live_digest,
            cursor_session,
            cursor_daemon_socket_path,
            status_poll_targets,
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
        let status_poll_targets = Vec::new();

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
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
                (
                    "workspace_status_targets".to_string(),
                    Value::from(usize_to_u64(status_poll_targets.len())),
                ),
                ("source".to_string(), Value::from("prioritized")),
            ],
        );
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            previous_live_digest,
            cursor_session,
            cursor_daemon_socket_path,
            status_poll_targets,
        ));
    }

    pub(super) fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
        let completion_generation = completion.generation;
        let live_capture_present = completion.live_capture.is_some();
        let cursor_capture_present = completion.cursor_capture.is_some();
        let status_capture_count = completion.workspace_status_captures.len();
        let attention_marker_count = completion.attention_markers.len();
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

        self.reconcile_workspace_attention_with_markers(completion.attention_markers);

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
}
