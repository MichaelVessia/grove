use super::*;

impl GroveApp {
    pub(super) fn status_poll_targets_for_async_preview(
        &self,
        live_preview: Option<&LivePreviewTarget>,
    ) -> Vec<WorkspaceStatusTarget> {
        let targets = workspace_status_targets_for_polling_with_live_preview(
            &self.state.workspaces,
            live_preview,
        );
        if targets.is_empty() {
            return targets;
        }

        let Some(last_polled_at) = self.last_workspace_status_poll_at else {
            return targets;
        };
        let since_last = Instant::now().saturating_duration_since(last_polled_at);
        if since_last >= Duration::from_millis(WORKSPACE_STATUS_POLL_INTERVAL_MS) {
            return targets;
        }

        self.event_log.log(
            LogEvent::new("preview_poll", "status_capture_rate_limited")
                .with_data(
                    "since_last_ms",
                    Value::from(Self::duration_millis(since_last)),
                )
                .with_data(
                    "min_interval_ms",
                    Value::from(WORKSPACE_STATUS_POLL_INTERVAL_MS),
                )
                .with_data("target_count", Value::from(usize_to_u64(targets.len()))),
        );
        Vec::new()
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
            return;
        }

        let live_preview = self.prepare_live_preview_session();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = self.status_poll_targets_for_async_preview(live_preview.as_ref());
        if !status_poll_targets.is_empty() {
            self.last_workspace_status_poll_at = Some(Instant::now());
        }

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
            self.preview_poll_requested = false;
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
            return;
        }

        self.poll_generation = self.poll_generation.saturating_add(1);
        self.preview_poll_in_flight = true;
        self.preview_poll_requested = false;
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            cursor_session,
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
            })
        });
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = Vec::new();

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
            self.preview_poll_requested = false;
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
            return;
        }

        self.poll_generation = self.poll_generation.saturating_add(1);
        self.preview_poll_requested = false;
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            cursor_session,
            status_poll_targets,
        ));
    }

    pub(super) fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
        if completion.generation < self.poll_generation {
            self.event_log.log(
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
                self.event_log.log(event);
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

        if let Some(cursor_capture) = completion.cursor_capture {
            self.apply_cursor_capture_result(cursor_capture);
        }

        if self.preview_poll_requested {
            self.preview_poll_requested = false;
            self.poll_preview();
        }
    }
}
