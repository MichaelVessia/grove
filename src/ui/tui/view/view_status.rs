use super::view_prelude::*;

impl GroveApp {
    fn footer_dialog_state_label(kind: &str) -> &'static str {
        match kind {
            "launch" => "Launch",
            "stop" => "Stop",
            "confirm" => "Confirm",
            "delete" => "Delete",
            "merge" => "Merge",
            "update_from_base" => "Update",
            "session_cleanup" => "Cleanup",
            "create" => "Create",
            "edit" => "Edit",
            "rename_tab" => "Rename",
            "project" => "Project",
            "settings" => "Settings",
            "performance" => "Performance",
            _ => "Dialog",
        }
    }

    fn footer_state_chip_label(&self) -> String {
        match &self.discovery_state {
            DiscoveryState::Error(_) => return "Discovery Error".to_string(),
            DiscoveryState::Empty => return "No Worktrees".to_string(),
            DiscoveryState::Ready => {}
        }

        if self.dialogs.command_palette.is_visible() {
            return "Palette".to_string();
        }

        if self.dialogs.keybind_help_open {
            return "Help".to_string();
        }

        if let Some(dialog_kind) = self.active_dialog_kind() {
            if dialog_kind == "performance" {
                return "Performance".to_string();
            }
            return format!("Dialog: {}", Self::footer_dialog_state_label(dialog_kind));
        }

        if self.session.interactive.is_some() {
            return "Interactive".to_string();
        }
        if self.task_reorder_active() {
            return "Task Reorder".to_string();
        }

        match self.state.focus {
            PaneFocus::WorkspaceList => "List".to_string(),
            PaneFocus::Preview => format!("Preview: {}", self.preview_tab.label()),
        }
    }

    #[cfg(test)]
    fn unsafe_label(&self) -> &'static str {
        if self.launch_skip_permissions {
            "on"
        } else {
            "off"
        }
    }

    #[cfg(test)]
    pub(super) fn status_bar_line(&self) -> String {
        if let Some(toast) = self.notifications.visible().last() {
            return match toast.config.style_variant {
                ToastStyle::Error => format!("Status: error: {}", toast.content.message),
                ToastStyle::Success => format!("Status: success: {}", toast.content.message),
                ToastStyle::Info => format!("Status: info: {}", toast.content.message),
                ToastStyle::Warning => format!("Status: warning: {}", toast.content.message),
                ToastStyle::Neutral => format!("Status: {}", toast.content.message),
            };
        }

        match &self.discovery_state {
            DiscoveryState::Error(message) => format!("Status: discovery error ({message})"),
            DiscoveryState::Empty => "Status: no worktrees found".to_string(),
            DiscoveryState::Ready => {
                if let Some(dialog) = self.create_dialog() {
                    return format!(
                        "Status: new task, field={}, name=\"{}\", pr_url=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.task_name,
                        dialog.pr_url.replace('\n', "\\n"),
                    );
                }
                if let Some(dialog) = self.launch_dialog() {
                    return format!(
                        "Status: start agent, field={}, unsafe={}, name=\"{}\", prompt=\"{}\", init=\"{}\"",
                        dialog.focused_field.label(),
                        if dialog.start_config.skip_permissions {
                            "on"
                        } else {
                            "off"
                        },
                        dialog.start_config.name.replace('\n', "\\n"),
                        dialog.start_config.prompt.replace('\n', "\\n"),
                        dialog.start_config.init_command.replace('\n', "\\n"),
                    );
                }
                if self.session.interactive.is_some() {
                    if let Some(message) = &self.session.last_tmux_error {
                        return format!(
                            "Status: INSERT, unsafe={}, tmux error: {message}",
                            self.unsafe_label()
                        );
                    }
                    return format!("Status: INSERT, unsafe={}", self.unsafe_label());
                }
                if self.task_reorder_active() {
                    return "Status: task reorder".to_string();
                }

                match self.state.mode {
                    UiMode::List => format!("Status: list, unsafe={}", self.unsafe_label()),
                    UiMode::Preview => {
                        let preview_height = self
                            .preview_output_dimensions()
                            .map_or(1, |(_, height)| usize::from(height));
                        format!(
                            "Status: preview, autoscroll={}, offset={}, split={}%, unsafe={}",
                            if self.preview_auto_scroll_for_height(preview_height) {
                                "on"
                            } else {
                                "off"
                            },
                            self.preview_scroll_offset_for_height(preview_height),
                            self.sidebar_width_pct,
                            self.unsafe_label(),
                        )
                    }
                }
            }
        }
    }

    fn selected_task_label(&self) -> String {
        self.state
            .selected_task()
            .map(|task| task.name.clone())
            .unwrap_or_else(|| self.repo_name.clone())
    }

    fn selected_worktree_label(&self) -> String {
        self.state
            .selected_worktree()
            .map(|worktree| worktree.repository_name.clone())
            .unwrap_or_else(|| "none".to_string())
    }

    fn footer_context_line(&self) -> String {
        let task_label = self.selected_task_label();
        let worktree_label = self.selected_worktree_label();

        format!("task: {task_label} · worktree: {worktree_label}")
    }

    pub(super) fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = self.active_ui_theme();
        let state_label = self.footer_state_chip_label();
        let state_chip = format!("[{state_label}]");
        let context = self.footer_context_line();
        let base_style = Style::new()
            .bg(packed(theme.surface))
            .fg(packed(theme.text));
        StatusLine::new()
            .style(base_style)
            .separator("  ")
            .left(StatusItem::text(state_chip.as_str()))
            .left(StatusItem::text(context.as_str()))
            .right(StatusItem::text("[Keys]"))
            .right(StatusItem::key_hint("?", "help"))
            .right(StatusItem::key_hint("Ctrl+K", "palette"))
            .render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }
}
