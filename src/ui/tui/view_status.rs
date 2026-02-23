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
                        "Status: new workspace, field={}, agent={}, base_branch=\"{}\", setup_auto_run={}, unsafe={}, setup_commands=\"{}\", name=\"{}\", prompt=\"{}\", pre=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.agent.label(),
                        dialog.base_branch.replace('\n', "\\n"),
                        if dialog.auto_run_setup_commands {
                            "on"
                        } else {
                            "off"
                        },
                        if dialog.start_config.skip_permissions {
                            "on"
                        } else {
                            "off"
                        },
                        dialog.setup_commands.replace('\n', "\\n"),
                        dialog.workspace_name,
                        dialog.start_config.prompt.replace('\n', "\\n"),
                        dialog.start_config.pre_launch_command.replace('\n', "\\n"),
                    );
                }
                if let Some(dialog) = self.launch_dialog() {
                    return format!(
                        "Status: start agent, field={}, unsafe={}, prompt=\"{}\", pre=\"{}\"",
                        dialog.focused_field.label(),
                        if dialog.start_config.skip_permissions {
                            "on"
                        } else {
                            "off"
                        },
                        dialog.start_config.prompt.replace('\n', "\\n"),
                        dialog.start_config.pre_launch_command.replace('\n', "\\n"),
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

    fn selected_project_label(&self) -> String {
        let Some(workspace) = self.state.selected_workspace() else {
            return self.repo_name.clone();
        };

        if let Some(project_name) = workspace.project_name.as_ref() {
            return project_name.clone();
        }

        if let Some(project_path) = workspace.project_path.as_ref()
            && let Some(project) = self
                .projects
                .iter()
                .find(|project| refer_to_same_location(project.path.as_path(), project_path))
        {
            return project.name.clone();
        }

        self.repo_name.clone()
    }

    fn footer_context_line(&self) -> String {
        let project_label = self.selected_project_label();
        let workspace_label = self
            .state
            .selected_workspace()
            .map(Self::workspace_display_name)
            .unwrap_or_else(|| "none".to_string());

        format!("project: {project_label} · workspace: {workspace_label}")
    }

    fn footer_key_hints_line(&self) -> &'static str {
        "? help, Ctrl+K palette"
    }

    pub(super) fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let context = self.footer_context_line();
        let hints = self.footer_key_hints_line();
        let base_style = Style::new().bg(theme.mantle).fg(theme.text);
        let context_chip_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let key_chip_style = Style::new().bg(theme.surface0).fg(theme.mauve).bold();
        let key_style = Style::new().bg(theme.mantle).fg(theme.lavender).bold();
        let text_style = Style::new().bg(theme.mantle).fg(theme.subtext0);
        let sep_style = Style::new().bg(theme.mantle).fg(theme.overlay0);

        let left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Context ".to_string(), context_chip_style),
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(context, text_style),
        ];

        let mut right: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Keys ".to_string(), key_chip_style),
            FtSpan::styled(" ".to_string(), base_style),
        ];
        right.extend(keybind_hint_spans(hints, text_style, key_style, sep_style));

        let line = chrome_bar_line(usize::from(area.width), base_style, left, Vec::new(), right);
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }
}
