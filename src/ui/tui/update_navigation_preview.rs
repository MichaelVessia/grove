use super::*;

impl GroveApp {
    pub(super) fn preview_output_dimensions(&self) -> Option<(u16, u16)> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }

        let inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if inner.is_empty() || inner.width == 0 {
            return None;
        }

        let output_height = inner.height.saturating_sub(PREVIEW_METADATA_ROWS).max(1);
        Some((inner.width, output_height))
    }

    fn ensure_lazygit_session_for_selected_workspace(&mut self) -> Option<String> {
        let workspace = self.state.selected_workspace()?;
        let session_name = git_session_name_for_workspace(workspace);

        if self.lazygit_ready_sessions.contains(&session_name) {
            return Some(session_name);
        }
        if self.lazygit_failed_sessions.contains(&session_name) {
            return None;
        }
        if self.lazygit_launch_in_flight.contains(&session_name) {
            return None;
        }

        let capture_cols = self
            .preview_output_dimensions()
            .map_or(self.viewport_width.saturating_sub(4), |(width, _)| width)
            .max(80);
        let capture_rows = self.viewport_height.saturating_sub(4).max(1);
        let launch_request = shell_launch_request_for_workspace(
            workspace,
            session_name.clone(),
            LAZYGIT_COMMAND.to_string(),
            Some(capture_cols),
            Some(capture_rows),
        );
        let async_launch = self.tmux_input.supports_background_launch();
        self.event_log.log(
            LogEvent::new("lazygit_launch", "started")
                .with_data("session", Value::from(session_name.clone()))
                .with_data("multiplexer", Value::from(self.multiplexer.label()))
                .with_data("async", Value::from(async_launch))
                .with_data("capture_cols", Value::from(capture_cols))
                .with_data("capture_rows", Value::from(capture_rows)),
        );

        if async_launch {
            self.lazygit_launch_in_flight.insert(session_name.clone());
            let multiplexer = self.multiplexer;
            let completion_session = session_name.clone();
            self.queue_cmd(Cmd::task(move || {
                let started_at = Instant::now();
                let (_, result) = execute_shell_launch_request_for_mode(
                    &launch_request,
                    multiplexer,
                    CommandExecutionMode::Process,
                );
                let duration_ms =
                    GroveApp::duration_millis(Instant::now().saturating_duration_since(started_at));
                Msg::LazygitLaunchCompleted(LazygitLaunchCompletion {
                    session_name: completion_session,
                    duration_ms,
                    result,
                })
            }));
            return None;
        }

        let launch_started_at = Instant::now();
        let (_, launch_result) = execute_shell_launch_request_for_mode(
            &launch_request,
            self.multiplexer,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        let duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(launch_started_at));
        let mut completion_event = LogEvent::new("lazygit_launch", "completed")
            .with_data("session", Value::from(session_name.clone()))
            .with_data("multiplexer", Value::from(self.multiplexer.label()))
            .with_data("async", Value::from(false))
            .with_data("duration_ms", Value::from(duration_ms))
            .with_data("ok", Value::from(launch_result.is_ok()));

        if let Err(error) = launch_result {
            completion_event = completion_event.with_data("error", Value::from(error.clone()));
            self.event_log.log(completion_event);
            self.last_tmux_error = Some(error);
            self.show_toast("lazygit launch failed", true);
            self.lazygit_ready_sessions.remove(&session_name);
            self.lazygit_failed_sessions.insert(session_name);
            return None;
        }

        self.event_log.log(completion_event);
        self.lazygit_failed_sessions.remove(&session_name);
        self.lazygit_ready_sessions.insert(session_name.clone());
        Some(session_name)
    }

    pub(super) fn prepare_live_preview_session(&mut self) -> Option<LivePreviewTarget> {
        if self.preview_tab == PreviewTab::Git {
            return self
                .ensure_lazygit_session_for_selected_workspace()
                .map(|session_name| LivePreviewTarget {
                    session_name,
                    include_escape_sequences: true,
                });
        }
        live_preview_capture_target_for_tab(
            self.state.selected_workspace(),
            false,
            &self.lazygit_ready_sessions,
        )
    }

    pub(super) fn handle_lazygit_launch_completed(&mut self, completion: LazygitLaunchCompletion) {
        let LazygitLaunchCompletion {
            session_name,
            duration_ms,
            result,
        } = completion;
        self.lazygit_launch_in_flight.remove(&session_name);

        let mut completion_event = LogEvent::new("lazygit_launch", "completed")
            .with_data("session", Value::from(session_name.clone()))
            .with_data("multiplexer", Value::from(self.multiplexer.label()))
            .with_data("async", Value::from(true))
            .with_data("duration_ms", Value::from(duration_ms))
            .with_data("ok", Value::from(result.is_ok()));

        match result {
            Ok(()) => {
                self.last_tmux_error = None;
                self.lazygit_failed_sessions.remove(&session_name);
                self.lazygit_ready_sessions.insert(session_name.clone());
                self.event_log.log(completion_event);

                let selected_session_matches =
                    self.state.selected_workspace().is_some_and(|workspace| {
                        git_session_name_for_workspace(workspace) == session_name
                    });
                if selected_session_matches && self.preview_tab == PreviewTab::Git {
                    self.poll_preview();
                }
            }
            Err(error) => {
                completion_event = completion_event.with_data("error", Value::from(error.clone()));
                self.event_log.log(completion_event);
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.lazygit_ready_sessions.remove(&session_name);
                self.lazygit_failed_sessions.insert(session_name);
                self.show_toast("lazygit launch failed", true);
            }
        }
    }

    pub(super) fn interactive_target_session(&self) -> Option<String> {
        self.interactive
            .as_ref()
            .map(|state| state.target_session.clone())
    }
}
