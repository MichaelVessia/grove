use super::*;

impl GroveApp {
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
        let status_poll_targets = workspace_status_targets_for_polling_with_live_preview(
            &self.state.workspaces,
            self.multiplexer,
            live_preview.as_ref(),
        );

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

        let had_live_capture = completion.live_capture.is_some();
        if let Some(live_capture) = completion.live_capture {
            self.apply_live_preview_capture(
                &live_capture.session,
                live_capture.include_escape_sequences,
                live_capture.capture_ms,
                live_capture.total_ms,
                live_capture.result,
            );
        } else {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
        }

        for status_capture in completion.workspace_status_captures {
            self.apply_workspace_status_capture(status_capture);
        }
        if !had_live_capture {
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
