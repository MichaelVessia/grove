use super::*;

impl GroveApp {
    pub(super) fn handle_tick_msg(&mut self) -> Cmd<Msg> {
        let now = Instant::now();
        let pending_before = self.pending_input_depth();
        let oldest_pending_before_ms = self.oldest_pending_input_age_ms(now);
        let late_by_ms = self
            .next_tick_due_at
            .map(|due_at| Self::duration_millis(now.saturating_duration_since(due_at)))
            .unwrap_or(0);
        let early_by_ms = self
            .next_tick_due_at
            .map(|due_at| Self::duration_millis(due_at.saturating_duration_since(now)))
            .unwrap_or(0);
        let _ = self
            .notifications
            .tick(Duration::from_millis(TOAST_TICK_INTERVAL_MS));
        if !self.tick_is_due(now) {
            self.emit_event(
                LogEvent::new("tick", "skipped")
                    .with_data("reason", Value::from("not_due"))
                    .with_data(
                        "interval_ms",
                        Value::from(self.next_tick_interval_ms.unwrap_or(0)),
                    )
                    .with_data("late_by_ms", Value::from(late_by_ms))
                    .with_data("early_by_ms", Value::from(early_by_ms))
                    .with_data("pending_depth", Value::from(pending_before))
                    .with_data(
                        "oldest_pending_age_ms",
                        Value::from(oldest_pending_before_ms),
                    ),
            );
            return Cmd::None;
        }

        let poll_due = self
            .next_poll_due_at
            .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));
        let workspace_refresh_due = self
            .next_workspace_refresh_due_at
            .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));
        let workspace_status_poll_due = self
            .next_workspace_status_poll_due_at
            .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));
        let visual_due = self
            .next_visual_due_at
            .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));

        self.next_tick_due_at = None;
        self.next_tick_interval_ms = None;
        if visual_due {
            self.next_visual_due_at = None;
            self.advance_visual_animation();
        }
        if poll_due {
            self.next_poll_due_at = None;
            if self
                .interactive_poll_due_at
                .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at))
            {
                self.interactive_poll_due_at = None;
            }
            if self.should_prioritize_interactive_io(now) {
                self.poll_preview_prioritized();
            } else {
                self.poll_preview();
            }
        }
        if workspace_refresh_due {
            if self.should_prioritize_interactive_io(now) {
                self.next_workspace_refresh_due_at =
                    Some(now + Duration::from_millis(WORKSPACE_REFRESH_DEFER_INTERVAL_MS));
                self.log_event_with_fields(
                    "refresh_workspaces",
                    "deferred_due_to_input",
                    [
                        (
                            "pending_depth".to_string(),
                            Value::from(self.pending_input_depth()),
                        ),
                        (
                            "oldest_pending_age_ms".to_string(),
                            Value::from(self.oldest_pending_input_age_ms(now)),
                        ),
                        (
                            "defer_ms".to_string(),
                            Value::from(WORKSPACE_REFRESH_DEFER_INTERVAL_MS),
                        ),
                    ],
                );
            } else {
                self.next_workspace_refresh_due_at =
                    Some(now + Duration::from_millis(WORKSPACE_REFRESH_INTERVAL_MS));
                self.refresh_workspaces(None);
            }
        }
        if workspace_status_poll_due {
            if self.should_prioritize_interactive_io(now) {
                self.next_workspace_status_poll_due_at =
                    Some(now + Duration::from_millis(WORKSPACE_REFRESH_DEFER_INTERVAL_MS));
                self.log_event_with_fields(
                    "workspace_status",
                    "poll_deferred_due_to_input",
                    [
                        (
                            "pending_depth".to_string(),
                            Value::from(self.pending_input_depth()),
                        ),
                        (
                            "oldest_pending_age_ms".to_string(),
                            Value::from(self.oldest_pending_input_age_ms(now)),
                        ),
                        (
                            "defer_ms".to_string(),
                            Value::from(WORKSPACE_REFRESH_DEFER_INTERVAL_MS),
                        ),
                    ],
                );
            } else {
                self.next_workspace_status_poll_due_at =
                    Some(now + Duration::from_millis(WORKSPACE_STATUS_POLL_INTERVAL_MS));
                self.poll_workspace_statuses_background();
            }
        }

        let pending_after = self.pending_input_depth();
        self.emit_event(
            LogEvent::new("tick", "processed")
                .with_data("late_by_ms", Value::from(late_by_ms))
                .with_data("early_by_ms", Value::from(early_by_ms))
                .with_data("poll_due", Value::from(poll_due))
                .with_data("workspace_refresh_due", Value::from(workspace_refresh_due))
                .with_data(
                    "workspace_status_poll_due",
                    Value::from(workspace_status_poll_due),
                )
                .with_data("visual_due", Value::from(visual_due))
                .with_data("pending_before", Value::from(pending_before))
                .with_data("pending_after", Value::from(pending_after))
                .with_data(
                    "drained_count",
                    Value::from(pending_before.saturating_sub(pending_after)),
                ),
        );
        self.schedule_next_tick()
    }
}
