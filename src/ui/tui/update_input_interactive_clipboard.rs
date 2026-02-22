use super::*;

impl GroveApp {
    pub(super) fn copy_interactive_capture(&mut self) {
        self.copy_interactive_selection_or_visible();
    }

    fn read_clipboard_or_cached_text(&mut self) -> Result<String, String> {
        let clipboard_text = self.clipboard.read_text();
        if let Ok(text) = clipboard_text
            && !text.is_empty()
        {
            return Ok(text);
        }

        if let Some(text) = self.copied_text.clone()
            && !text.is_empty()
        {
            return Ok(text);
        }

        Err("clipboard empty".to_string())
    }

    pub(super) fn paste_clipboard_text(
        &mut self,
        target_session: &str,
        bracketed_paste: bool,
        trace_context: Option<InputTraceContext>,
    ) -> Cmd<Msg> {
        let text = match self.read_clipboard_or_cached_text() {
            Ok(text) => text,
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "paste_clipboard_missing",
                        trace_context.seq,
                        vec![(
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        )],
                    );
                }
                return Cmd::None;
            }
        };

        if bracketed_paste {
            let payload = format!("\u{1b}[200~{text}\u{1b}[201~");
            return self.send_interactive_action(
                &InteractiveAction::SendLiteral(payload),
                target_session,
                trace_context,
            );
        }

        let paste_started_at = Instant::now();
        let (path_kind, paste_result) =
            if let Some(socket_path) = &self.interactive_daemon_socket_path() {
                let payload = DaemonSessionPasteBufferPayload {
                    session_name: target_session.to_string(),
                    text: text.clone(),
                };
                (
                    "daemon",
                    match session_paste_buffer_via_socket(socket_path, payload) {
                        Ok(Ok(())) => Ok(()),
                        Ok(Err(daemon_error)) => Err(std::io::Error::other(daemon_error.message)),
                        Err(io_error) => Err(io_error),
                    },
                )
            } else {
                ("tmux", self.tmux_input.paste_buffer(target_session, &text))
            };
        let paste_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(paste_started_at));
        match paste_result {
            Ok(()) => {
                self.last_tmux_error = None;
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "interactive_paste_buffer_completed",
                        trace_context.seq,
                        vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("path".to_string(), Value::from(path_kind)),
                            (
                                "chars".to_string(),
                                Value::from(usize_to_u64(text.chars().count())),
                            ),
                            ("text".to_string(), Value::from(text.clone())),
                            ("paste_ms".to_string(), Value::from(paste_ms)),
                        ],
                    );
                } else {
                    self.log_event_with_fields(
                        "input",
                        "interactive_paste_buffer_completed",
                        [
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("path".to_string(), Value::from(path_kind)),
                            (
                                "chars".to_string(),
                                Value::from(usize_to_u64(text.chars().count())),
                            ),
                            ("text".to_string(), Value::from(text.clone())),
                            ("paste_ms".to_string(), Value::from(paste_ms)),
                        ],
                    );
                }
            }
            Err(error) => {
                let message = error.to_string();
                self.last_tmux_error = Some(message.clone());
                self.log_tmux_error(message.clone());
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "interactive_paste_buffer_failed",
                        trace_context.seq,
                        vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("path".to_string(), Value::from(path_kind)),
                            (
                                "chars".to_string(),
                                Value::from(usize_to_u64(text.chars().count())),
                            ),
                            ("text".to_string(), Value::from(text.clone())),
                            ("paste_ms".to_string(), Value::from(paste_ms)),
                            ("error".to_string(), Value::from(message)),
                        ],
                    );
                } else {
                    self.log_event_with_fields(
                        "input",
                        "interactive_paste_buffer_failed",
                        [
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("path".to_string(), Value::from(path_kind)),
                            (
                                "chars".to_string(),
                                Value::from(usize_to_u64(text.chars().count())),
                            ),
                            ("text".to_string(), Value::from(text.clone())),
                            ("paste_ms".to_string(), Value::from(paste_ms)),
                            ("error".to_string(), Value::from(message)),
                        ],
                    );
                }
            }
        }

        Cmd::None
    }
}
