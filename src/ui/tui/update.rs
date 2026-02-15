use super::*;

impl GroveApp {
    pub(super) fn init_model(&mut self) -> Cmd<Msg> {
        self.poll_preview();
        let next_tick_cmd = self.schedule_next_tick();
        let init_cmd = Cmd::batch(vec![next_tick_cmd, Cmd::set_mouse_capture(true)]);
        self.merge_deferred_cmds(init_cmd)
    }

    pub(super) fn update_model(&mut self, msg: Msg) -> Cmd<Msg> {
        let update_started_at = Instant::now();
        let msg_kind = Self::msg_kind(&msg);
        let before = self.capture_transition_snapshot();
        let cmd = match msg {
            Msg::Tick => {
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
                    self.event_log.log(
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
                    Cmd::None
                } else {
                    let poll_due = self
                        .next_poll_due_at
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
                        self.poll_preview();
                    }

                    let pending_after = self.pending_input_depth();
                    self.event_log.log(
                        LogEvent::new("tick", "processed")
                            .with_data("late_by_ms", Value::from(late_by_ms))
                            .with_data("early_by_ms", Value::from(early_by_ms))
                            .with_data("poll_due", Value::from(poll_due))
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
            Msg::Key(key_event) => {
                let (quit, key_cmd) = self.handle_key(key_event);
                if quit {
                    Cmd::Quit
                } else {
                    let tick_cmd = self.schedule_next_tick();
                    if matches!(key_cmd, Cmd::None) {
                        tick_cmd
                    } else {
                        Cmd::batch(vec![key_cmd, tick_cmd])
                    }
                }
            }
            Msg::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
                self.schedule_next_tick()
            }
            Msg::Paste(paste_event) => {
                let paste_cmd = self.handle_paste_event(paste_event);
                let tick_cmd = self.schedule_next_tick();
                if matches!(paste_cmd, Cmd::None) {
                    tick_cmd
                } else {
                    Cmd::batch(vec![paste_cmd, tick_cmd])
                }
            }
            Msg::Resize { width, height } => {
                self.viewport_width = width;
                self.viewport_height = height;
                let interactive_active = self.interactive.is_some();
                if let Some(state) = self.interactive.as_mut() {
                    state.update_cursor(
                        state.cursor_row,
                        state.cursor_col,
                        state.cursor_visible,
                        height,
                        width,
                    );
                }
                self.sync_interactive_session_geometry();
                if interactive_active {
                    self.poll_preview();
                }
                Cmd::None
            }
            Msg::PreviewPollCompleted(completion) => {
                self.handle_preview_poll_completed(completion);
                Cmd::None
            }
            Msg::RefreshWorkspacesCompleted(completion) => {
                self.apply_refresh_workspaces_completion(completion);
                Cmd::None
            }
            Msg::DeleteWorkspaceCompleted(completion) => {
                self.apply_delete_workspace_completion(completion);
                Cmd::None
            }
            Msg::CreateWorkspaceCompleted(completion) => {
                self.apply_create_workspace_completion(completion);
                Cmd::None
            }
            Msg::StartAgentCompleted(completion) => {
                self.apply_start_agent_completion(completion);
                Cmd::None
            }
            Msg::StopAgentCompleted(completion) => {
                self.apply_stop_agent_completion(completion);
                Cmd::None
            }
            Msg::InteractiveSendCompleted(completion) => {
                self.handle_interactive_send_completed(completion)
            }
            Msg::Noop => Cmd::None,
        };
        self.emit_transition_events(&before);
        self.event_log.log(
            LogEvent::new("update_timing", "message_handled")
                .with_data("msg_kind", Value::from(msg_kind))
                .with_data(
                    "update_ms",
                    Value::from(Self::duration_millis(
                        Instant::now().saturating_duration_since(update_started_at),
                    )),
                )
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
        self.merge_deferred_cmds(cmd)
    }
}
