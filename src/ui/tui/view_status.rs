use super::*;

impl GroveApp {
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
            if matches!(toast.config.style_variant, ToastStyle::Error) {
                return format!("Status: error: {}", toast.content.message);
            }
            return format!("Status: {}", toast.content.message);
        }

        match &self.discovery_state {
            DiscoveryState::Error(message) => format!("Status: discovery error ({message})"),
            DiscoveryState::Empty => "Status: no worktrees found".to_string(),
            DiscoveryState::Ready => {
                if let Some(dialog) = &self.create_dialog {
                    return format!(
                        "Status: new workspace, field={}, agent={}, base_branch=\"{}\", setup_auto_run={}, setup_commands=\"{}\", name=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.agent.label(),
                        dialog.base_branch.replace('\n', "\\n"),
                        if dialog.auto_run_setup_commands {
                            "on"
                        } else {
                            "off"
                        },
                        dialog.setup_commands.replace('\n', "\\n"),
                        dialog.workspace_name
                    );
                }
                if let Some(dialog) = &self.launch_dialog {
                    return format!(
                        "Status: start agent, field={}, unsafe={}, prompt=\"{}\", pre=\"{}\"",
                        dialog.focused_field.label(),
                        if dialog.skip_permissions { "on" } else { "off" },
                        dialog.prompt.replace('\n', "\\n"),
                        dialog.pre_launch_command.replace('\n', "\\n"),
                    );
                }
                if self.interactive.is_some() {
                    if let Some(message) = &self.last_tmux_error {
                        return format!(
                            "Status: INSERT, unsafe={}, tmux error: {message}",
                            self.unsafe_label()
                        );
                    }
                    return format!("Status: INSERT, unsafe={}", self.unsafe_label());
                }

                match self.state.mode {
                    UiMode::List => format!("Status: list, unsafe={}", self.unsafe_label()),
                    UiMode::Preview => format!(
                        "Status: preview, autoscroll={}, offset={}, split={}%, unsafe={}",
                        if self.preview.auto_scroll {
                            "on"
                        } else {
                            "off"
                        },
                        self.preview.offset,
                        self.sidebar_width_pct,
                        self.unsafe_label(),
                    ),
                }
            }
        }
    }

    pub(super) fn keybind_help_line(&self, context: HelpHintContext) -> String {
        UiCommand::help_hints_for(context)
            .iter()
            .filter_map(|command| command.help_hint_label(context))
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn status_hints_line(&self, context: StatusHintContext) -> String {
        UiCommand::status_hints_for(context)
            .iter()
            .filter_map(|command| command.status_hint_label(context))
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn keybind_hints_line(&self) -> String {
        if self.command_palette.is_visible() {
            return "Type to search, Up/Down choose, Enter run, Esc close".to_string();
        }
        if self.keybind_help_open {
            return "Esc/? close help".to_string();
        }
        if self.create_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, j/k adjust controls, ';' separates setup commands, Space toggles auto-run, h/l buttons, Enter select/create, Esc cancel"
                .to_string();
        }
        if self.edit_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, type/backspace base branch, h/l buttons, Space toggle agent, Enter save/select, Esc cancel"
                .to_string();
        }
        if self.launch_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, h/l buttons, Space toggle unsafe, Enter select/start, Esc cancel"
                .to_string();
        }
        if self.delete_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, j/k move, Space toggle branch delete, Enter select/delete, D confirm, Esc cancel"
                .to_string();
        }
        if self.merge_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, j/k move, Space toggle cleanup, Enter select/merge, m confirm, Esc cancel"
                .to_string();
        }
        if self.update_from_base_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, h/l buttons, Enter select/update, u confirm, Esc cancel"
                .to_string();
        }
        if self.settings_dialog.is_some() {
            return "Tab/S-Tab or C-n/C-p field, j/k or h/l change, Enter save/select, Esc cancel"
                .to_string();
        }
        if self.project_dialog.is_some() {
            if self
                .project_dialog
                .as_ref()
                .and_then(|dialog| dialog.defaults_dialog.as_ref())
                .is_some()
            {
                return "Tab/S-Tab or C-n/C-p field, type/backspace edit defaults, Space toggle auto-run, Enter save/select, Esc close"
                    .to_string();
            }
            return "Type filter, Up/Down or Tab/S-Tab/C-n/C-p navigate, Enter focus project, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Esc close"
                .to_string();
        }
        if self.interactive.is_some() {
            return "Reserved: Ctrl+K palette, Esc Esc/Ctrl+\\ exit, Alt+J/K browse, Alt+C copy, Alt+V paste"
                .to_string();
        }
        if self.preview_agent_tab_is_focused() {
            return self.status_hints_line(StatusHintContext::PreviewAgent);
        }
        if self.preview_git_tab_is_focused() {
            return self.status_hints_line(StatusHintContext::PreviewGit);
        }

        self.status_hints_line(StatusHintContext::List)
    }

    pub(super) fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let hints = self.keybind_hints_line();
        let base_style = Style::new().bg(theme.mantle).fg(theme.text);
        let chip_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let key_style = Style::new().bg(theme.mantle).fg(theme.lavender).bold();
        let text_style = Style::new().bg(theme.mantle).fg(theme.subtext0);
        let sep_style = Style::new().bg(theme.mantle).fg(theme.overlay0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Keys ".to_string(), chip_style),
            FtSpan::styled(" ".to_string(), base_style),
        ];
        left.extend(keybind_hint_spans(
            hints.as_str(),
            text_style,
            key_style,
            sep_style,
        ));

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }
}
