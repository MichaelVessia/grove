use super::*;

impl GroveApp {
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
            InteractiveKey::ModifiedEnter { .. } => "modified_enter",
            InteractiveKey::Tab => "tab",
            InteractiveKey::BackTab => "back_tab",
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
                        Value::from(usize_to_u64(self.pending_interactive_inputs.len())),
                    ),
                ],
            );
        }
    }

    pub(super) fn clear_pending_inputs_for_session(&mut self, target_session: &str) {
        self.pending_interactive_inputs
            .retain(|input| input.session != target_session);
    }

    pub(super) fn clear_pending_sends_for_session(&mut self, target_session: &str) {
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
        usize_to_u64(self.pending_interactive_inputs.len())
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
}
