use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransitionSnapshot {
    selected_index: usize,
    selected_workspace: Option<String>,
    focus: PaneFocus,
    mode: UiMode,
    interactive_session: Option<String>,
}

impl GroveApp {
    pub(super) fn mode_label(&self) -> &'static str {
        if self.interactive.is_some() {
            return "Interactive";
        }

        match self.state.mode {
            UiMode::List => "List",
            UiMode::Preview => "Preview",
        }
    }

    pub(super) fn focus_label(&self) -> &'static str {
        match self.state.focus {
            PaneFocus::WorkspaceList => "WorkspaceList",
            PaneFocus::Preview => "Preview",
        }
    }

    pub(super) fn focus_name(focus: PaneFocus) -> &'static str {
        match focus {
            PaneFocus::WorkspaceList => "workspace_list",
            PaneFocus::Preview => "preview",
        }
    }

    pub(super) fn mode_name(mode: UiMode) -> &'static str {
        match mode {
            UiMode::List => "list",
            UiMode::Preview => "preview",
        }
    }

    pub(super) fn hit_region_name(region: HitRegion) -> &'static str {
        match region {
            HitRegion::WorkspaceList => "workspace_list",
            HitRegion::Preview => "preview",
            HitRegion::Divider => "divider",
            HitRegion::StatusLine => "status_line",
            HitRegion::Header => "header",
            HitRegion::Outside => "outside",
        }
    }

    pub(super) fn duration_millis(duration: Duration) -> u64 {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    }

    pub(super) fn msg_kind(msg: &Msg) -> &'static str {
        match msg {
            Msg::Tick => "tick",
            Msg::Key(_) => "key",
            Msg::Mouse(_) => "mouse",
            Msg::Paste(_) => "paste",
            Msg::Resize { .. } => "resize",
            Msg::PreviewPollCompleted(_) => "preview_poll_completed",
            Msg::RefreshWorkspacesCompleted(_) => "refresh_workspaces_completed",
            Msg::DeleteWorkspaceCompleted(_) => "delete_workspace_completed",
            Msg::CreateWorkspaceCompleted(_) => "create_workspace_completed",
            Msg::StartAgentCompleted(_) => "start_agent_completed",
            Msg::StopAgentCompleted(_) => "stop_agent_completed",
            Msg::InteractiveSendCompleted(_) => "interactive_send_completed",
            Msg::Noop => "noop",
        }
    }

    pub(super) fn capture_transition_snapshot(&self) -> TransitionSnapshot {
        TransitionSnapshot {
            selected_index: self.state.selected_index,
            selected_workspace: self.selected_workspace_name(),
            focus: self.state.focus,
            mode: self.state.mode,
            interactive_session: self.interactive_target_session(),
        }
    }

    pub(super) fn emit_transition_events(&mut self, before: &TransitionSnapshot) {
        let after = self.capture_transition_snapshot();
        if after.selected_index != before.selected_index {
            let selection_index = u64::try_from(after.selected_index).unwrap_or(u64::MAX);
            let workspace_value = after
                .selected_workspace
                .clone()
                .map(Value::from)
                .unwrap_or(Value::Null);
            self.event_log.log(
                LogEvent::new("state_change", "selection_changed")
                    .with_data("index", Value::from(selection_index))
                    .with_data("workspace", workspace_value),
            );
        }
        if after.focus != before.focus {
            self.event_log.log(
                LogEvent::new("state_change", "focus_changed")
                    .with_data("focus", Value::from(Self::focus_name(after.focus))),
            );
        }
        if after.mode != before.mode {
            self.event_log.log(
                LogEvent::new("mode_change", "mode_changed")
                    .with_data("mode", Value::from(Self::mode_name(after.mode))),
            );
        }
        match (&before.interactive_session, &after.interactive_session) {
            (None, Some(session)) => {
                self.event_log.log(
                    LogEvent::new("mode_change", "interactive_entered")
                        .with_data("session", Value::from(session.clone())),
                );
            }
            (Some(session), None) => {
                self.event_log.log(
                    LogEvent::new("mode_change", "interactive_exited")
                        .with_data("session", Value::from(session.clone())),
                );
                self.interactive_poll_due_at = None;
                self.pending_resize_verification = None;
                let pending_before = self.pending_interactive_inputs.len();
                self.clear_pending_inputs_for_session(session);
                let pending_after = self.pending_interactive_inputs.len();
                self.clear_pending_sends_for_session(session);
                if pending_before != pending_after {
                    self.event_log.log(
                        LogEvent::new("input", "pending_inputs_cleared")
                            .with_data("session", Value::from(session.clone()))
                            .with_data(
                                "cleared",
                                Value::from(
                                    u64::try_from(pending_before.saturating_sub(pending_after))
                                        .unwrap_or(u64::MAX),
                                ),
                            ),
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn log_dialog_event_with_fields(
        &self,
        kind: &str,
        action: &str,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) {
        let event = LogEvent::new("dialog", action)
            .with_data("kind", Value::from(kind.to_string()))
            .with_data_fields(fields);
        self.event_log.log(event);
    }

    pub(super) fn log_dialog_event(&self, kind: &str, action: &str) {
        self.log_dialog_event_with_fields(kind, action, std::iter::empty());
    }

    pub(super) fn log_tmux_error(&self, message: String) {
        self.event_log
            .log(LogEvent::new("error", "tmux_error").with_data("message", Value::from(message)));
    }

    pub(super) fn execute_tmux_command(&mut self, command: &[String]) -> std::io::Result<()> {
        let started_at = Instant::now();
        self.event_log.log(
            LogEvent::new("tmux_cmd", "execute")
                .with_data("command", Value::from(command.join(" "))),
        );
        let result = execute_command_with(command, |command| self.tmux_input.execute(command));
        let duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(started_at));
        let mut completion_event = LogEvent::new("tmux_cmd", "completed")
            .with_data("command", Value::from(command.join(" ")))
            .with_data("duration_ms", Value::from(duration_ms))
            .with_data("ok", Value::from(result.is_ok()));
        if let Err(error) = &result {
            completion_event = completion_event.with_data("error", Value::from(error.to_string()));
            self.log_tmux_error(error.to_string());
        }
        self.event_log.log(completion_event);
        result
    }

    pub(super) fn show_toast(&mut self, text: impl Into<String>, is_error: bool) {
        let message = text.into();
        self.event_log.log(
            LogEvent::new("toast", "toast_shown")
                .with_data("text", Value::from(message.clone()))
                .with_data("is_error", Value::from(is_error)),
        );

        let toast = if is_error {
            Toast::new(message)
                .title("Error")
                .icon(ToastIcon::Error)
                .style_variant(ToastStyle::Error)
                .duration(Duration::from_secs(3))
        } else {
            Toast::new(message)
                .icon(ToastIcon::Success)
                .style_variant(ToastStyle::Success)
                .duration(Duration::from_secs(3))
        };
        let priority = if is_error {
            NotificationPriority::High
        } else {
            NotificationPriority::Normal
        };
        let _ = self.notifications.push(toast, priority);
        let _ = self.notifications.tick(Duration::ZERO);
    }

    pub(super) fn log_input_event_with_fields(
        &self,
        kind: &str,
        seq: u64,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) {
        self.event_log.log(
            LogEvent::new("input", kind)
                .with_data("seq", Value::from(seq))
                .with_data_fields(fields),
        );
    }

    pub(super) fn interactive_action_kind(action: &InteractiveAction) -> &'static str {
        match action {
            InteractiveAction::SendNamed(_) => "send_named",
            InteractiveAction::SendLiteral(_) => "send_literal",
            InteractiveAction::ExitInteractive => "exit_interactive",
            InteractiveAction::CopySelection => "copy_selection",
            InteractiveAction::PasteClipboard => "paste_clipboard",
            InteractiveAction::Noop => "noop",
        }
    }

    pub(super) fn interactive_key_kind(key: &InteractiveKey) -> &'static str {
        match key {
            InteractiveKey::Enter => "enter",
            InteractiveKey::Tab => "tab",
            InteractiveKey::Backspace => "backspace",
            InteractiveKey::Delete => "delete",
            InteractiveKey::Up => "up",
            InteractiveKey::Down => "down",
            InteractiveKey::Left => "left",
            InteractiveKey::Right => "right",
            InteractiveKey::Home => "home",
            InteractiveKey::End => "end",
            InteractiveKey::PageUp => "page_up",
            InteractiveKey::PageDown => "page_down",
            InteractiveKey::Escape => "escape",
            InteractiveKey::CtrlBackslash => "ctrl_backslash",
            InteractiveKey::Ctrl(_) => "ctrl",
            InteractiveKey::Function(_) => "function",
            InteractiveKey::Char(_) => "char",
            InteractiveKey::AltC => "alt_c",
            InteractiveKey::AltV => "alt_v",
        }
    }

    pub(super) fn track_pending_interactive_input(
        &mut self,
        trace_context: InputTraceContext,
        target_session: &str,
        forwarded_at: Instant,
    ) {
        self.pending_interactive_inputs
            .push_back(PendingInteractiveInput {
                seq: trace_context.seq,
                session: target_session.to_string(),
                received_at: trace_context.received_at,
                forwarded_at,
            });

        if self.pending_interactive_inputs.len() <= MAX_PENDING_INPUT_TRACES {
            return;
        }

        if let Some(dropped) = self.pending_interactive_inputs.pop_front() {
            self.log_input_event_with_fields(
                "pending_input_dropped",
                dropped.seq,
                vec![
                    ("session".to_string(), Value::from(dropped.session)),
                    (
                        "queue_depth".to_string(),
                        Value::from(
                            u64::try_from(self.pending_interactive_inputs.len())
                                .unwrap_or(u64::MAX),
                        ),
                    ),
                ],
            );
        }
    }

    fn clear_pending_inputs_for_session(&mut self, target_session: &str) {
        self.pending_interactive_inputs
            .retain(|input| input.session != target_session);
    }

    fn clear_pending_sends_for_session(&mut self, target_session: &str) {
        self.pending_interactive_sends
            .retain(|send| send.target_session != target_session);
    }

    pub(super) fn drain_pending_inputs_for_session(
        &mut self,
        target_session: &str,
    ) -> Vec<PendingInteractiveInput> {
        let mut retained = VecDeque::new();
        let mut drained = Vec::new();

        while let Some(input) = self.pending_interactive_inputs.pop_front() {
            if input.session == target_session {
                drained.push(input);
            } else {
                retained.push_back(input);
            }
        }

        self.pending_interactive_inputs = retained;
        drained
    }

    pub(super) fn pending_input_depth(&self) -> u64 {
        u64::try_from(self.pending_interactive_inputs.len()).unwrap_or(u64::MAX)
    }

    pub(super) fn oldest_pending_input_age_ms(&self, now: Instant) -> u64 {
        self.pending_interactive_inputs
            .front()
            .map(|trace| Self::duration_millis(now.saturating_duration_since(trace.received_at)))
            .unwrap_or(0)
    }

    pub(super) fn schedule_interactive_debounced_poll(&mut self, now: Instant) {
        if self.interactive.is_none() {
            return;
        }

        self.interactive_poll_due_at =
            Some(now + Duration::from_millis(INTERACTIVE_KEYSTROKE_DEBOUNCE_MS));
        let next_generation = self.poll_generation.saturating_add(1);
        self.event_log.log(
            LogEvent::new("tick", "interactive_debounce_scheduled")
                .with_data("generation", Value::from(next_generation))
                .with_data("due_in_ms", Value::from(INTERACTIVE_KEYSTROKE_DEBOUNCE_MS))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
    }

    fn frame_lines_hash(lines: &[String]) -> u64 {
        let mut hasher = DefaultHasher::new();
        lines.hash(&mut hasher);
        hasher.finish()
    }

    fn frame_buffer_lines(frame: &mut Frame) -> Vec<String> {
        let height = frame.buffer.height();
        let mut lines = Vec::with_capacity(usize::from(height));
        for y in 0..height {
            let mut row = String::with_capacity(usize::from(frame.buffer.width()));
            for x in 0..frame.buffer.width() {
                let Some(cell) = frame.buffer.get(x, y).copied() else {
                    continue;
                };
                if cell.is_continuation() {
                    continue;
                }
                if let Some(value) = cell.content.as_char() {
                    row.push(value);
                    continue;
                }
                if let Some(grapheme_id) = cell.content.grapheme_id()
                    && let Some(grapheme) = frame.pool.get(grapheme_id)
                {
                    row.push_str(grapheme);
                    continue;
                }
                row.push(' ');
            }
            lines.push(row.trim_end_matches(' ').to_string());
        }

        lines
    }

    pub(super) fn log_frame_render(&self, frame: &mut Frame) {
        let Some(app_start_ts) = self.debug_record_start_ts else {
            return;
        };

        let lines = Self::frame_buffer_lines(frame);
        let frame_hash = Self::frame_lines_hash(&lines);
        let non_empty_line_count = lines.iter().filter(|line| !line.is_empty()).count();
        let mut seq = self.frame_render_seq.borrow_mut();
        *seq = seq.saturating_add(1);
        let seq_value = *seq;

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default();
        let interactive_session = self
            .interactive
            .as_ref()
            .map(|state| state.target_session.clone())
            .unwrap_or_default();
        let pending_input_depth = self.pending_input_depth();
        let oldest_pending_input_seq = self
            .pending_interactive_inputs
            .front()
            .map(|trace| trace.seq)
            .unwrap_or(0);
        let oldest_pending_input_age_ms = self.oldest_pending_input_age_ms(Instant::now());

        let mut frame_event = LogEvent::new("frame", "rendered")
            .with_data("seq", Value::from(seq_value))
            .with_data("app_start_ts", Value::from(app_start_ts))
            .with_data("width", Value::from(frame.buffer.width()))
            .with_data("height", Value::from(frame.buffer.height()))
            .with_data(
                "line_count",
                Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
            )
            .with_data(
                "non_empty_line_count",
                Value::from(u64::try_from(non_empty_line_count).unwrap_or(u64::MAX)),
            )
            .with_data("frame_hash", Value::from(frame_hash))
            .with_data("degradation", Value::from(frame.degradation.as_str()))
            .with_data("mode", Value::from(self.mode_label()))
            .with_data("focus", Value::from(self.focus_label()))
            .with_data("selected_workspace", Value::from(selected_workspace))
            .with_data("interactive_session", Value::from(interactive_session))
            .with_data("sidebar_width_pct", Value::from(self.sidebar_width_pct))
            .with_data(
                "preview_offset",
                Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
            )
            .with_data("preview_auto_scroll", Value::from(self.preview.auto_scroll))
            .with_data("output_changing", Value::from(self.output_changing))
            .with_data("pending_input_depth", Value::from(pending_input_depth))
            .with_data(
                "oldest_pending_input_seq",
                Value::from(oldest_pending_input_seq),
            )
            .with_data(
                "oldest_pending_input_age_ms",
                Value::from(oldest_pending_input_age_ms),
            )
            .with_data("frame_cursor_visible", Value::from(frame.cursor_visible))
            .with_data(
                "frame_cursor_has_position",
                Value::from(frame.cursor_position.is_some()),
            );
        if let Some((cursor_col, cursor_row)) = frame.cursor_position {
            frame_event = frame_event
                .with_data("frame_cursor_col", Value::from(cursor_col))
                .with_data("frame_cursor_row", Value::from(cursor_row));
        }
        if let Some(interactive) = self.interactive.as_ref() {
            frame_event = frame_event
                .with_data(
                    "interactive_cursor_visible",
                    Value::from(interactive.cursor_visible),
                )
                .with_data(
                    "interactive_cursor_row",
                    Value::from(interactive.cursor_row),
                )
                .with_data(
                    "interactive_cursor_col",
                    Value::from(interactive.cursor_col),
                )
                .with_data(
                    "interactive_pane_width",
                    Value::from(interactive.pane_width),
                )
                .with_data(
                    "interactive_pane_height",
                    Value::from(interactive.pane_height),
                );

            let layout = Self::view_layout_for_size(
                frame.buffer.width(),
                frame.buffer.height(),
                self.sidebar_width_pct,
            );
            let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
            let preview_height = usize::from(
                preview_inner
                    .height
                    .saturating_sub(PREVIEW_METADATA_ROWS)
                    .max(1),
            );
            let cursor_target = self.interactive_cursor_target(preview_height);
            frame_event = frame_event.with_data(
                "interactive_cursor_in_viewport",
                Value::from(cursor_target.is_some()),
            );
            if let Some((visible_index, target_col, target_visible)) = cursor_target {
                frame_event = frame_event
                    .with_data(
                        "interactive_cursor_visible_index",
                        Value::from(u64::try_from(visible_index).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "interactive_cursor_target_col",
                        Value::from(u64::try_from(target_col).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "interactive_cursor_target_visible",
                        Value::from(target_visible),
                    );
            }
        }
        frame_event = frame_event.with_data(
            "frame_lines",
            Value::Array(lines.into_iter().map(Value::from).collect()),
        );
        self.event_log.log(frame_event);
    }
}
