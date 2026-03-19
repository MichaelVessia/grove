use super::update_prelude::*;

impl GroveApp {
    pub(super) fn status_poll_targets_for_async_preview(
        &self,
        live_preview: Option<&LivePreviewTarget>,
    ) -> Vec<WorkspaceStatusTarget> {
        let selected_live_session = live_preview.map(|target| target.session_name.as_str());
        let targets = self
            .state
            .workspaces
            .iter()
            .filter_map(|workspace| {
                if !workspace.supported_agent {
                    return None;
                }
                let session_name = self.workspace_running_agent_session_for_status_poll(
                    workspace.path.as_path(),
                    selected_live_session,
                )?;
                Some(WorkspaceStatusTarget {
                    workspace_name: workspace.name.clone(),
                    workspace_path: workspace.path.clone(),
                    session_name,
                    supported_agent: workspace.supported_agent,
                })
            })
            .collect::<Vec<WorkspaceStatusTarget>>();
        if targets.is_empty() {
            return targets;
        }

        let Some(last_polled_at) = self.polling.last_workspace_status_poll_at else {
            return targets;
        };
        let since_last = Instant::now().saturating_duration_since(last_polled_at);
        if since_last >= Duration::from_millis(WORKSPACE_STATUS_POLL_INTERVAL_MS) {
            return targets;
        }

        self.telemetry.event_log.log(
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
            PreviewTab::Home => self.selected_task_preview_session_if_ready(),
            PreviewTab::Git => {
                let workspace = self.state.selected_workspace()?;
                let session_name = git_session_name_for_workspace(workspace);
                self.session
                    .lazygit_sessions
                    .is_ready(&session_name)
                    .then_some(session_name)
            }
            PreviewTab::Shell => self.selected_shell_preview_session_if_ready(),
            PreviewTab::Agent => self.selected_agent_preview_session_if_ready(),
        }
    }

    fn selected_live_preview_session_for_completion(&self) -> Option<String> {
        self.selected_live_preview_session_if_ready()
    }

    pub(super) fn poll_preview(&mut self) {
        self.sync_preview_stream_target();
        if !self.tmux_input.supports_background_poll() {
            self.poll_preview_sync();
            return;
        }
        if self.polling.preview_poll_in_flight {
            self.polling.preview_poll_requested = true;
            return;
        }

        let prepared_session = self.prepare_live_preview_session();
        let stream_blocking = prepared_session.as_ref().is_some_and(|target| {
            self.preview_stream_blocks_selected_poll(target.session_name.as_str())
        });
        let live_preview = prepared_session.filter(|_| !stream_blocking);
        let live_scrollback_lines = self.live_preview_scrollback_lines();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = self.status_poll_targets_for_async_preview(live_preview.as_ref());
        if !status_poll_targets.is_empty() {
            self.polling.last_workspace_status_poll_at = Some(Instant::now());
        }

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
            self.polling.preview_poll_requested = false;
            if !stream_blocking {
                self.clear_agent_activity_tracking();
                self.refresh_preview_summary();
            }
            return;
        }

        self.polling.poll_generation = self.polling.poll_generation.saturating_add(1);
        self.polling.preview_poll_in_flight = true;
        self.polling.preview_poll_requested = false;
        self.queue_cmd(self.schedule_async_preview_poll(
            self.polling.poll_generation,
            live_preview,
            live_scrollback_lines,
            cursor_session,
            status_poll_targets,
        ));
    }

    pub(super) fn poll_preview_prioritized(&mut self) {
        self.sync_preview_stream_target();
        if !self.tmux_input.supports_background_poll() || !self.polling.preview_poll_in_flight {
            self.poll_preview();
            return;
        }

        let prepared_session = self.prepare_live_preview_session();
        let stream_blocking = prepared_session.as_ref().is_some_and(|target| {
            self.preview_stream_blocks_selected_poll(target.session_name.as_str())
        });
        let live_preview = prepared_session.filter(|_| !stream_blocking).or_else(|| {
            if stream_blocking || self.preview_tab == PreviewTab::Git {
                return None;
            }
            self.selected_live_preview_session_if_ready()
                .filter(|session_name| {
                    !self.preview_stream_blocks_selected_poll(session_name.as_str())
                })
                .map(|session_name| LivePreviewTarget {
                    session_name,
                    include_escape_sequences: true,
                })
        });
        let live_scrollback_lines = self.live_preview_scrollback_lines();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = Vec::new();

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
            self.polling.preview_poll_requested = false;
            if !stream_blocking {
                self.clear_agent_activity_tracking();
                self.refresh_preview_summary();
            }
            return;
        }

        self.polling.poll_generation = self.polling.poll_generation.saturating_add(1);
        self.polling.preview_poll_requested = false;
        self.queue_cmd(self.schedule_async_preview_poll(
            self.polling.poll_generation,
            live_preview,
            live_scrollback_lines,
            cursor_session,
            status_poll_targets,
        ));
    }

    pub(super) fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
        if completion.generation < self.polling.poll_generation {
            self.telemetry.event_log.log(
                LogEvent::new("preview_poll", "stale_result_dropped")
                    .with_data("generation", Value::from(completion.generation))
                    .with_data(
                        "latest_generation",
                        Value::from(self.polling.poll_generation),
                    ),
            );
            return;
        }

        self.polling.preview_poll_in_flight = false;
        if completion.generation > self.polling.poll_generation {
            self.polling.poll_generation = completion.generation;
        }

        let mut had_live_capture = false;
        if let Some(live_capture) = completion.live_capture {
            let selected_live_session = self.selected_live_preview_session_for_completion();
            let captured_session = live_capture.session.clone();
            let capture_succeeded = live_capture.result.is_ok();
            let mismatched_missing_session = live_capture
                .result
                .as_ref()
                .err()
                .is_some_and(|error| tmux_capture_error_indicates_missing_session(error));
            if selected_live_session.as_deref() == Some(live_capture.session.as_str()) {
                had_live_capture = true;
                self.apply_live_preview_capture(
                    &live_capture.session,
                    live_capture.scrollback_lines,
                    live_capture.include_escape_sequences,
                    live_capture.capture_ms,
                    live_capture.total_ms,
                    live_capture.result,
                );
                if capture_succeeded {
                    self.mark_preview_stream_bootstrap_completed(captured_session.as_str());
                }
            } else {
                let mut event = LogEvent::new("preview_poll", "session_mismatch_dropped")
                    .with_data("captured_session", Value::from(captured_session.clone()));
                if let Some(selected_session) = selected_live_session {
                    event = event.with_data("selected_session", Value::from(selected_session));
                }
                self.telemetry.event_log.log(event);
                if mismatched_missing_session {
                    self.handle_missing_preview_session(captured_session.as_str());
                }
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

        let workspace_index_by_path = self
            .state
            .workspaces
            .iter()
            .enumerate()
            .map(|(index, workspace)| (workspace.path.clone(), index))
            .collect::<std::collections::HashMap<_, _>>();
        for status_capture in completion.workspace_status_captures {
            let Some(workspace_index) = workspace_index_by_path
                .get(status_capture.workspace_path.as_path())
                .copied()
            else {
                continue;
            };
            self.apply_workspace_status_capture_at_index(status_capture, workspace_index);
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

        if self.polling.preview_poll_requested {
            self.polling.preview_poll_requested = false;
            self.poll_preview();
        }
    }
}
