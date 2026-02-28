use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransitionSnapshot {
    selected_index: usize,
    selected_workspace: Option<String>,
    focus: PaneFocus,
    mode: UiMode,
    interactive_session: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastSeverity {
    Success,
    Error,
    Info,
}

impl GroveApp {
    pub(super) fn mode_label(&self) -> &'static str {
        if self.interactive.is_some() {
            return "Interactive";
        }
        self.state.mode.label()
    }

    pub(super) fn focus_label(&self) -> &'static str {
        self.state.focus.label()
    }

    pub(super) fn hit_region_name(region: HitRegion) -> &'static str {
        match region {
            HitRegion::WorkspaceList => "workspace_list",
            HitRegion::WorkspacePullRequest => "workspace_pull_request",
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

    fn toast_max_width(&self) -> u16 {
        self.viewport_width.saturating_sub(4).clamp(72, 180)
    }

    fn toast_message_for_width(raw: impl Into<String>, max_width: u16) -> String {
        let normalized = raw
            .into()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");
        if normalized.is_empty() {
            return String::new();
        }

        let max_message_width = usize::from(max_width).saturating_sub(8).max(16);
        truncate_to_display_width(normalized.as_str(), max_message_width)
    }

    pub(super) fn msg_kind(msg: &Msg) -> &'static str {
        match msg {
            Msg::Tick => "tick",
            Msg::Key(_) => "key",
            Msg::Mouse(_) => "mouse",
            Msg::Paste(_) => "paste",
            Msg::Resize { .. } => "resize",
            Msg::PreviewPollCompleted(_) => "preview_poll_completed",
            Msg::LazygitLaunchCompleted(_) => "lazygit_launch_completed",
            Msg::WorkspaceShellLaunchCompleted(_) => "workspace_shell_launch_completed",
            Msg::RefreshWorkspacesCompleted(_) => "refresh_workspaces_completed",
            Msg::DeleteProjectCompleted(_) => "delete_project_completed",
            Msg::DeleteWorkspaceCompleted(_) => "delete_workspace_completed",
            Msg::MergeWorkspaceCompleted(_) => "merge_workspace_completed",
            Msg::UpdateWorkspaceFromBaseCompleted(_) => "update_workspace_from_base_completed",
            Msg::CreateWorkspaceCompleted(_) => "create_workspace_completed",
            Msg::StartAgentCompleted(_) => "start_agent_completed",
            Msg::StopAgentCompleted(_) => "stop_agent_completed",
            Msg::RestartAgentCompleted(_) => "restart_agent_completed",
            Msg::InteractiveSendCompleted(_) => "interactive_send_completed",
            Msg::Noop => "noop",
        }
    }

    pub(super) fn log_event_with_fields(
        &self,
        event: &str,
        kind: &str,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) {
        self.event_log
            .log(LogEvent::new(event, kind).with_data_fields(fields));
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
            let selection_index = usize_to_u64(after.selected_index);
            let workspace_value = after
                .selected_workspace
                .clone()
                .map(Value::from)
                .unwrap_or(Value::Null);
            self.log_event_with_fields(
                "state_change",
                "selection_changed",
                [
                    ("index".to_string(), Value::from(selection_index)),
                    ("workspace".to_string(), workspace_value),
                ],
            );
        }
        if after.focus != before.focus {
            self.log_event_with_fields(
                "state_change",
                "focus_changed",
                [("focus".to_string(), Value::from(after.focus.name()))],
            );
        }
        if after.mode != before.mode {
            self.log_event_with_fields(
                "mode_change",
                "mode_changed",
                [("mode".to_string(), Value::from(after.mode.name()))],
            );
        }
        match (&before.interactive_session, &after.interactive_session) {
            (None, Some(session)) => {
                self.log_event_with_fields(
                    "mode_change",
                    "interactive_entered",
                    [("session".to_string(), Value::from(session.clone()))],
                );
            }
            (Some(session), None) => {
                self.log_event_with_fields(
                    "mode_change",
                    "interactive_exited",
                    [("session".to_string(), Value::from(session.clone()))],
                );
                self.interactive_poll_due_at = None;
                self.pending_resize_verification = None;
                let pending_before = self.pending_interactive_inputs.len();
                self.clear_pending_inputs_for_session(session);
                let pending_after = self.pending_interactive_inputs.len();
                self.clear_pending_sends_for_session(session);
                if pending_before != pending_after {
                    self.log_event_with_fields(
                        "input",
                        "pending_inputs_cleared",
                        [
                            ("session".to_string(), Value::from(session.clone())),
                            (
                                "cleared".to_string(),
                                Value::from(usize_to_u64(
                                    pending_before.saturating_sub(pending_after),
                                )),
                            ),
                        ],
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
        let mut all_fields = vec![("kind".to_string(), Value::from(kind))];
        all_fields.extend(fields);
        self.log_event_with_fields("dialog", action, all_fields);
    }

    pub(super) fn log_dialog_event(&self, kind: &str, action: &str) {
        self.log_dialog_event_with_fields(kind, action, std::iter::empty());
    }

    pub(super) fn log_tmux_error(&self, message: String) {
        self.log_event_with_fields(
            "error",
            "tmux_error",
            [("message".to_string(), Value::from(message))],
        );
    }

    pub(super) fn execute_tmux_command(&mut self, command: &[String]) -> std::io::Result<()> {
        let started_at = Instant::now();
        let command_text = command.join(" ");
        self.log_event_with_fields(
            "tmux_cmd",
            "execute",
            [("command".to_string(), Value::from(command_text.clone()))],
        );
        let result = execute_command_with(command, |command| self.tmux_input.execute(command));
        let duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(started_at));
        let mut completion_fields = vec![
            ("command".to_string(), Value::from(command_text)),
            ("duration_ms".to_string(), Value::from(duration_ms)),
            ("ok".to_string(), Value::from(result.is_ok())),
        ];
        if let Err(error) = &result {
            completion_fields.push(("error".to_string(), Value::from(error.to_string())));
            self.log_tmux_error(error.to_string());
        }
        self.log_event_with_fields("tmux_cmd", "completed", completion_fields);
        result
    }

    pub(super) fn show_success_toast(&mut self, text: impl Into<String>) {
        self.show_toast(text, ToastSeverity::Success);
    }

    pub(super) fn show_error_toast(&mut self, text: impl Into<String>) {
        self.show_toast(text, ToastSeverity::Error);
    }

    pub(super) fn show_info_toast(&mut self, text: impl Into<String>) {
        self.show_toast(text, ToastSeverity::Info);
    }

    pub(super) fn show_toast(&mut self, text: impl Into<String>, severity: ToastSeverity) {
        let max_width = self.toast_max_width();
        let message = Self::toast_message_for_width(text, max_width);
        let is_error = matches!(severity, ToastSeverity::Error);
        self.log_event_with_fields(
            "toast",
            "toast_shown",
            [
                ("text".to_string(), Value::from(message.clone())),
                (
                    "severity".to_string(),
                    Value::from(match severity {
                        ToastSeverity::Success => "success",
                        ToastSeverity::Error => "error",
                        ToastSeverity::Info => "info",
                    }),
                ),
                ("is_error".to_string(), Value::from(is_error)),
            ],
        );

        let theme = ui_theme();
        let (title, icon, style_variant, duration, priority, color) = match severity {
            ToastSeverity::Success => (
                "Success",
                ToastIcon::Success,
                ToastStyle::Success,
                Duration::from_secs(4),
                NotificationPriority::Normal,
                theme.teal,
            ),
            ToastSeverity::Error => (
                "Error",
                ToastIcon::Error,
                ToastStyle::Error,
                Duration::from_secs(12),
                NotificationPriority::High,
                theme.red,
            ),
            ToastSeverity::Info => (
                "Info",
                ToastIcon::Info,
                ToastStyle::Info,
                Duration::from_secs(6),
                NotificationPriority::Normal,
                theme.blue,
            ),
        };
        let base_style = Style::new().bg(theme.surface0).fg(color);
        let title_style = Style::new().bg(theme.surface0).fg(color).bold();
        let icon_style = Style::new().bg(theme.surface0).fg(color).bold();
        let toast = Toast::new(message)
            .title(title)
            .icon(icon)
            .style_variant(style_variant)
            .style(base_style)
            .with_title_style(title_style)
            .with_icon_style(icon_style)
            .max_width(max_width)
            .duration(duration);
        let _ = self.notifications.push(toast, priority);
        let _ = self.notifications.tick(Duration::ZERO);
    }
}
