use super::*;

impl GroveApp {
    pub(super) fn selected_workspace_name(&self) -> Option<String> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
    }

    pub(super) fn selected_workspace_path(&self) -> Option<PathBuf> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.path.clone())
    }

    pub(super) fn queue_cmd(&mut self, cmd: Cmd<Msg>) {
        if matches!(cmd, Cmd::None) {
            return;
        }

        self.deferred_cmds.push(cmd);
    }

    fn merge_deferred_cmds(&mut self, cmd: Cmd<Msg>) -> Cmd<Msg> {
        let deferred_cmds = std::mem::take(&mut self.deferred_cmds);
        if deferred_cmds.is_empty() {
            return cmd;
        }

        if matches!(cmd, Cmd::Quit) {
            return Cmd::Quit;
        }

        if matches!(cmd, Cmd::None) {
            return Cmd::batch(deferred_cmds);
        }

        let mut merged = Vec::with_capacity(deferred_cmds.len().saturating_add(1));
        merged.push(cmd);
        merged.extend(deferred_cmds);
        Cmd::batch(merged)
    }

    fn next_input_seq(&mut self) -> u64 {
        let seq = self.input_seq_counter;
        self.input_seq_counter = self.input_seq_counter.saturating_add(1);
        seq
    }

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
            Msg::LazygitLaunchCompleted(completion) => {
                self.handle_lazygit_launch_completed(completion);
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
            Msg::MergeWorkspaceCompleted(completion) => {
                self.apply_merge_workspace_completion(completion);
                Cmd::None
            }
            Msg::UpdateWorkspaceFromBaseCompleted(completion) => {
                self.apply_update_from_base_completion(completion);
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

    fn apply_workspace_status_capture(&mut self, capture: WorkspaceStatusCapture) {
        let supported_agent = capture.supported_agent;
        let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == capture.workspace_path)
        else {
            return;
        };

        match capture.result {
            Ok(output) => {
                self.capture_changed_cleaned_for_workspace(&capture.workspace_path, &output);
                let workspace_path = self.state.workspaces[workspace_index].path.clone();
                let workspace_agent = self.state.workspaces[workspace_index].agent;
                let workspace_is_main = self.state.workspaces[workspace_index].is_main;
                let workspace = &mut self.state.workspaces[workspace_index];
                workspace.status = detect_status_with_session_override(
                    output.as_str(),
                    SessionActivity::Active,
                    workspace_is_main,
                    true,
                    supported_agent,
                    workspace_agent,
                    &workspace_path,
                );
                workspace.is_orphaned = false;
            }
            Err(error) => {
                if tmux_capture_error_indicates_missing_session(&error) {
                    let workspace = &mut self.state.workspaces[workspace_index];
                    let previously_had_live_session = workspace.status.has_session();
                    workspace.status = if workspace.is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    workspace.is_orphaned = if workspace.is_main {
                        false
                    } else {
                        previously_had_live_session || workspace.is_orphaned
                    };
                    self.clear_status_tracking_for_workspace_path(&capture.workspace_path);
                }
            }
        }
    }

    fn poll_interactive_cursor_sync(&mut self, target_session: &str) {
        let started_at = Instant::now();
        let result = self
            .tmux_input
            .capture_cursor_metadata(target_session)
            .map_err(|error| error.to_string());
        let capture_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(started_at));
        self.apply_cursor_capture_result(CursorCapture {
            session: target_session.to_string(),
            capture_ms,
            result,
        });
    }

    pub(super) fn sync_interactive_session_geometry(&mut self) {
        let Some(target_session) = self.interactive_target_session() else {
            return;
        };
        let Some((pane_width, pane_height)) = self.preview_output_dimensions() else {
            return;
        };

        let needs_resize = self.interactive.as_ref().is_some_and(|state| {
            state.pane_width != pane_width || state.pane_height != pane_height
        });
        if !needs_resize {
            return;
        }

        if let Some(state) = self.interactive.as_mut() {
            state.update_cursor(
                state.cursor_row,
                state.cursor_col,
                state.cursor_visible,
                pane_height,
                pane_width,
            );
        }

        if let Err(error) = self
            .tmux_input
            .resize_session(&target_session, pane_width, pane_height)
        {
            let message = error.to_string();
            self.last_tmux_error = Some(message.clone());
            self.log_tmux_error(message);
            self.pending_resize_verification = None;
            return;
        }

        self.pending_resize_verification = Some(PendingResizeVerification {
            session: target_session,
            expected_width: pane_width,
            expected_height: pane_height,
            retried: false,
        });
    }

    fn apply_live_preview_capture(
        &mut self,
        session_name: &str,
        include_escape_sequences: bool,
        capture_ms: u64,
        base_total_ms: u64,
        result: Result<String, String>,
    ) {
        match result {
            Ok(output) => {
                let apply_started_at = Instant::now();
                let update = self.preview.apply_capture(&output);
                let apply_capture_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(apply_started_at),
                );
                let consumed_inputs = if update.changed_cleaned {
                    self.drain_pending_inputs_for_session(session_name)
                } else {
                    Vec::new()
                };
                self.output_changing = update.changed_cleaned;
                self.agent_output_changing = update.changed_cleaned && consumed_inputs.is_empty();
                self.push_agent_activity_frame(self.agent_output_changing);
                let selected_workspace_index =
                    self.state.selected_workspace().and_then(|workspace| {
                        if session_name_for_workspace_ref(workspace) != session_name {
                            return None;
                        }
                        Some(self.state.selected_index)
                    });
                if let Some(index) = selected_workspace_index {
                    let supported_agent = self.state.workspaces[index].supported_agent;
                    let workspace_path = self.state.workspaces[index].path.clone();
                    let workspace_agent = self.state.workspaces[index].agent;
                    let workspace_is_main = self.state.workspaces[index].is_main;
                    self.capture_changed_cleaned_for_workspace(&workspace_path, output.as_str());
                    let resolved_status = detect_status_with_session_override(
                        output.as_str(),
                        SessionActivity::Active,
                        workspace_is_main,
                        true,
                        supported_agent,
                        workspace_agent,
                        &workspace_path,
                    );
                    let workspace = &mut self.state.workspaces[index];
                    workspace.status = resolved_status;
                    workspace.is_orphaned = false;
                }
                self.last_tmux_error = None;
                self.event_log.log(
                    LogEvent::new("preview_poll", "capture_completed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data("apply_capture_ms", Value::from(apply_capture_ms))
                        .with_data(
                            "total_ms",
                            Value::from(base_total_ms.saturating_add(apply_capture_ms)),
                        )
                        .with_data(
                            "output_bytes",
                            Value::from(u64::try_from(output.len()).unwrap_or(u64::MAX)),
                        )
                        .with_data("changed", Value::from(update.changed_cleaned))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        ),
                );
                if update.changed_cleaned {
                    let line_count = u64::try_from(self.preview.lines.len()).unwrap_or(u64::MAX);
                    let now = Instant::now();
                    let mut output_event = LogEvent::new("preview_update", "output_changed")
                        .with_data("line_count", Value::from(line_count))
                        .with_data("session", Value::from(session_name.to_string()));
                    if let Some(first_input) = consumed_inputs.first() {
                        let last_index = consumed_inputs.len().saturating_sub(1);
                        let last_input = &consumed_inputs[last_index];
                        let oldest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.received_at),
                        );
                        let newest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.received_at),
                        );
                        let oldest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.forwarded_at),
                        );
                        let newest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.forwarded_at),
                        );
                        let consumed_count =
                            u64::try_from(consumed_inputs.len()).unwrap_or(u64::MAX);
                        let consumed_seq_first = first_input.seq;
                        let consumed_seq_last = last_input.seq;

                        output_event = output_event
                            .with_data("input_seq", Value::from(consumed_seq_first))
                            .with_data(
                                "input_to_preview_ms",
                                Value::from(oldest_input_to_preview_ms),
                            )
                            .with_data("tmux_to_preview_ms", Value::from(oldest_tmux_to_preview_ms))
                            .with_data("consumed_input_count", Value::from(consumed_count))
                            .with_data("consumed_input_seq_first", Value::from(consumed_seq_first))
                            .with_data("consumed_input_seq_last", Value::from(consumed_seq_last))
                            .with_data(
                                "newest_input_to_preview_ms",
                                Value::from(newest_input_to_preview_ms),
                            )
                            .with_data(
                                "newest_tmux_to_preview_ms",
                                Value::from(newest_tmux_to_preview_ms),
                            );

                        self.log_input_event_with_fields(
                            "interactive_input_to_preview",
                            consumed_seq_first,
                            vec![
                                ("session".to_string(), Value::from(session_name.to_string())),
                                (
                                    "input_to_preview_ms".to_string(),
                                    Value::from(oldest_input_to_preview_ms),
                                ),
                                (
                                    "tmux_to_preview_ms".to_string(),
                                    Value::from(oldest_tmux_to_preview_ms),
                                ),
                                (
                                    "newest_input_to_preview_ms".to_string(),
                                    Value::from(newest_input_to_preview_ms),
                                ),
                                (
                                    "newest_tmux_to_preview_ms".to_string(),
                                    Value::from(newest_tmux_to_preview_ms),
                                ),
                                (
                                    "consumed_input_count".to_string(),
                                    Value::from(consumed_count),
                                ),
                                (
                                    "consumed_input_seq_first".to_string(),
                                    Value::from(consumed_seq_first),
                                ),
                                (
                                    "consumed_input_seq_last".to_string(),
                                    Value::from(consumed_seq_last),
                                ),
                                (
                                    "queue_depth".to_string(),
                                    Value::from(self.pending_input_depth()),
                                ),
                            ],
                        );
                        if consumed_inputs.len() > 1 {
                            self.log_input_event_with_fields(
                                "interactive_inputs_coalesced",
                                consumed_seq_first,
                                vec![
                                    ("session".to_string(), Value::from(session_name.to_string())),
                                    (
                                        "consumed_input_count".to_string(),
                                        Value::from(consumed_count),
                                    ),
                                    (
                                        "consumed_input_seq_last".to_string(),
                                        Value::from(consumed_seq_last),
                                    ),
                                ],
                            );
                        }
                    }
                    self.event_log.log(output_event);
                }
            }
            Err(message) => {
                self.clear_agent_activity_tracking();
                let capture_error_indicates_missing_session =
                    tmux_capture_error_indicates_missing_session(&message);
                if capture_error_indicates_missing_session {
                    self.lazygit_ready_sessions.remove(session_name);
                    if let Some(workspace) = self.state.selected_workspace_mut()
                        && session_name_for_workspace_ref(workspace) == session_name
                    {
                        let workspace_path = workspace.path.clone();
                        workspace.status = if workspace.is_main {
                            WorkspaceStatus::Main
                        } else {
                            WorkspaceStatus::Idle
                        };
                        workspace.is_orphaned = !workspace.is_main;
                        self.clear_status_tracking_for_workspace_path(&workspace_path);
                    }
                    if self
                        .interactive
                        .as_ref()
                        .is_some_and(|interactive| interactive.target_session == session_name)
                    {
                        self.interactive = None;
                    }
                }
                self.last_tmux_error = Some(message.clone());
                self.event_log.log(
                    LogEvent::new("preview_poll", "capture_failed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        )
                        .with_data("error", Value::from(message.clone())),
                );
                self.log_tmux_error(message.clone());
                self.show_toast("preview capture failed", true);
                self.refresh_preview_summary();
            }
        }
    }

    fn apply_cursor_capture_result(&mut self, cursor_capture: CursorCapture) {
        let parse_started_at = Instant::now();
        let raw_metadata = match cursor_capture.result {
            Ok(raw_metadata) => raw_metadata,
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("preview_poll", "cursor_capture_failed")
                        .with_data("session", Value::from(cursor_capture.session))
                        .with_data("duration_ms", Value::from(cursor_capture.capture_ms))
                        .with_data("error", Value::from(error)),
                );
                return;
            }
        };
        let metadata = match parse_cursor_metadata(&raw_metadata) {
            Some(metadata) => metadata,
            None => {
                self.event_log.log(
                    LogEvent::new("preview_poll", "cursor_parse_failed")
                        .with_data("session", Value::from(cursor_capture.session))
                        .with_data("capture_ms", Value::from(cursor_capture.capture_ms))
                        .with_data(
                            "parse_ms",
                            Value::from(Self::duration_millis(
                                Instant::now().saturating_duration_since(parse_started_at),
                            )),
                        )
                        .with_data("raw_metadata", Value::from(raw_metadata)),
                );
                return;
            }
        };
        let Some(state) = self.interactive.as_mut() else {
            return;
        };
        let session = cursor_capture.session.clone();

        let changed = state.update_cursor(
            metadata.cursor_row,
            metadata.cursor_col,
            metadata.cursor_visible,
            metadata.pane_height,
            metadata.pane_width,
        );
        self.verify_resize_after_cursor_capture(
            &session,
            metadata.pane_width,
            metadata.pane_height,
        );
        let parse_duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(parse_started_at));
        self.event_log.log(
            LogEvent::new("preview_poll", "cursor_capture_completed")
                .with_data("session", Value::from(session))
                .with_data("capture_ms", Value::from(cursor_capture.capture_ms))
                .with_data("parse_ms", Value::from(parse_duration_ms))
                .with_data("changed", Value::from(changed))
                .with_data("cursor_visible", Value::from(metadata.cursor_visible))
                .with_data("cursor_row", Value::from(metadata.cursor_row))
                .with_data("cursor_col", Value::from(metadata.cursor_col))
                .with_data("pane_width", Value::from(metadata.pane_width))
                .with_data("pane_height", Value::from(metadata.pane_height)),
        );
    }

    fn verify_resize_after_cursor_capture(
        &mut self,
        session: &str,
        pane_width: u16,
        pane_height: u16,
    ) {
        let Some(pending) = self.pending_resize_verification.clone() else {
            return;
        };
        if pending.session != session {
            return;
        }

        if pending.expected_width == pane_width && pending.expected_height == pane_height {
            self.pending_resize_verification = None;
            return;
        }

        if pending.retried {
            self.event_log.log(
                LogEvent::new("preview_poll", "resize_verify_failed")
                    .with_data("session", Value::from(session.to_string()))
                    .with_data("expected_width", Value::from(pending.expected_width))
                    .with_data("expected_height", Value::from(pending.expected_height))
                    .with_data("actual_width", Value::from(pane_width))
                    .with_data("actual_height", Value::from(pane_height)),
            );
            self.pending_resize_verification = None;
            return;
        }

        self.event_log.log(
            LogEvent::new("preview_poll", "resize_verify_retry")
                .with_data("session", Value::from(session.to_string()))
                .with_data("expected_width", Value::from(pending.expected_width))
                .with_data("expected_height", Value::from(pending.expected_height))
                .with_data("actual_width", Value::from(pane_width))
                .with_data("actual_height", Value::from(pane_height)),
        );
        self.pending_resize_verification = Some(PendingResizeVerification {
            retried: true,
            ..pending.clone()
        });
        if let Err(error) =
            self.tmux_input
                .resize_session(session, pending.expected_width, pending.expected_height)
        {
            let message = error.to_string();
            self.last_tmux_error = Some(message.clone());
            self.log_tmux_error(message);
            self.pending_resize_verification = None;
            return;
        }

        self.poll_preview();
    }

    fn poll_preview_sync(&mut self) {
        let live_preview = self.prepare_live_preview_session();
        let has_live_preview = live_preview.is_some();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = workspace_status_targets_for_polling_with_live_preview(
            &self.state.workspaces,
            self.multiplexer,
            live_preview.as_ref(),
        );

        if let Some(live_preview_target) = live_preview {
            let capture_started_at = Instant::now();
            let result = self
                .tmux_input
                .capture_output(
                    &live_preview_target.session_name,
                    600,
                    live_preview_target.include_escape_sequences,
                )
                .map_err(|error| error.to_string());
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            self.apply_live_preview_capture(
                &live_preview_target.session_name,
                live_preview_target.include_escape_sequences,
                capture_ms,
                capture_ms,
                result,
            );
        } else {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
        }

        for target in status_poll_targets {
            let capture_started_at = Instant::now();
            let result = self
                .tmux_input
                .capture_output(&target.session_name, 120, false)
                .map_err(|error| error.to_string());
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            self.apply_workspace_status_capture(WorkspaceStatusCapture {
                workspace_name: target.workspace_name,
                workspace_path: target.workspace_path,
                session_name: target.session_name,
                supported_agent: target.supported_agent,
                capture_ms,
                result,
            });
        }
        if !has_live_preview {
            self.refresh_preview_summary();
        }

        if let Some(target_session) = cursor_session {
            self.poll_interactive_cursor_sync(&target_session);
        }
    }

    fn schedule_async_preview_poll(
        &self,
        generation: u64,
        live_preview: Option<LivePreviewTarget>,
        cursor_session: Option<String>,
        status_poll_targets: Vec<WorkspaceStatusPollTarget>,
    ) -> Cmd<Msg> {
        Cmd::task(move || {
            let live_capture = live_preview.map(|target| {
                let capture_started_at = Instant::now();
                let result = CommandTmuxInput::capture_session_output(
                    &target.session_name,
                    600,
                    target.include_escape_sequences,
                )
                .map_err(|error| error.to_string());
                let capture_ms = GroveApp::duration_millis(
                    Instant::now().saturating_duration_since(capture_started_at),
                );
                LivePreviewCapture {
                    session: target.session_name,
                    include_escape_sequences: target.include_escape_sequences,
                    capture_ms,
                    total_ms: capture_ms,
                    result,
                }
            });

            let cursor_capture = cursor_session.map(|session| {
                let started_at = Instant::now();
                let result = CommandTmuxInput::capture_session_cursor_metadata(&session)
                    .map_err(|error| error.to_string());
                let capture_ms =
                    GroveApp::duration_millis(Instant::now().saturating_duration_since(started_at));
                CursorCapture {
                    session,
                    capture_ms,
                    result,
                }
            });

            let workspace_status_captures = status_poll_targets
                .into_iter()
                .map(|target| {
                    let capture_started_at = Instant::now();
                    let result =
                        CommandTmuxInput::capture_session_output(&target.session_name, 120, false)
                            .map_err(|error| error.to_string());
                    let capture_ms = GroveApp::duration_millis(
                        Instant::now().saturating_duration_since(capture_started_at),
                    );
                    WorkspaceStatusCapture {
                        workspace_name: target.workspace_name,
                        workspace_path: target.workspace_path,
                        session_name: target.session_name,
                        supported_agent: target.supported_agent,
                        capture_ms,
                        result,
                    }
                })
                .collect();

            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation,
                live_capture,
                cursor_capture,
                workspace_status_captures,
            })
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

    fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
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

    pub(super) fn scroll_preview(&mut self, delta: i32) {
        let viewport_height = self
            .preview_output_dimensions()
            .map_or(1, |(_, height)| usize::from(height));
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        let changed = self.preview.scroll(delta, Instant::now(), viewport_height);
        if changed {
            let offset = u64::try_from(self.preview.offset).unwrap_or(u64::MAX);
            self.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from(i64::from(delta)))
                    .with_data("offset", Value::from(offset)),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "previous_offset",
                        Value::from(u64::try_from(old_offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
    }

    pub(super) fn jump_preview_to_bottom(&mut self) {
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        self.preview.jump_to_bottom();
        if old_offset != self.preview.offset {
            self.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from("jump_bottom"))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "previous_offset",
                        Value::from(u64::try_from(old_offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
    }

    fn next_poll_interval(&self) -> Duration {
        let status = self.selected_workspace_status();

        let since_last_key = self
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            status,
            true,
            self.state.focus == PaneFocus::Preview,
            self.interactive.is_some(),
            since_last_key,
            self.output_changing,
        )
    }

    fn selected_workspace_status(&self) -> WorkspaceStatus {
        self.state
            .selected_workspace()
            .map_or(WorkspaceStatus::Unknown, |workspace| workspace.status)
    }

    pub(super) fn clear_agent_activity_tracking(&mut self) {
        self.output_changing = false;
        self.agent_output_changing = false;
        self.agent_activity_frames.clear();
    }

    fn workspace_status_tracking_key(workspace_path: &Path) -> String {
        workspace_path.to_string_lossy().to_string()
    }

    pub(super) fn clear_status_tracking_for_workspace_path(&mut self, workspace_path: &Path) {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_status_digests.remove(&key);
        self.workspace_output_changing.remove(&key);
    }

    pub(super) fn clear_status_tracking(&mut self) {
        self.workspace_status_digests.clear();
        self.workspace_output_changing.clear();
    }

    fn capture_changed_cleaned_for_workspace(
        &mut self,
        workspace_path: &Path,
        output: &str,
    ) -> bool {
        let key = Self::workspace_status_tracking_key(workspace_path);
        let previous_digest = self.workspace_status_digests.get(&key);
        let change = evaluate_capture_change(previous_digest, output);
        self.workspace_status_digests
            .insert(key.clone(), change.digest);
        self.workspace_output_changing
            .insert(key, change.changed_cleaned);
        change.changed_cleaned
    }

    fn workspace_output_changing(&self, workspace_path: &Path) -> bool {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_output_changing
            .get(&key)
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn push_agent_activity_frame(&mut self, changed: bool) {
        if self.agent_activity_frames.len() >= AGENT_ACTIVITY_WINDOW_FRAMES {
            self.agent_activity_frames.pop_front();
        }
        self.agent_activity_frames.push_back(changed);
    }

    fn has_recent_agent_activity(&self) -> bool {
        self.agent_activity_frames
            .iter()
            .copied()
            .any(|changed| changed)
    }

    fn visual_tick_interval(&self) -> Option<Duration> {
        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.as_path());
        if self.status_is_visually_working(
            selected_workspace_path,
            self.selected_workspace_status(),
            true,
        ) {
            return Some(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS));
        }
        None
    }

    fn advance_visual_animation(&mut self) {
        self.fast_animation_frame = self.fast_animation_frame.wrapping_add(1);
    }

    pub(super) fn status_is_visually_working(
        &self,
        workspace_path: Option<&Path>,
        status: WorkspaceStatus,
        is_selected: bool,
    ) -> bool {
        if is_selected
            && self.interactive.as_ref().is_some_and(|interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
                    < Duration::from_millis(LOCAL_TYPING_SUPPRESS_MS)
            })
        {
            return false;
        }
        match status {
            WorkspaceStatus::Thinking => true,
            WorkspaceStatus::Active => {
                if workspace_path.is_some_and(|path| self.workspace_output_changing(path)) {
                    return true;
                }
                if is_selected {
                    return self.agent_output_changing || self.has_recent_agent_activity();
                }
                false
            }
            _ => false,
        }
    }

    fn is_due_with_tolerance(now: Instant, due_at: Instant) -> bool {
        let tolerance = Duration::from_millis(TICK_EARLY_TOLERANCE_MS);
        let now_with_tolerance = now.checked_add(tolerance).unwrap_or(now);
        now_with_tolerance >= due_at
    }

    fn schedule_next_tick(&mut self) -> Cmd<Msg> {
        let scheduled_at = Instant::now();
        let mut poll_due_at = scheduled_at + self.next_poll_interval();
        let mut source = "adaptive_poll";
        if let Some(interactive_due_at) = self.interactive_poll_due_at
            && interactive_due_at < poll_due_at
        {
            poll_due_at = interactive_due_at;
            source = "interactive_debounce";
        }

        if let Some(existing_poll_due_at) = self.next_poll_due_at
            && existing_poll_due_at <= poll_due_at
        {
            if existing_poll_due_at > scheduled_at {
                poll_due_at = existing_poll_due_at;
                source = "retained_poll";
            } else {
                poll_due_at = scheduled_at;
                source = "overdue_poll";
            }
        }
        self.next_poll_due_at = Some(poll_due_at);

        self.next_visual_due_at = if let Some(interval) = self.visual_tick_interval() {
            let candidate = scheduled_at + interval;
            Some(
                if let Some(existing_visual_due_at) = self.next_visual_due_at {
                    if existing_visual_due_at <= candidate && existing_visual_due_at > scheduled_at
                    {
                        existing_visual_due_at
                    } else {
                        candidate
                    }
                } else {
                    candidate
                },
            )
        } else {
            None
        };

        let mut due_at = poll_due_at;
        let mut trigger = "poll";
        if let Some(visual_due_at) = self.next_visual_due_at
            && visual_due_at < due_at
        {
            due_at = visual_due_at;
            trigger = "visual";
        }

        if let Some(existing_due_at) = self.next_tick_due_at
            && existing_due_at <= due_at
            && existing_due_at > scheduled_at
        {
            self.event_log.log(
                LogEvent::new("tick", "retained")
                    .with_data("source", Value::from(source))
                    .with_data("trigger", Value::from(trigger))
                    .with_data(
                        "interval_ms",
                        Value::from(Self::duration_millis(
                            existing_due_at.saturating_duration_since(scheduled_at),
                        )),
                    )
                    .with_data("pending_depth", Value::from(self.pending_input_depth()))
                    .with_data(
                        "oldest_pending_age_ms",
                        Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                    ),
            );
            return Cmd::None;
        }

        let interval = due_at.saturating_duration_since(scheduled_at);
        let interval_ms = Self::duration_millis(interval);
        self.next_tick_due_at = Some(due_at);
        self.next_tick_interval_ms = Some(interval_ms);
        self.event_log.log(
            LogEvent::new("tick", "scheduled")
                .with_data("source", Value::from(source))
                .with_data("trigger", Value::from(trigger))
                .with_data("interval_ms", Value::from(interval_ms))
                .with_data("pending_depth", Value::from(self.pending_input_depth()))
                .with_data(
                    "oldest_pending_age_ms",
                    Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                ),
        );
        Cmd::tick(interval)
    }

    fn tick_is_due(&self, now: Instant) -> bool {
        let Some(due_at) = self.next_tick_due_at else {
            return true;
        };

        Self::is_due_with_tolerance(now, due_at)
    }

    fn handle_paste_event(&mut self, paste_event: PasteEvent) -> Cmd<Msg> {
        let input_seq = self.next_input_seq();
        let received_at = Instant::now();
        let (target_session, bracketed) = {
            let Some(state) = self.interactive.as_mut() else {
                return Cmd::None;
            };
            state.bracketed_paste = paste_event.bracketed;
            (state.target_session.clone(), state.bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_paste_received",
            input_seq,
            vec![
                (
                    "chars".to_string(),
                    Value::from(
                        u64::try_from(paste_event.text.chars().count()).unwrap_or(u64::MAX),
                    ),
                ),
                ("bracketed".to_string(), Value::from(paste_event.bracketed)),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );

        let payload = encode_paste_payload(&paste_event.text, bracketed || paste_event.bracketed);
        let send_cmd = self.send_interactive_action(
            &InteractiveAction::SendLiteral(payload),
            &target_session,
            Some(InputTraceContext {
                seq: input_seq,
                received_at,
            }),
        );
        self.schedule_interactive_debounced_poll(received_at);
        send_cmd
    }

    pub(super) fn enter_preview_or_interactive(&mut self) {
        if !self.enter_interactive(Instant::now()) {
            reduce(&mut self.state, Action::EnterPreviewMode);
            self.poll_preview();
        }
    }

    fn non_interactive_command_for_key(&self, key_event: &KeyEvent) -> Option<UiCommand> {
        let in_preview_focus =
            self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview;
        let in_preview_agent = in_preview_focus && self.preview_tab == PreviewTab::Agent;
        let can_enter_interactive = workspace_can_enter_interactive(
            self.state.selected_workspace(),
            self.preview_tab == PreviewTab::Git,
        );

        for command in UiCommand::all() {
            let matches = match command {
                UiCommand::ToggleFocus => {
                    matches!(
                        key_event.code,
                        KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l')
                    )
                }
                UiCommand::ToggleSidebar => matches!(key_event.code, KeyCode::Char('\\')),
                UiCommand::OpenPreview => match key_event.code {
                    KeyCode::Enter => !in_preview_focus || !can_enter_interactive,
                    _ => false,
                },
                UiCommand::EnterInteractive => {
                    matches!(key_event.code, KeyCode::Enter)
                        && in_preview_focus
                        && can_enter_interactive
                }
                UiCommand::FocusPreview => false,
                UiCommand::FocusList => matches!(key_event.code, KeyCode::Escape),
                UiCommand::MoveSelectionUp => {
                    matches!(key_event.code, KeyCode::Char('k') | KeyCode::Up) && !in_preview_focus
                }
                UiCommand::MoveSelectionDown => {
                    matches!(key_event.code, KeyCode::Char('j') | KeyCode::Down)
                        && !in_preview_focus
                }
                UiCommand::ScrollUp => {
                    matches!(key_event.code, KeyCode::Char('k') | KeyCode::Up) && in_preview_agent
                }
                UiCommand::ScrollDown => {
                    matches!(key_event.code, KeyCode::Char('j') | KeyCode::Down) && in_preview_agent
                }
                UiCommand::PageUp => matches!(key_event.code, KeyCode::PageUp) && in_preview_agent,
                UiCommand::PageDown => {
                    matches!(key_event.code, KeyCode::PageDown) && in_preview_agent
                }
                UiCommand::ScrollBottom => {
                    matches!(key_event.code, KeyCode::Char('G')) && in_preview_agent
                }
                UiCommand::PreviousTab => {
                    matches!(key_event.code, KeyCode::Char('[')) && in_preview_focus
                }
                UiCommand::NextTab => {
                    matches!(key_event.code, KeyCode::Char(']')) && in_preview_focus
                }
                UiCommand::NewWorkspace => {
                    matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'))
                }
                UiCommand::EditWorkspace => {
                    matches!(key_event.code, KeyCode::Char('e') | KeyCode::Char('E'))
                }
                UiCommand::StartAgent => matches!(key_event.code, KeyCode::Char('s')),
                UiCommand::StopAgent => matches!(key_event.code, KeyCode::Char('x')),
                UiCommand::DeleteWorkspace => matches!(key_event.code, KeyCode::Char('D')),
                UiCommand::MergeWorkspace => matches!(key_event.code, KeyCode::Char('m')),
                UiCommand::UpdateFromBase => matches!(key_event.code, KeyCode::Char('u')),
                UiCommand::OpenProjects => {
                    matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'))
                }
                UiCommand::OpenSettings => matches!(key_event.code, KeyCode::Char('S')),
                UiCommand::ToggleUnsafe => matches!(key_event.code, KeyCode::Char('!')),
                UiCommand::OpenHelp => matches!(key_event.code, KeyCode::Char('?')),
                UiCommand::OpenCommandPalette => Self::is_ctrl_char_key(key_event, 'k'),
                UiCommand::Quit => {
                    matches!(key_event.code, KeyCode::Char('q')) && key_event.modifiers.is_empty()
                }
            };
            if matches {
                return Some(*command);
            }
        }

        None
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) -> bool {
        let Some(command) = self.non_interactive_command_for_key(&key_event) else {
            return false;
        };
        self.execute_ui_command(command)
    }

    fn sidebar_workspace_index_at_y(&self, y: u16) -> Option<usize> {
        if self.projects.is_empty() {
            return None;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            return None;
        }

        let layout = self.view_layout();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return None;
        }

        let target_row = usize::from(y.saturating_sub(sidebar_inner.y));
        let mut visual_row = 0usize;
        for (project_index, project) in self.projects.iter().enumerate() {
            if project_index > 0 {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
            }

            if visual_row == target_row {
                return None;
            }
            visual_row = visual_row.saturating_add(1);

            let workspace_indices: Vec<usize> = self
                .state
                .workspaces
                .iter()
                .enumerate()
                .filter(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| project_paths_equal(path, &project.path))
                })
                .map(|(index, _)| index)
                .collect();
            if workspace_indices.is_empty() {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
                continue;
            }

            for workspace_index in workspace_indices {
                if visual_row == target_row {
                    return Some(workspace_index);
                }
                visual_row = visual_row.saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT));
            }
        }

        None
    }

    fn select_workspace_by_mouse(&mut self, y: u16) {
        let Some(row) = self.sidebar_workspace_index_at_y(y) else {
            return;
        };

        if row != self.state.selected_index {
            self.state.selected_index = row;
            self.preview.jump_to_bottom();
            self.clear_agent_activity_tracking();
            self.clear_preview_selection();
            self.poll_preview();
        }
    }

    pub(super) fn select_workspace_by_index(&mut self, index: usize) {
        if index >= self.state.workspaces.len() {
            return;
        }
        if index == self.state.selected_index {
            return;
        }

        self.state.selected_index = index;
        self.preview.jump_to_bottom();
        self.clear_agent_activity_tracking();
        self.clear_preview_selection();
        self.poll_preview();
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(state) = self.interactive.as_mut() {
            state.note_mouse_event(Instant::now());
        }

        let (region, row_data) = self.hit_region_for_point(mouse_event.x, mouse_event.y);
        let mut event = LogEvent::new("mouse", "event")
            .with_data("x", Value::from(mouse_event.x))
            .with_data("y", Value::from(mouse_event.y))
            .with_data("kind", Value::from(format!("{:?}", mouse_event.kind)))
            .with_data("region", Value::from(Self::hit_region_name(region)))
            .with_data("modal_open", Value::from(self.modal_open()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("divider_drag_active", Value::from(self.divider_drag_active))
            .with_data("focus", Value::from(Self::focus_name(self.state.focus)))
            .with_data("mode", Value::from(Self::mode_name(self.state.mode)));
        if let Some(row_data) = row_data {
            event = event.with_data("row_data", Value::from(row_data));
        }
        if matches!(region, HitRegion::Preview)
            && let Some(point) = self.preview_text_point_at(mouse_event.x, mouse_event.y)
        {
            event = event
                .with_data(
                    "mapped_line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "mapped_col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "mapped_", point);
        }
        self.event_log.log(event);

        if self.modal_open() {
            return;
        }

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => match region {
                HitRegion::Divider => {
                    self.divider_drag_active = true;
                }
                HitRegion::WorkspaceList => {
                    self.state.focus = PaneFocus::WorkspaceList;
                    self.state.mode = UiMode::List;
                    if let Some(row_data) = row_data {
                        if let Ok(index) = usize::try_from(row_data) {
                            self.select_workspace_by_index(index);
                        }
                    } else {
                        self.select_workspace_by_mouse(mouse_event.y);
                    }
                }
                HitRegion::Preview => {
                    self.state.focus = PaneFocus::Preview;
                    self.state.mode = UiMode::Preview;
                    if self.interactive.is_some() {
                        self.prepare_preview_selection_drag(mouse_event.x, mouse_event.y);
                    } else {
                        self.clear_preview_selection();
                    }
                }
                HitRegion::StatusLine | HitRegion::Header | HitRegion::Outside => {}
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.divider_drag_active {
                    let ratio =
                        clamp_sidebar_ratio(ratio_from_drag(self.viewport_width, mouse_event.x));
                    if ratio != self.sidebar_width_pct {
                        self.sidebar_width_pct = ratio;
                        self.persist_sidebar_ratio();
                        self.sync_interactive_session_geometry();
                    }
                } else if self.interactive.is_some() {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Moved => {
                if self.interactive.is_some() && !self.divider_drag_active {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.divider_drag_active = false;
                self.finish_preview_selection_drag(mouse_event.x, mouse_event.y);
            }
            MouseEventKind::ScrollUp => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(-1);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(1);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_key(&mut self, key_event: KeyEvent) -> (bool, Cmd<Msg>) {
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return (false, Cmd::None);
        }

        if Self::is_ctrl_char_key(&key_event, 'k') {
            return (
                self.execute_ui_command(UiCommand::OpenCommandPalette),
                Cmd::None,
            );
        }

        if self.command_palette.is_visible() {
            let event = Event::Key(key_event);
            if let Some(action) = self.command_palette.handle_event(&event) {
                return match action {
                    PaletteAction::Dismiss => (false, Cmd::None),
                    PaletteAction::Execute(id) => {
                        (self.execute_command_palette_action(id.as_str()), Cmd::None)
                    }
                };
            }
            return (false, Cmd::None);
        }

        if self.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        if self.create_dialog.is_some()
            && key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('p'))
        {
            self.handle_create_dialog_key(key_event);
            return (false, Cmd::None);
        }

        let keybinding_state = self.keybinding_state();
        if let Some(action) = self
            .action_mapper
            .map(&key_event, &keybinding_state, Instant::now())
        {
            if !matches!(action, KeybindingAction::PassThrough) {
                return (self.apply_keybinding_action(action), Cmd::None);
            }
        } else {
            return (false, Cmd::None);
        }

        if self.create_dialog.is_some() {
            self.handle_create_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.edit_dialog.is_some() {
            self.handle_edit_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.launch_dialog.is_some() {
            self.handle_launch_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.delete_dialog.is_some() {
            self.handle_delete_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.merge_dialog.is_some() {
            self.handle_merge_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.update_from_base_dialog.is_some() {
            self.handle_update_from_base_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.project_dialog.is_some() {
            self.handle_project_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.settings_dialog.is_some() {
            self.handle_settings_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.keybind_help_open {
            self.handle_keybind_help_key(key_event);
            return (false, Cmd::None);
        }

        (self.handle_non_interactive_key(key_event), Cmd::None)
    }

    fn map_interactive_key(key_event: KeyEvent) -> Option<InteractiveKey> {
        let ctrl = key_event.modifiers.contains(Modifiers::CTRL);
        let alt = key_event.modifiers.contains(Modifiers::ALT);

        match key_event.code {
            KeyCode::Enter => Some(InteractiveKey::Enter),
            KeyCode::Tab => Some(InteractiveKey::Tab),
            KeyCode::Backspace => Some(InteractiveKey::Backspace),
            KeyCode::Delete => Some(InteractiveKey::Delete),
            KeyCode::Up => Some(InteractiveKey::Up),
            KeyCode::Down => Some(InteractiveKey::Down),
            KeyCode::Left => Some(InteractiveKey::Left),
            KeyCode::Right => Some(InteractiveKey::Right),
            KeyCode::Home => Some(InteractiveKey::Home),
            KeyCode::End => Some(InteractiveKey::End),
            KeyCode::PageUp => Some(InteractiveKey::PageUp),
            KeyCode::PageDown => Some(InteractiveKey::PageDown),
            KeyCode::Escape => Some(InteractiveKey::Escape),
            KeyCode::F(index) => Some(InteractiveKey::Function(index)),
            KeyCode::Char(character) => {
                if (ctrl && matches!(character, '\\' | '|' | '4')) || character == '\u{1c}' {
                    return Some(InteractiveKey::CtrlBackslash);
                }
                if alt && matches!(character, 'c' | 'C') {
                    return Some(InteractiveKey::AltC);
                }
                if alt && matches!(character, 'v' | 'V') {
                    return Some(InteractiveKey::AltV);
                }
                if ctrl {
                    return Some(InteractiveKey::Ctrl(character));
                }
                Some(InteractiveKey::Char(character))
            }
            _ => None,
        }
    }

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
        Cmd::task(move || {
            let started_at = Instant::now();
            let execution = CommandTmuxInput::execute_command(&command);
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

    fn handle_interactive_send_completed(
        &mut self,
        completion: InteractiveSendCompletion,
    ) -> Cmd<Msg> {
        let InteractiveSendCompletion {
            send:
                QueuedInteractiveSend {
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
                ("tmux_send_ms".to_string(), Value::from(tmux_send_ms)),
                (
                    "queue_depth".to_string(),
                    Value::from(
                        u64::try_from(self.pending_interactive_inputs.len()).unwrap_or(u64::MAX),
                    ),
                ),
            ];
            if let Some(literal_chars) = literal_chars {
                fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
            }
            self.log_input_event_with_fields("interactive_forwarded", trace_context.seq, fields);
        }
        self.dispatch_next_interactive_send()
    }

    fn send_interactive_action(
        &mut self,
        action: &InteractiveAction,
        target_session: &str,
        trace_context: Option<InputTraceContext>,
    ) -> Cmd<Msg> {
        let Some(command) =
            multiplexer_send_input_command(self.multiplexer, target_session, action)
        else {
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
            Some(u64::try_from(text.chars().count()).unwrap_or(u64::MAX))
        } else {
            None
        };

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
        match self.execute_tmux_command(&command) {
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
                        ("tmux_send_ms".to_string(), Value::from(send_duration_ms)),
                        (
                            "queue_depth".to_string(),
                            Value::from(
                                u64::try_from(self.pending_interactive_inputs.len())
                                    .unwrap_or(u64::MAX),
                            ),
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
                            ("error".to_string(), Value::from(error.to_string())),
                        ],
                    );
                }
            }
        }
        Cmd::None
    }

    fn copy_interactive_capture(&mut self) {
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

    fn paste_clipboard_text(
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

        match self.tmux_input.paste_buffer(target_session, &text) {
            Ok(()) => {
                self.last_tmux_error = None;
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
                            ("error".to_string(), Value::from(message)),
                        ],
                    );
                }
            }
        }

        Cmd::None
    }

    fn handle_interactive_key(&mut self, key_event: KeyEvent) -> Cmd<Msg> {
        let now = Instant::now();
        let input_seq = self.next_input_seq();
        if let KeyCode::Char(character) = key_event.code
            && key_event.modifiers.is_empty()
            && let Some(state) = self.interactive.as_mut()
            && state.should_drop_split_mouse_fragment(character, now)
        {
            self.log_input_event_with_fields(
                "interactive_key_dropped_mouse_fragment",
                input_seq,
                vec![
                    ("code".to_string(), Value::from("char")),
                    ("modifiers".to_string(), Value::from("none")),
                ],
            );
            return Cmd::None;
        }

        let Some(interactive_key) = Self::map_interactive_key(key_event) else {
            self.log_input_event_with_fields(
                "interactive_key_unmapped",
                input_seq,
                vec![(
                    "code".to_string(),
                    Value::from(format!("{:?}", key_event.code)),
                )],
            );
            return Cmd::None;
        };
        self.log_input_event_with_fields(
            "interactive_key_received",
            input_seq,
            vec![
                (
                    "key".to_string(),
                    Value::from(Self::interactive_key_kind(&interactive_key)),
                ),
                (
                    "repeat".to_string(),
                    Value::from(matches!(key_event.kind, KeyEventKind::Repeat)),
                ),
            ],
        );

        let (action, target_session, bracketed_paste) = {
            let Some(state) = self.interactive.as_mut() else {
                return Cmd::None;
            };
            let action = state.handle_key(interactive_key, now);
            let session = state.target_session.clone();
            let bracketed_paste = state.bracketed_paste;
            (action, session, bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_action_selected",
            input_seq,
            vec![
                (
                    "action".to_string(),
                    Value::from(Self::interactive_action_kind(&action)),
                ),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );
        let trace_context = InputTraceContext {
            seq: input_seq,
            received_at: now,
        };

        match action {
            InteractiveAction::ExitInteractive => {
                self.interactive = None;
                self.state.mode = UiMode::Preview;
                self.state.focus = PaneFocus::Preview;
                self.clear_preview_selection();
                Cmd::None
            }
            InteractiveAction::CopySelection => {
                self.copy_interactive_capture();
                Cmd::None
            }
            InteractiveAction::PasteClipboard => {
                if self.preview.offset > 0 {
                    self.preview.jump_to_bottom();
                }
                let send_cmd = self.paste_clipboard_text(
                    &target_session,
                    bracketed_paste,
                    Some(trace_context),
                );
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
            InteractiveAction::Noop
            | InteractiveAction::SendNamed(_)
            | InteractiveAction::SendLiteral(_) => {
                let send_cmd =
                    self.send_interactive_action(&action, &target_session, Some(trace_context));
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
        }
    }

    fn is_ctrl_char_key(key_event: &KeyEvent, character: char) -> bool {
        matches!(
            key_event.code,
            KeyCode::Char(value)
                if value == character
                    && key_event.kind == KeyEventKind::Press
                    && key_event.modifiers == Modifiers::CTRL
        )
    }

    fn keybinding_task_running(&self) -> bool {
        self.refresh_in_flight
            || self.delete_in_flight
            || self.merge_in_flight
            || self.update_from_base_in_flight
            || self.create_in_flight
            || self.start_in_flight
            || self.stop_in_flight
    }

    fn keybinding_input_nonempty(&self) -> bool {
        if let Some(dialog) = self.launch_dialog.as_ref() {
            return !dialog.prompt.is_empty() || !dialog.pre_launch_command.is_empty();
        }
        if let Some(dialog) = self.create_dialog.as_ref() {
            return !dialog.workspace_name.is_empty() || !dialog.base_branch.is_empty();
        }
        if let Some(project_dialog) = self.project_dialog.as_ref() {
            if !project_dialog.filter.is_empty() {
                return true;
            }
            if let Some(add_dialog) = project_dialog.add_dialog.as_ref() {
                return !add_dialog.name.is_empty() || !add_dialog.path.is_empty();
            }
        }

        false
    }

    fn keybinding_state(&self) -> KeybindingAppState {
        KeybindingAppState::new()
            .with_input(self.keybinding_input_nonempty())
            .with_task(self.keybinding_task_running())
            .with_modal(self.modal_open())
    }

    pub(super) fn preview_agent_tab_is_focused(&self) -> bool {
        self.state.mode == UiMode::Preview
            && self.state.focus == PaneFocus::Preview
            && self.preview_tab == PreviewTab::Agent
    }

    pub(super) fn preview_git_tab_is_focused(&self) -> bool {
        self.state.mode == UiMode::Preview
            && self.state.focus == PaneFocus::Preview
            && self.preview_tab == PreviewTab::Git
    }

    fn apply_keybinding_action(&mut self, action: KeybindingAction) -> bool {
        match action {
            KeybindingAction::DismissModal => {
                if self.create_dialog.is_some() {
                    self.log_dialog_event("create", "dialog_cancelled");
                    self.create_dialog = None;
                    self.clear_create_branch_picker();
                } else if self.edit_dialog.is_some() {
                    self.log_dialog_event("edit", "dialog_cancelled");
                    self.edit_dialog = None;
                } else if self.launch_dialog.is_some() {
                    self.log_dialog_event("launch", "dialog_cancelled");
                    self.launch_dialog = None;
                } else if self.delete_dialog.is_some() {
                    self.log_dialog_event("delete", "dialog_cancelled");
                    self.delete_dialog = None;
                } else if self.merge_dialog.is_some() {
                    self.log_dialog_event("merge", "dialog_cancelled");
                    self.merge_dialog = None;
                } else if self.update_from_base_dialog.is_some() {
                    self.log_dialog_event("update_from_base", "dialog_cancelled");
                    self.update_from_base_dialog = None;
                } else if self.settings_dialog.is_some() {
                    self.log_dialog_event("settings", "dialog_cancelled");
                    self.settings_dialog = None;
                } else if self.project_dialog.is_some() {
                    self.project_dialog = None;
                } else if self.keybind_help_open {
                    self.keybind_help_open = false;
                }
                false
            }
            KeybindingAction::ClearInput => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => dialog.prompt.clear(),
                        LaunchDialogField::PreLaunchCommand => dialog.pre_launch_command.clear(),
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                    return false;
                }
                if let Some(dialog) = self.create_dialog.as_mut() {
                    let mut refresh_base_branch = false;
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => dialog.workspace_name.clear(),
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.clear();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                    if refresh_base_branch {
                        self.refresh_create_branch_filtered();
                    }
                }
                false
            }
            KeybindingAction::CancelTask => {
                self.show_toast("cannot cancel running lifecycle task", true);
                false
            }
            KeybindingAction::Quit | KeybindingAction::HardQuit => true,
            KeybindingAction::SoftQuit => !self.keybinding_task_running(),
            KeybindingAction::CloseOverlay
            | KeybindingAction::ToggleTreeView
            | KeybindingAction::Bell
            | KeybindingAction::PassThrough => false,
        }
    }

    pub(super) fn enter_interactive(&mut self, now: Instant) -> bool {
        if !workspace_can_enter_interactive(
            self.state.selected_workspace(),
            self.preview_tab == PreviewTab::Git,
        ) {
            return false;
        }

        let git_preview_session = if self.preview_tab == PreviewTab::Git {
            let Some(target) = self.prepare_live_preview_session() else {
                return false;
            };
            Some(target.session_name)
        } else {
            None
        };

        let Some(session_name) = workspace_session_for_preview_tab(
            self.state.selected_workspace(),
            self.preview_tab == PreviewTab::Git,
            git_preview_session.as_deref(),
        ) else {
            return false;
        };

        self.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            session_name,
            now,
            self.viewport_height,
            self.viewport_width,
        ));
        self.interactive_poll_due_at = None;
        self.last_tmux_error = None;
        self.state.mode = UiMode::Preview;
        self.state.focus = PaneFocus::Preview;
        self.clear_preview_selection();
        self.sync_interactive_session_geometry();
        self.poll_preview();
        true
    }

    pub(super) fn open_keybind_help(&mut self) {
        if self.modal_open() {
            return;
        }
        self.keybind_help_open = true;
    }

    fn persist_sidebar_ratio(&mut self) {
        if let Err(error) = fs::write(
            &self.sidebar_ratio_path,
            serialize_sidebar_ratio(self.sidebar_width_pct),
        ) {
            self.last_tmux_error = Some(format!("sidebar ratio persist failed: {error}"));
        }
    }

    pub(super) fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.preview.jump_to_bottom();
            self.clear_agent_activity_tracking();
            self.clear_preview_selection();
            self.poll_preview();
        }
    }
}
