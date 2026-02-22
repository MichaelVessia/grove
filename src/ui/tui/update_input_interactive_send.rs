use super::*;

impl GroveApp {
    fn queue_interactive_send(&mut self, send: QueuedInteractiveSend) -> Cmd<Msg> {
        self.pending_interactive_sends.push_back(send);
        self.dispatch_next_interactive_send()
    }

    fn dispatch_next_interactive_send(&mut self) -> Cmd<Msg> {
        if self.interactive_send_in_flight {
            return Cmd::None;
        }
        let Some(send) = self.pending_interactive_sends.pop_front() else {
            return Cmd::None;
        };
        self.interactive_send_in_flight = true;
        let command = send.command.clone();
        let daemon_socket_path = self.interactive_daemon_socket_path();
        Cmd::task(move || {
            let started_at = Instant::now();
            let execution = if let Some(socket_path) = daemon_socket_path {
                let payload = DaemonSessionSendKeysPayload {
                    command: command.clone(),
                    fire_and_forget: false,
                };
                match session_send_keys_via_socket(&socket_path, payload) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(daemon_error)) => Err(std::io::Error::other(daemon_error.message)),
                    Err(io_error) => Err(io_error),
                }
            } else {
                CommandTmuxInput::execute_command(&command)
            };
            let completed_at = Instant::now();
            let tmux_send_ms = u64::try_from(
                completed_at
                    .saturating_duration_since(started_at)
                    .as_millis(),
            )
            .unwrap_or(u64::MAX);
            Msg::InteractiveSendCompleted(InteractiveSendCompletion {
                send,
                tmux_send_ms,
                error: execution.err().map(|error| error.to_string()),
            })
        })
    }

    pub(super) fn handle_interactive_send_completed(
        &mut self,
        completion: InteractiveSendCompletion,
    ) -> Cmd<Msg> {
        let InteractiveSendCompletion {
            send:
                QueuedInteractiveSend {
                    command,
                    target_session,
                    action_kind,
                    trace_context,
                    literal_chars,
                    ..
                },
            tmux_send_ms,
            error,
        } = completion;
        self.interactive_send_in_flight = false;
        if let Some(error) = error {
            self.last_tmux_error = Some(error.clone());
            self.log_tmux_error(error.clone());
            if let Some(trace_context) = trace_context {
                self.log_input_event_with_fields(
                    "interactive_forward_failed",
                    trace_context.seq,
                    vec![
                        ("session".to_string(), Value::from(target_session)),
                        ("action".to_string(), Value::from(action_kind)),
                        ("command".to_string(), Value::from(command)),
                        ("error".to_string(), Value::from(error)),
                    ],
                );
            }
            return self.dispatch_next_interactive_send();
        }

        self.last_tmux_error = None;
        if let Some(trace_context) = trace_context {
            let forwarded_at = Instant::now();
            self.track_pending_interactive_input(trace_context, &target_session, forwarded_at);
            let mut fields = vec![
                ("session".to_string(), Value::from(target_session)),
                ("action".to_string(), Value::from(action_kind)),
                ("command".to_string(), Value::from(command)),
                ("tmux_send_ms".to_string(), Value::from(tmux_send_ms)),
                (
                    "queue_depth".to_string(),
                    Value::from(usize_to_u64(self.pending_interactive_inputs.len())),
                ),
            ];
            if let Some(literal_chars) = literal_chars {
                fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
            }
            self.log_input_event_with_fields("interactive_forwarded", trace_context.seq, fields);
        }
        self.dispatch_next_interactive_send()
    }

    pub(super) fn send_interactive_action(
        &mut self,
        action: &InteractiveAction,
        target_session: &str,
        trace_context: Option<InputTraceContext>,
    ) -> Cmd<Msg> {
        let Some(command) = multiplexer_send_input_command(target_session, action) else {
            if let Some(trace_context) = trace_context {
                self.log_input_event_with_fields(
                    "interactive_action_unmapped",
                    trace_context.seq,
                    vec![
                        (
                            "action".to_string(),
                            Value::from(Self::interactive_action_kind(action)),
                        ),
                        (
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        ),
                    ],
                );
            }
            return Cmd::None;
        };

        let literal_chars = if let InteractiveAction::SendLiteral(text) = action {
            Some(usize_to_u64(text.chars().count()))
        } else {
            None
        };

        if let Some(state) = self.interactive.as_mut()
            && state.daemon_stream.is_some()
        {
            let payload = DaemonSessionSendKeysPayload {
                command: command.clone(),
                fire_and_forget: true,
            };
            match state.pipeline_send_keys(payload) {
                Ok(()) => {
                    self.last_tmux_error = None;
                    if let Some(trace_context) = trace_context {
                        let forwarded_at = Instant::now();
                        self.track_pending_interactive_input(
                            trace_context,
                            target_session,
                            forwarded_at,
                        );
                        let mut fields = vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            (
                                "action".to_string(),
                                Value::from(Self::interactive_action_kind(action)),
                            ),
                            ("command".to_string(), Value::from(command.clone())),
                            ("pipeline".to_string(), Value::from(true)),
                            (
                                "queue_depth".to_string(),
                                Value::from(usize_to_u64(self.pending_interactive_inputs.len())),
                            ),
                        ];
                        if let Some(literal_chars) = literal_chars {
                            fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
                        }
                        self.log_input_event_with_fields(
                            "interactive_forwarded",
                            trace_context.seq,
                            fields,
                        );
                    }
                    return Cmd::None;
                }
                Err(error) => {
                    self.log_event_with_fields(
                        "input",
                        "pipeline_send_fallback",
                        [
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("command".to_string(), Value::from(command.clone())),
                            ("error".to_string(), Value::from(error.to_string())),
                        ],
                    );
                    if let Some(state) = self.interactive.as_mut() {
                        state.close_daemon_stream();
                    }
                }
            }
        }

        if self.tmux_input.supports_background_send() {
            return self.queue_interactive_send(QueuedInteractiveSend {
                command,
                target_session: target_session.to_string(),
                action_kind: Self::interactive_action_kind(action).to_string(),
                trace_context,
                literal_chars,
            });
        }

        let send_started_at = Instant::now();
        let sync_result = if let Some(socket_path) = &self.interactive_daemon_socket_path() {
            let payload = DaemonSessionSendKeysPayload {
                command: command.clone(),
                fire_and_forget: false,
            };
            match session_send_keys_via_socket(socket_path, payload) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(daemon_error)) => Err(std::io::Error::other(daemon_error.message)),
                Err(io_error) => Err(io_error),
            }
        } else {
            self.execute_tmux_command(&command)
        };
        match sync_result {
            Ok(()) => {
                self.last_tmux_error = None;
                if let Some(trace_context) = trace_context {
                    let forwarded_at = Instant::now();
                    let send_duration_ms = Self::duration_millis(
                        forwarded_at.saturating_duration_since(send_started_at),
                    );
                    self.track_pending_interactive_input(
                        trace_context,
                        target_session,
                        forwarded_at,
                    );

                    let mut fields = vec![
                        (
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        ),
                        (
                            "action".to_string(),
                            Value::from(Self::interactive_action_kind(action)),
                        ),
                        ("command".to_string(), Value::from(command.clone())),
                        ("tmux_send_ms".to_string(), Value::from(send_duration_ms)),
                        (
                            "queue_depth".to_string(),
                            Value::from(usize_to_u64(self.pending_interactive_inputs.len())),
                        ),
                    ];
                    if let Some(literal_chars) = literal_chars {
                        fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
                    }
                    self.log_input_event_with_fields(
                        "interactive_forwarded",
                        trace_context.seq,
                        fields,
                    );
                }
            }
            Err(error) => {
                let message = error.to_string();
                self.last_tmux_error = Some(message.clone());
                self.log_tmux_error(message);
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "interactive_forward_failed",
                        trace_context.seq,
                        vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            (
                                "action".to_string(),
                                Value::from(Self::interactive_action_kind(action)),
                            ),
                            ("command".to_string(), Value::from(command.clone())),
                            ("error".to_string(), Value::from(error.to_string())),
                        ],
                    );
                }
            }
        }
        Cmd::None
    }
}
