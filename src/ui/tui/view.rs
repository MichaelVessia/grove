use super::*;

impl GroveApp {
    pub(super) fn render_model(&self, frame: &mut Frame) {
        let view_started_at = Instant::now();
        frame.set_cursor(None);
        frame.set_cursor_visible(false);
        frame.enable_hit_testing();
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let layout = Self::view_layout_for_size(
            frame.buffer.width(),
            frame.buffer.height(),
            self.sidebar_width_pct,
        );

        self.render_header(frame, layout.header);
        self.render_sidebar(frame, layout.sidebar);
        self.render_divider(frame, layout.divider);
        self.render_preview_pane(frame, layout.preview);
        self.render_status_line(frame, layout.status);
        self.render_create_dialog_overlay(frame, area);
        self.render_edit_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        self.render_delete_dialog_overlay(frame, area);
        self.render_settings_dialog_overlay(frame, area);
        self.render_project_dialog_overlay(frame, area);
        self.render_keybind_help_overlay(frame, area);
        self.render_command_palette_overlay(frame, area);
        self.render_toasts(frame, area);
        let draw_completed_at = Instant::now();
        self.last_hit_grid.replace(frame.hit_grid.clone());
        let frame_log_started_at = Instant::now();
        self.log_frame_render(frame);
        let view_completed_at = Instant::now();
        self.event_log.log(
            LogEvent::new("frame", "timing")
                .with_data(
                    "draw_ms",
                    Value::from(Self::duration_millis(
                        draw_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data(
                    "frame_log_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(frame_log_started_at),
                    )),
                )
                .with_data(
                    "view_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data("degradation", Value::from(frame.degradation.as_str()))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
    }

    fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }

    fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 11 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 11u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let unsafe_focused = focused(LaunchDialogField::Unsafe);
        let unsafe_state = if dialog.skip_permissions {
            "on, bypass approvals and sandbox"
        } else {
            "off, standard safety checks"
        };
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Launch profile", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Prompt",
                dialog.prompt.as_str(),
                "Describe initial task for the agent",
                focused(LaunchDialogField::Prompt),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "PreLaunch",
                dialog.pre_launch_command.as_str(),
                "Optional command to run before launch",
                focused(LaunchDialogField::PreLaunchCommand),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Unsafe",
                unsafe_state,
                unsafe_focused,
                theme.peach,
                if dialog.skip_permissions {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Start",
                "Cancel",
                start_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, Space toggle unsafe, Enter start, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);
        let content = OverlayModalContent {
            title: "Start Agent",
            body,
            theme,
            border_color: theme.mauve,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_LAUNCH_DIALOG))
            .render(area, frame);
    }

    fn render_delete_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.delete_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 16u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let warning_lines = if dialog.is_missing {
            (
                "  • Directory already removed",
                "  • Clean up git worktree metadata",
            )
        } else {
            (
                "  • Remove the working directory",
                "  • Uncommitted changes will be lost",
            )
        };
        let cleanup_focused = focused(DeleteDialogField::DeleteLocalBranch);
        let cleanup_state = if dialog.delete_local_branch {
            format!("enabled, remove '{}' branch locally", dialog.branch)
        } else {
            "disabled, keep local branch".to_string()
        };
        let delete_focused = focused(DeleteDialogField::DeleteButton);
        let cancel_focused = focused(DeleteDialogField::CancelButton);
        let delete_hint = pad_or_truncate_to_display_width(
            "Tab move, Space toggle branch cleanup, Enter or D delete, Esc cancel",
            content_width,
        );
        let path = dialog.path.display().to_string();
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Deletion plan", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                theme.blue,
                theme.overlay0,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  [Risk] Changes are destructive", content_width),
                Style::new().fg(theme.peach).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.0,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.1,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "BranchCleanup",
                cleanup_state.as_str(),
                cleanup_focused,
                theme.peach,
                if dialog.delete_local_branch {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Delete",
                "Cancel",
                delete_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                delete_hint,
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Delete Worktree?",
            body,
            theme,
            border_color: theme.red,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_DELETE_DIALOG))
            .render(area, frame);
    }

    fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };
        if area.width < 40 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 12u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let current = dialog.multiplexer.label();
        let multiplexer_focused = focused(SettingsDialogField::Multiplexer);
        let save_focused = focused(SettingsDialogField::SaveButton);
        let cancel_focused = focused(SettingsDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Global settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Multiplexer",
                current,
                multiplexer_focused,
                theme.blue,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  h/l, Left/Right, Space cycles", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Switching requires restarting running workspaces",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                save_focused,
                cancel_focused,
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Saved to ~/.config/grove/config.toml",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Settings",
            body,
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_SETTINGS_DIALOG))
            .render(area, frame);
    }

    fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog.as_ref() else {
            return;
        };
        if area.width < 44 || area.height < 14 {
            return;
        }

        let theme = ui_theme();
        let dialog_width = area.width.saturating_sub(8).min(96);
        let content_width = usize::from(dialog_width.saturating_sub(2));

        if let Some(add_dialog) = dialog.add_dialog.as_ref() {
            let dialog_height = 12u16;
            let focused = |field| add_dialog.focused_field == field;
            let body = FtText::from_lines(vec![
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Name",
                    add_dialog.name.as_str(),
                    "Optional, defaults to directory name",
                    focused(ProjectAddDialogField::Name),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Path",
                    add_dialog.path.as_str(),
                    "Absolute path or ~/path to repo root",
                    focused(ProjectAddDialogField::Path),
                ),
                FtLine::raw(""),
                modal_actions_row(
                    content_width,
                    theme,
                    "Add",
                    "Cancel",
                    focused(ProjectAddDialogField::AddButton),
                    focused(ProjectAddDialogField::CancelButton),
                ),
                FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "Tab move, Enter confirm, Esc back",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]),
            ]);
            let content = OverlayModalContent {
                title: "Add Project",
                body,
                theme,
                border_color: theme.mauve,
            };

            Modal::new(content)
                .size(
                    ModalSizeConstraints::new()
                        .min_width(dialog_width)
                        .max_width(dialog_width)
                        .min_height(dialog_height)
                        .max_height(dialog_height),
                )
                .backdrop(BackdropConfig::new(theme.crust, 0.55))
                .hit_id(HitId::new(HIT_ID_PROJECT_ADD_DIALOG))
                .render(area, frame);
            return;
        }

        let mut lines = Vec::new();
        lines.push(modal_labeled_input_row(
            content_width,
            theme,
            "Filter",
            dialog.filter.as_str(),
            "Type project name or path",
            true,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                format!("{} projects", self.projects.len()).as_str(),
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        lines.push(FtLine::raw(""));

        if dialog.filtered_project_indices.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No matches",
                Style::new().fg(theme.subtext0),
            )]));
        } else {
            for (visible_index, project_index) in
                dialog.filtered_project_indices.iter().take(8).enumerate()
            {
                let Some(project) = self.projects.get(*project_index) else {
                    continue;
                };
                let selected = visible_index == dialog.selected_filtered_index;
                let marker = if selected { ">" } else { " " };
                let name_style = if selected {
                    Style::new().fg(theme.mauve).bold()
                } else {
                    Style::new().fg(theme.text)
                };
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("{marker} {}", project.name),
                    name_style,
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        format!("  {}", project.path.display()).as_str(),
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Enter focus, Up/Down or Tab/S-Tab navigate, Ctrl+A add, Esc close",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));

        let content = OverlayModalContent {
            title: "Projects",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(16)
                    .max_height(20),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG))
            .render(area, frame);
    }

    fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.command_palette.is_visible() {
            return;
        }

        self.command_palette.render(area, frame);
    }

    fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.keybind_help_open {
            return;
        }
        if area.width < 56 || area.height < 18 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(108);
        let dialog_height = area.height.saturating_sub(6).clamp(18, 26);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));

        let lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Global]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  ? help, q quit, Tab/h/l switch pane, Enter open/attach, Esc list pane",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  n new, e edit, p projects, D delete, S settings, ! unsafe toggle",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  Ctrl+K command palette", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[List]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  j/k or Up/Down move selection", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Preview]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Agent tab: [/] tab, j/k or Up/Down scroll, PgUp/PgDn page, G bottom, s start, x stop",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Git tab: [/] tab, Enter attach lazygit",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Interactive]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  type sends input to agent", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Esc Esc or Ctrl+\\ exit, Alt+C copy, Alt+V paste",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Create: Tab/S-Tab fields, j/k or C-n/C-p move, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Edit:   Tab/S-Tab fields, h/l or Space toggle agent, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Start:  Tab/S-Tab fields, Space toggle unsafe, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Delete: Tab/S-Tab fields, j/k move, Space toggle, Enter/D confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Close help: Esc, Enter, or ?", content_width),
                Style::new().fg(theme.lavender).bold(),
            )]),
        ];

        let content = OverlayModalContent {
            title: "Keybind Help",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.blue,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_KEYBIND_HELP_DIALOG))
            .render(area, frame);
    }

    fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 16u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let selected_project_label = self
            .projects
            .get(dialog.project_index)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "(missing project)".to_string());

        let focused = |field| dialog.focused_field == field;
        let selected_agent = dialog.agent;
        let selected_agent_style = Style::new()
            .fg(theme.text)
            .bg(if focused(CreateDialogField::Agent) {
                theme.surface1
            } else {
                theme.base
            })
            .bold();
        let unselected_agent_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let selected_dropdown_style = Style::new().fg(theme.text).bg(theme.surface1).bold();
        let unselected_dropdown_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let agent_row = |agent: AgentType| {
            let is_selected = selected_agent == agent;
            let prefix = if is_selected { "▸" } else { " " };
            let line = pad_or_truncate_to_display_width(
                format!("{} [Agent] {}", prefix, agent.label()).as_str(),
                content_width,
            );
            if is_selected {
                FtLine::from_spans(vec![FtSpan::styled(line, selected_agent_style)])
            } else {
                FtLine::from_spans(vec![FtSpan::styled(line, unselected_agent_style)])
            }
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace setup", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                "feature-name",
                focused(CreateDialogField::WorkspaceName),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "Project",
                selected_project_label.as_str(),
                "j/k or C-n/C-p select",
                focused(CreateDialogField::Project),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "BaseBranch",
                dialog.base_branch.as_str(),
                "current branch (fallback: main/master)",
                focused(CreateDialogField::BaseBranch),
            ),
        ];
        if focused(CreateDialogField::Project)
            && let Some(project) = self.projects.get(dialog.project_index)
        {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [ProjectPath] {}", project.path.display()).as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]));
        }
        if focused(CreateDialogField::BaseBranch) {
            if self.create_branch_all.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] Loading branches...",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else if self.create_branch_filtered.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] No matching branches",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else {
                let max_dropdown = 4usize;
                for (index, branch) in self.create_branch_filtered.iter().enumerate() {
                    if index >= max_dropdown {
                        break;
                    }
                    let is_selected = index == self.create_branch_index;
                    let prefix = if is_selected { "▸" } else { " " };
                    let line = pad_or_truncate_to_display_width(
                        format!("{prefix} [Branches] {branch}").as_str(),
                        content_width,
                    );
                    if is_selected {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            selected_dropdown_style,
                        )]));
                    } else {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            unselected_dropdown_style,
                        )]));
                    }
                }
                if self.create_branch_filtered.len() > max_dropdown {
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        pad_or_truncate_to_display_width(
                            format!(
                                "  [Branches] ... and {} more",
                                self.create_branch_filtered.len() - max_dropdown
                            )
                            .as_str(),
                            content_width,
                        ),
                        Style::new().fg(theme.overlay0),
                    )]));
                }
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(agent_row(AgentType::Claude));
        lines.push(agent_row(AgentType::Codex));
        lines.push(FtLine::raw(""));
        let create_focused = focused(CreateDialogField::CreateButton);
        let cancel_focused = focused(CreateDialogField::CancelButton);
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Create",
            "Cancel",
            create_focused,
            cancel_focused,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Tab move, j/k or C-n/C-p adjust project/branch, Enter create, Esc cancel",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        let content = OverlayModalContent {
            title: "New Workspace",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.mauve,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_CREATE_DIALOG))
            .render(area, frame);
    }

    fn render_edit_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.edit_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(10).min(80);
        let dialog_height = 13u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let path = dialog.workspace_path.display().to_string();
        let running_note = if dialog.was_running {
            "Running now, restart agent to apply change"
        } else {
            "Agent change applies on next agent start"
        };

        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                theme.blue,
                theme.overlay0,
            ),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Agent",
                dialog.agent.label(),
                focused(EditDialogField::Agent),
                theme.peach,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [Note] {running_note}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                focused(EditDialogField::SaveButton),
                focused(EditDialogField::CancelButton),
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, h/l or Space toggle agent, Enter save, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Edit Workspace",
            body,
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_EDIT_DIALOG))
            .render(area, frame);
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
                        "Status: new workspace, field={}, agent={}, base_branch=\"{}\", name=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.agent.label(),
                        dialog.base_branch.replace('\n', "\\n"),
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

    fn keybind_hints_line(&self) -> &'static str {
        if self.command_palette.is_visible() {
            return "Type to search, Up/Down choose, Enter run, Esc close";
        }
        if self.keybind_help_open {
            return "Esc/? close help";
        }
        if self.create_dialog.is_some() {
            return "Tab/S-Tab field, j/k or C-n/C-p move, h/l buttons, Enter select/create, Esc cancel";
        }
        if self.edit_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle agent, Enter save/select, Esc cancel";
        }
        if self.launch_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle unsafe, Enter select/start, Esc cancel";
        }
        if self.delete_dialog.is_some() {
            return "Tab/S-Tab field, j/k move, Space toggle branch delete, Enter select/delete, D confirm, Esc cancel";
        }
        if self.settings_dialog.is_some() {
            return "Tab/S-Tab field, j/k or h/l change, Enter save/select, Esc cancel";
        }
        if self.project_dialog.is_some() {
            return "Type filter, Up/Down or Tab/S-Tab navigate, Enter focus project, Ctrl+A add, Esc close";
        }
        if self.interactive.is_some() {
            return "Esc Esc / Ctrl+\\ exit, Alt+C copy, Alt+V paste";
        }
        if self.preview_agent_tab_is_focused() {
            return "[ prev tab, ] next tab, j/k scroll, PgUp/PgDn, G bottom, h/l pane, Enter open, n new, e edit, p projects, s start, x stop, D delete, S settings, Ctrl+K palette, ? help, q quit";
        }
        if self.preview_git_tab_is_focused() {
            return "[ prev tab, ] next tab, h/l pane, Enter attach lazygit, n new, e edit, p projects, D delete, S settings, Ctrl+K palette, ? help, q quit";
        }

        "j/k move, h/l pane, Enter open, n new, e edit, p projects, D delete, S settings, Ctrl+K palette, ? help, q quit"
    }

    fn pane_border_style(&self, focused: bool) -> Style {
        let theme = ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    fn workspace_agent_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.peach,
            AgentType::Codex => theme.text,
        }
    }

    fn activity_effect_secondary_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.text,
            AgentType::Codex => theme.overlay0,
        }
    }

    fn activity_effect_gradient(&self, agent: AgentType) -> ColorGradient {
        let primary = self.workspace_agent_color(agent);
        let secondary = self.activity_effect_secondary_color(agent);
        ColorGradient::new(vec![(0.0, primary), (0.5, secondary), (1.0, primary)])
    }

    fn activity_effect_time(&self) -> f64 {
        self.fast_animation_frame as f64 * (FAST_ANIMATION_INTERVAL_MS as f64 / 1000.0)
    }

    fn render_activity_effect_label(
        &self,
        label: &str,
        agent: AgentType,
        area: Rect,
        frame: &mut Frame,
    ) {
        if area.is_empty() || label.is_empty() {
            return;
        }

        let primary = self.workspace_agent_color(agent);
        StyledText::new(label)
            .bold()
            .base_color(primary)
            .effect(TextEffect::AnimatedGradient {
                gradient: self.activity_effect_gradient(agent),
                speed: 1.8,
            })
            .time(self.activity_effect_time())
            .render(area, frame);
    }

    pub(super) fn relative_age_label(&self, unix_secs: Option<i64>) -> String {
        let Some(unix_secs) = unix_secs else {
            return String::new();
        };
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        let Some(now_secs) = now_secs else {
            return String::new();
        };
        let age_secs = now_secs.saturating_sub(unix_secs).max(0);
        if age_secs < 60 {
            return "now".to_string();
        }
        if age_secs < 3_600 {
            return format!("{}m", age_secs / 60);
        }
        if age_secs < 86_400 {
            return format!("{}h", age_secs / 3_600);
        }
        format!("{}d", age_secs / 86_400)
    }

    fn workspace_display_name(workspace: &Workspace) -> String {
        if workspace.is_main {
            "base".to_string()
        } else {
            workspace.name.clone()
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let base_style = Style::new().bg(theme.crust).fg(theme.text);
        let left_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let repo_style = Style::new().bg(theme.mantle).fg(theme.subtext0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Grove ".to_string(), left_style),
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(format!(" {} ", self.repo_name), repo_style),
        ];
        if self.command_palette.is_visible() {
            left.push(FtSpan::styled(
                " [Palette] ".to_string(),
                Style::new().bg(theme.surface1).fg(theme.mauve).bold(),
            ));
        }

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_HEADER));
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Workspaces")
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(
                self.state.focus == PaneFocus::WorkspaceList && !self.modal_open(),
            ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_WORKSPACE_LIST));

        if inner.is_empty() {
            return;
        }

        let theme = ui_theme();
        let mut lines: Vec<FtLine> = Vec::new();
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let max_lines = usize::from(inner.height);
        if self.projects.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No projects configured",
                Style::new().fg(theme.subtext0),
            )]));
            lines.push(FtLine::raw(""));
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "Press 'p' to add a project",
                Style::new().fg(theme.text).bold(),
            )]));
        } else if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "Discovery error",
                    Style::new().fg(theme.red).bold(),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    message.as_str(),
                    Style::new().fg(theme.peach),
                )]));
            }
        } else {
            for (project_index, project) in self.projects.iter().enumerate() {
                if lines.len() >= max_lines {
                    break;
                }
                if project_index > 0 && lines.len() < max_lines {
                    lines.push(FtLine::raw(""));
                }
                if lines.len() >= max_lines {
                    break;
                }
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("▾ {}", project.name),
                    Style::new().fg(theme.overlay0).bold(),
                )]));

                let project_workspaces: Vec<(usize, &Workspace)> = self
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
                    .collect();

                if project_workspaces.is_empty() {
                    if lines.len() < max_lines {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            "  (no workspaces)",
                            Style::new().fg(theme.subtext0),
                        )]));
                    }
                    continue;
                }

                for (idx, workspace) in project_workspaces {
                    if lines
                        .len()
                        .saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT))
                        > max_lines
                    {
                        break;
                    }

                    let row_y = inner
                        .y
                        .saturating_add(u16::try_from(lines.len()).unwrap_or(u16::MAX));
                    let is_selected = idx == self.state.selected_index;
                    let is_working = self.status_is_visually_working(
                        Some(workspace.path.as_path()),
                        workspace.status,
                        is_selected,
                    );
                    let selected = if is_selected { "▸" } else { " " };
                    let row_background = if is_selected {
                        if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                            Some(theme.surface1)
                        } else {
                            Some(theme.surface0)
                        }
                    } else {
                        None
                    };

                    let mut primary_style = Style::new().fg(theme.text);
                    let mut secondary_style = Style::new().fg(theme.subtext0);
                    if let Some(bg) = row_background {
                        primary_style = primary_style.bg(bg);
                        secondary_style = secondary_style.bg(bg);
                    }
                    if is_selected {
                        primary_style = primary_style.bold();
                    }

                    let workspace_label_style = if is_working {
                        primary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold()
                    } else {
                        primary_style
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    let show_branch = workspace.branch != workspace_name;
                    let branch_text = if show_branch {
                        format!(" · {}", workspace.branch)
                    } else {
                        String::new()
                    };
                    let agent_separator = " · ";
                    let mut row_spans = vec![
                        FtSpan::styled(format!("{selected} "), primary_style),
                        FtSpan::styled(workspace_name.clone(), workspace_label_style),
                    ];
                    if !branch_text.is_empty() {
                        row_spans.push(FtSpan::styled(branch_text.clone(), secondary_style));
                    }
                    row_spans.push(FtSpan::styled(agent_separator, secondary_style));
                    row_spans.push(FtSpan::styled(
                        workspace.agent.label().to_string(),
                        secondary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold(),
                    ));
                    if workspace.is_orphaned {
                        row_spans.push(FtSpan::styled(
                            " · session ended",
                            secondary_style.fg(theme.peach),
                        ));
                    }
                    lines.push(FtLine::from_spans(row_spans));

                    if is_working {
                        let primary_label_x = inner.x.saturating_add(
                            u16::try_from(text_display_width("▸ ")).unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace_name.clone(),
                            workspace.agent,
                            primary_label_x,
                            row_y,
                        ));
                        let agent_prefix =
                            format!("{workspace_name}{branch_text}{agent_separator}");
                        let secondary_label_x = inner.x.saturating_add(
                            u16::try_from(
                                text_display_width("▸ ")
                                    .saturating_add(text_display_width(&agent_prefix)),
                            )
                            .unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace.agent.label().to_string(),
                            workspace.agent,
                            secondary_label_x,
                            row_y,
                        ));
                    }

                    if let Ok(data) = u64::try_from(idx) {
                        let row_height =
                            WORKSPACE_ITEM_HEIGHT.min(inner.bottom().saturating_sub(row_y));
                        let row_rect = Rect::new(inner.x, row_y, inner.width, row_height);
                        let _ = frame.register_hit(
                            row_rect,
                            HitId::new(HIT_ID_WORKSPACE_ROW),
                            FrameHitRegion::Content,
                            data,
                        );
                    }
                }
            }
        }

        Paragraph::new(FtText::from_lines(lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
    }

    fn render_divider(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let glyph = if self.divider_drag_active {
            "█"
        } else {
            "│"
        };
        let divider = std::iter::repeat_n(glyph, usize::from(area.height))
            .collect::<Vec<&str>>()
            .join("\n");
        let theme = ui_theme();
        Paragraph::new(divider)
            .style(Style::new().fg(if self.divider_drag_active {
                theme.blue
            } else {
                theme.overlay0
            }))
            .render(area, frame);
        let _ = frame.register_hit_region(
            Self::divider_hit_area(area, frame.width()),
            HitId::new(HIT_ID_DIVIDER),
        );
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let title = "Preview";
        let block =
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_style(self.pane_border_style(
                    self.state.focus == PaneFocus::Preview && !self.modal_open(),
                ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_PREVIEW));

        if inner.is_empty() {
            return;
        }

        let selected_workspace = self.state.selected_workspace();
        let selected_agent = selected_workspace.map(|workspace| workspace.agent);
        let allow_cursor_overlay =
            self.preview_tab == PreviewTab::Git || selected_agent != Some(AgentType::Codex);
        let theme = ui_theme();
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let selected_workspace_header = selected_workspace.map(|workspace| {
            let workspace_name = Self::workspace_display_name(workspace);
            let is_working = self.status_is_visually_working(
                Some(workspace.path.as_path()),
                workspace.status,
                true,
            );
            let branch_label = if workspace.branch != workspace_name {
                Some(workspace.branch.clone())
            } else {
                None
            };
            let age_label = self.relative_age_label(workspace.last_activity_unix_secs);
            (
                workspace_name,
                branch_label,
                age_label,
                is_working,
                workspace.agent,
                workspace.is_orphaned,
            )
        });

        let metadata_rows = usize::from(PREVIEW_METADATA_ROWS);
        let preview_height = usize::from(inner.height)
            .saturating_sub(metadata_rows)
            .max(1);

        let mut text_lines = vec![if let Some((
            name_label,
            branch_label,
            age_label,
            is_working,
            agent,
            is_orphaned,
        )) = selected_workspace_header.as_ref()
        {
            let mut spans = vec![FtSpan::styled(
                name_label.clone(),
                if *is_working {
                    Style::new().fg(self.workspace_agent_color(*agent)).bold()
                } else {
                    Style::new().fg(theme.text).bold()
                },
            )];
            if let Some(branch_label) = branch_label {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    branch_label.clone(),
                    Style::new().fg(theme.subtext0),
                ));
            }
            spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            spans.push(FtSpan::styled(
                agent.label().to_string(),
                Style::new().fg(self.workspace_agent_color(*agent)).bold(),
            ));
            if !age_label.is_empty() {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    age_label.clone(),
                    Style::new().fg(theme.overlay0),
                ));
            }
            if *is_orphaned {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    "session ended",
                    Style::new().fg(theme.peach),
                ));
            }
            FtLine::from_spans(spans)
        } else {
            FtLine::from_spans(vec![FtSpan::styled(
                "none selected",
                Style::new().fg(theme.subtext0),
            )])
        }];
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut tab_spans = Vec::new();
        for (index, tab) in [PreviewTab::Agent, PreviewTab::Git]
            .iter()
            .copied()
            .enumerate()
        {
            if index > 0 {
                tab_spans.push(FtSpan::raw(" ".to_string()));
            }
            let style = if tab == self.preview_tab {
                tab_active_style
            } else {
                tab_inactive_style
            };
            tab_spans.push(FtSpan::styled(format!(" {} ", tab.label()), style));
        }
        if let Some(workspace) = selected_workspace {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                workspace.path.display().to_string(),
                Style::new().fg(theme.overlay0),
            ));
        } else {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                "no workspace",
                Style::new().fg(theme.overlay0),
            ));
        }
        text_lines.push(FtLine::from_spans(tab_spans));
        if let Some((name_label, branch_label, _, true, agent, _)) =
            selected_workspace_header.as_ref()
        {
            animated_labels.push((name_label.clone(), *agent, inner.x, inner.y));
            let branch_prefix = branch_label
                .as_ref()
                .map_or(String::new(), |branch| format!(" · {branch}"));
            let agent_prefix = format!("{name_label}{branch_prefix} · ");
            animated_labels.push((
                agent.label().to_string(),
                *agent,
                inner.x.saturating_add(
                    u16::try_from(text_display_width(&agent_prefix)).unwrap_or(u16::MAX),
                ),
                inner.y,
            ));
        }

        let visible_range = self.preview_visible_range_for_height(preview_height);
        let visible_start = visible_range.0;
        let visible_end = visible_range.1;
        let visible_plain_lines = self.preview_plain_lines_range(visible_start, visible_end);
        match self.preview_tab {
            PreviewTab::Agent => {
                let mut visible_render_lines = if self.preview.render_lines.is_empty() {
                    Vec::new()
                } else {
                    let render_start = visible_start.min(self.preview.render_lines.len());
                    let render_end = visible_end.min(self.preview.render_lines.len());
                    if render_start < render_end {
                        self.preview.render_lines[render_start..render_end].to_vec()
                    } else {
                        Vec::new()
                    }
                };
                if visible_render_lines.len() < visible_plain_lines.len() {
                    visible_render_lines.extend(
                        visible_plain_lines[visible_render_lines.len()..]
                            .iter()
                            .cloned(),
                    );
                }
                if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
                    visible_render_lines = visible_plain_lines.clone();
                }
                if allow_cursor_overlay {
                    self.apply_interactive_cursor_overlay_render(
                        &visible_plain_lines,
                        &mut visible_render_lines,
                        preview_height,
                    );
                }

                if visible_render_lines.is_empty() {
                    text_lines.push(FtLine::raw("(no preview output)"));
                } else {
                    text_lines.extend(
                        visible_render_lines
                            .iter()
                            .map(|line| ansi_line_to_styled_line(line)),
                    );
                }
            }
            PreviewTab::Git => {
                let mut visible_render_lines = if self.preview.render_lines.is_empty() {
                    Vec::new()
                } else {
                    let render_start = visible_start.min(self.preview.render_lines.len());
                    let render_end = visible_end.min(self.preview.render_lines.len());
                    if render_start < render_end {
                        self.preview.render_lines[render_start..render_end].to_vec()
                    } else {
                        Vec::new()
                    }
                };
                if visible_render_lines.len() < visible_plain_lines.len() {
                    visible_render_lines.extend(
                        visible_plain_lines[visible_render_lines.len()..]
                            .iter()
                            .cloned(),
                    );
                }
                if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
                    visible_render_lines = visible_plain_lines.clone();
                }
                if allow_cursor_overlay {
                    self.apply_interactive_cursor_overlay_render(
                        &visible_plain_lines,
                        &mut visible_render_lines,
                        preview_height,
                    );
                }

                if visible_render_lines.is_empty() {
                    let fallback = if let Some(workspace) = selected_workspace {
                        let session_name = git_session_name_for_workspace(workspace);
                        if self.lazygit_failed_sessions.contains(&session_name) {
                            "(lazygit launch failed)"
                        } else if self.lazygit_ready_sessions.contains(&session_name) {
                            "(no lazygit output yet)"
                        } else {
                            "(launching lazygit...)"
                        }
                    } else {
                        "(no workspace selected)"
                    };
                    text_lines.push(FtLine::raw(fallback.to_string()));
                } else {
                    text_lines.extend(
                        visible_render_lines
                            .iter()
                            .map(|line| ansi_line_to_styled_line(line)),
                    );
                }
            }
        }

        Paragraph::new(FtText::from_lines(text_lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
        self.apply_preview_selection_highlight_cells(
            frame,
            inner,
            &visible_plain_lines,
            visible_start,
        );
    }

    fn render_status_line(&self, frame: &mut Frame, area: Rect) {
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
        left.extend(keybind_hint_spans(hints, text_style, key_style, sep_style));

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

    #[cfg(test)]
    pub(super) fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab/h/l focus, Enter preview, n create, e edit, s/x start-stop, D delete, S settings, ? help, ! unsafe, Esc list, mouse)"
                .to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        "▸"
                    } else {
                        " "
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    lines.push(format!(
                        "{} {} | {} | {} | {}{}",
                        selected,
                        workspace_name,
                        workspace.branch,
                        workspace.agent.label(),
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        if let Some(dialog) = &self.launch_dialog {
            lines.push(String::new());
            lines.push("Start Agent Dialog".to_string());
            lines.push(format!("Field: {}", dialog.focused_field.label()));
            lines.push(format!(
                "Prompt: {}",
                if dialog.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.prompt.clone()
                }
            ));
            lines.push(format!(
                "Pre-launch command: {}",
                if dialog.pre_launch_command.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.pre_launch_command.clone()
                }
            ));
            lines.push(format!(
                "Unsafe launch: {}",
                if dialog.skip_permissions { "on" } else { "off" }
            ));
        }
        if let Some(dialog) = &self.delete_dialog {
            lines.push(String::new());
            lines.push("Delete Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.branch));
            lines.push(format!(
                "Delete local branch: {}",
                if dialog.delete_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    Self::workspace_display_name(workspace),
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let mut visible_lines = self.preview.visible_lines(preview_height);
        self.apply_interactive_cursor_overlay(&mut visible_lines, preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }

    pub(super) fn view_layout_for_size(
        width: u16,
        height: u16,
        sidebar_width_pct: u16,
    ) -> ViewLayout {
        let area = Rect::from_size(width, height);
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(HEADER_HEIGHT),
                Constraint::Fill,
                Constraint::Fixed(STATUS_HEIGHT),
            ])
            .split(area);

        let sidebar_width = ((u32::from(rows[1].width) * u32::from(sidebar_width_pct)) / 100)
            .try_into()
            .unwrap_or(rows[1].width);
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Fixed(sidebar_width),
                Constraint::Fixed(DIVIDER_WIDTH),
                Constraint::Fill,
            ])
            .split(rows[1]);

        ViewLayout {
            header: rows[0],
            sidebar: cols[0],
            divider: cols[1],
            preview: cols[2],
            status: rows[2],
        }
    }

    fn effective_viewport_size(&self) -> (u16, u16) {
        let from_hit_grid = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .map(|grid| (grid.width(), grid.height()));
        let (width, height) = from_hit_grid.unwrap_or((self.viewport_width, self.viewport_height));
        (width.max(1), height.max(1))
    }

    pub(super) fn view_layout(&self) -> ViewLayout {
        let (width, height) = self.effective_viewport_size();
        Self::view_layout_for_size(width, height, self.sidebar_width_pct)
    }

    fn divider_hit_area(divider: Rect, viewport_width: u16) -> Rect {
        let left = divider.x.saturating_sub(1);
        let right = divider.right().saturating_add(1).min(viewport_width);
        Rect::new(left, divider.y, right.saturating_sub(left), divider.height)
    }

    pub(super) fn hit_region_for_point(&self, x: u16, y: u16) -> (HitRegion, Option<u64>) {
        if let Some((id, _region, data)) = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
        {
            let mapped = match id.id() {
                HIT_ID_HEADER => HitRegion::Header,
                HIT_ID_STATUS => HitRegion::StatusLine,
                HIT_ID_DIVIDER => HitRegion::Divider,
                HIT_ID_PREVIEW => HitRegion::Preview,
                HIT_ID_WORKSPACE_LIST | HIT_ID_WORKSPACE_ROW => HitRegion::WorkspaceList,
                HIT_ID_CREATE_DIALOG
                | HIT_ID_LAUNCH_DIALOG
                | HIT_ID_DELETE_DIALOG
                | HIT_ID_KEYBIND_HELP_DIALOG => HitRegion::Outside,
                _ => HitRegion::Outside,
            };
            let row_data = if id.id() == HIT_ID_WORKSPACE_ROW {
                Some(data)
            } else {
                None
            };
            return (mapped, row_data);
        }

        let (viewport_width, viewport_height) = self.effective_viewport_size();
        let layout = self.view_layout();

        if x >= viewport_width || y >= viewport_height {
            return (HitRegion::Outside, None);
        }
        if y < layout.header.bottom() {
            return (HitRegion::Header, None);
        }
        if y >= layout.status.y {
            return (HitRegion::StatusLine, None);
        }

        let divider_area = Self::divider_hit_area(layout.divider, viewport_width);
        if x >= divider_area.x && x < divider_area.right() {
            return (HitRegion::Divider, None);
        }
        if x >= layout.sidebar.x && x < layout.sidebar.right() {
            return (HitRegion::WorkspaceList, None);
        }
        if x >= layout.preview.x && x < layout.preview.right() {
            return (HitRegion::Preview, None);
        }

        (HitRegion::Outside, None)
    }

    pub(super) fn interactive_cursor_target(
        &self,
        preview_height: usize,
    ) -> Option<(usize, usize, bool)> {
        let interactive = self.interactive.as_ref()?;
        if self.preview.lines.is_empty() {
            return None;
        }

        let pane_height = usize::from(interactive.pane_height.max(1));
        let cursor_row = usize::from(interactive.cursor_row);
        if cursor_row >= pane_height {
            return None;
        }

        let preview_len = self.preview.lines.len();
        let pane_start = preview_len.saturating_sub(pane_height);
        let cursor_line = pane_start.saturating_add(cursor_row);
        if cursor_line >= preview_len {
            return None;
        }

        let end = preview_len.saturating_sub(self.preview.offset);
        let start = end.saturating_sub(preview_height);
        if cursor_line < start || cursor_line >= end {
            return None;
        }

        let visible_index = cursor_line - start;
        Some((
            visible_index,
            usize::from(interactive.cursor_col),
            interactive.cursor_visible,
        ))
    }

    #[cfg(test)]
    fn apply_interactive_cursor_overlay(
        &self,
        visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(line) = visible_lines.get_mut(visible_index) else {
            return;
        };

        *line = render_cursor_overlay(line, cursor_col, cursor_visible);
    }

    fn apply_interactive_cursor_overlay_render(
        &self,
        plain_visible_lines: &[String],
        render_visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(plain_line) = plain_visible_lines.get(visible_index) else {
            return;
        };
        let Some(render_line) = render_visible_lines.get_mut(visible_index) else {
            return;
        };

        *render_line =
            render_cursor_overlay_ansi(render_line, plain_line, cursor_col, cursor_visible);
    }

    pub(super) fn clear_preview_selection(&mut self) {
        self.preview_selection.clear();
    }

    fn preview_visible_range_for_height(&self, preview_height: usize) -> (usize, usize) {
        if preview_height == 0 {
            return (0, 0);
        }

        let max_offset = self.preview.max_scroll_offset(preview_height);
        let clamped_offset = self.preview.offset.min(max_offset);
        let end = self.preview.lines.len().saturating_sub(clamped_offset);
        let start = end.saturating_sub(preview_height);
        (start, end)
    }

    fn preview_content_viewport(&self) -> Option<PreviewContentViewport> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }
        let inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if inner.is_empty() {
            return None;
        }

        let preview_height = usize::from(inner.height)
            .saturating_sub(usize::from(PREVIEW_METADATA_ROWS))
            .max(1);
        let (visible_start, visible_end) = self.preview_visible_range_for_height(preview_height);

        Some(PreviewContentViewport {
            output_x: inner.x,
            output_y: inner.y.saturating_add(PREVIEW_METADATA_ROWS),
            visible_start,
            visible_end,
        })
    }

    pub(super) fn preview_text_point_at(&self, x: u16, y: u16) -> Option<TextSelectionPoint> {
        if self.preview_tab != PreviewTab::Agent {
            return None;
        }

        let viewport = self.preview_content_viewport()?;
        if y < viewport.output_y {
            return None;
        }

        let visible_row = usize::from(y - viewport.output_y);
        let visible_count = viewport.visible_end.saturating_sub(viewport.visible_start);
        if visible_row >= visible_count {
            return None;
        }

        let line_idx = viewport.visible_start.saturating_add(visible_row);
        let line = self.preview_plain_line(line_idx)?;
        let line_width = line_visual_width(&line);
        if x < viewport.output_x {
            return Some(TextSelectionPoint {
                line: line_idx,
                col: 0,
            });
        }

        let relative_x = usize::from(x - viewport.output_x);
        let col = if line_width == 0 {
            0
        } else {
            relative_x.min(line_width.saturating_sub(1))
        };

        Some(TextSelectionPoint {
            line: line_idx,
            col,
        })
    }

    fn preview_plain_line(&self, line_idx: usize) -> Option<String> {
        if let Some(line) = self.preview.render_lines.get(line_idx) {
            return Some(ansi_line_to_plain_text(line));
        }

        self.preview.lines.get(line_idx).cloned()
    }

    fn preview_plain_lines_range(&self, start: usize, end: usize) -> Vec<String> {
        if start >= end {
            return Vec::new();
        }

        let mut lines = Vec::with_capacity(end.saturating_sub(start));
        for line_idx in start..end {
            if let Some(line) = self.preview_plain_line(line_idx) {
                lines.push(line);
                continue;
            }
            break;
        }

        lines
    }

    pub(super) fn add_selection_point_snapshot_fields(
        &self,
        mut event: LogEvent,
        key_prefix: &str,
        point: TextSelectionPoint,
    ) -> LogEvent {
        let raw_line = self.preview.lines.get(point.line).cloned();
        let clean_line = self.preview_plain_line(point.line);
        let render_line = self.preview.render_lines.get(point.line).cloned();

        if let Some(line) = raw_line {
            event = event.with_data(
                format!("{key_prefix}line_raw_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        if let Some(line) = clean_line {
            event = event
                .with_data(
                    format!("{key_prefix}line_clean_preview"),
                    Value::from(truncate_for_log(&line, 120)),
                )
                .with_data(
                    format!("{key_prefix}line_visual_width"),
                    Value::from(u64::try_from(line_visual_width(&line)).unwrap_or(u64::MAX)),
                )
                .with_data(
                    format!("{key_prefix}line_context"),
                    Value::from(truncate_for_log(
                        &visual_substring(
                            &line,
                            point.col.saturating_sub(16),
                            Some(point.col.saturating_add(16)),
                        ),
                        120,
                    )),
                );

            if let Some((grapheme, start_col, end_col)) = visual_grapheme_at(&line, point.col) {
                event = event
                    .with_data(
                        format!("{key_prefix}grapheme"),
                        Value::from(truncate_for_log(&grapheme, 16)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_start_col"),
                        Value::from(u64::try_from(start_col).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_end_col"),
                        Value::from(u64::try_from(end_col).unwrap_or(u64::MAX)),
                    );
            }
        }

        if let Some(line) = render_line {
            event = event.with_data(
                format!("{key_prefix}line_render_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        event
    }

    fn log_preview_drag_started(&self, x: u16, y: u16, point: Option<TextSelectionPoint>) {
        let mut event = LogEvent::new("selection", "preview_drag_started")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("mode", Value::from(Self::mode_name(self.state.mode)))
            .with_data("focus", Value::from(Self::focus_name(self.state.focus)))
            .with_data(
                "preview_offset",
                Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
            );

        if let Some(viewport) = self.preview_content_viewport() {
            event = event
                .with_data("output_x", Value::from(viewport.output_x))
                .with_data("output_y", Value::from(viewport.output_y))
                .with_data(
                    "visible_start",
                    Value::from(u64::try_from(viewport.visible_start).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "visible_end",
                    Value::from(u64::try_from(viewport.visible_end).unwrap_or(u64::MAX)),
                );
        }

        if let Some(point) = point {
            event = event
                .with_data(
                    "line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "", point);
            if let Some(line) = self.preview_plain_line(point.line) {
                event = event.with_data("line_preview", Value::from(truncate_for_log(&line, 120)));
            }
            if let Some(render_line) = self.preview.render_lines.get(point.line) {
                event = event.with_data(
                    "render_line_preview",
                    Value::from(truncate_for_log(render_line, 120)),
                );
            }
        }

        self.event_log.log(event);
    }

    fn log_preview_drag_finished(&self, x: u16, y: u16, point: Option<TextSelectionPoint>) {
        let mut event = LogEvent::new("selection", "preview_drag_finished")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data(
                "has_selection",
                Value::from(self.preview_selection.has_selection()),
            )
            .with_data("interactive", Value::from(self.interactive.is_some()));

        if let Some(point) = point {
            event = event
                .with_data(
                    "release_line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "release_col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "release_", point);
        }

        if let Some(anchor) = self.preview_selection.anchor {
            event = event
                .with_data(
                    "anchor_line",
                    Value::from(u64::try_from(anchor.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "anchor_col",
                    Value::from(u64::try_from(anchor.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "anchor_", anchor);
        }

        if let Some(start) = self.preview_selection.start {
            event = event
                .with_data(
                    "start_line",
                    Value::from(u64::try_from(start.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "start_col",
                    Value::from(u64::try_from(start.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "start_", start);
        }
        if let Some(end) = self.preview_selection.end {
            event = event
                .with_data(
                    "end_line",
                    Value::from(u64::try_from(end.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "end_col",
                    Value::from(u64::try_from(end.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "end_", end);
        }

        if let Some(lines) = self.selected_preview_text_lines() {
            let text = lines.join("\n");
            event = event
                .with_data(
                    "selected_line_count",
                    Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "selected_char_count",
                    Value::from(u64::try_from(text.chars().count()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "selected_preview",
                    Value::from(truncate_for_log(&text, 240)),
                );
        }

        self.event_log.log(event);
    }

    pub(super) fn prepare_preview_selection_drag(&mut self, x: u16, y: u16) {
        let point = self.preview_text_point_at(x, y);
        self.log_preview_drag_started(x, y, point);
        if let Some(point) = point {
            self.preview_selection.prepare_drag(point);
            return;
        }

        self.clear_preview_selection();
    }

    pub(super) fn update_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        let Some(point) = self.preview_text_point_at(x, y) else {
            return;
        };
        self.preview_selection.handle_drag(point);
    }

    pub(super) fn finish_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        let release_point = self.preview_text_point_at(x, y);
        if !self.preview_selection.has_selection()
            && let Some(point) = release_point
        {
            self.preview_selection.handle_drag(point);
        }
        self.log_preview_drag_finished(x, y, release_point);
        self.preview_selection.finish_drag();
    }

    fn apply_preview_selection_highlight_cells(
        &self,
        frame: &mut Frame,
        inner: Rect,
        visible_plain_lines: &[String],
        visible_start: usize,
    ) {
        if !self.preview_selection.has_selection() {
            return;
        }

        let selection_bg = ui_theme().surface1;
        let output_y = inner.y.saturating_add(PREVIEW_METADATA_ROWS);
        for (offset, line) in visible_plain_lines.iter().enumerate() {
            let line_idx = visible_start.saturating_add(offset);
            let Some((start_col, end_col)) = self.preview_selection.line_selection_cols(line_idx)
            else {
                continue;
            };

            let line_width = line_visual_width(line);
            if line_width == 0 {
                continue;
            }

            let start = start_col.min(line_width.saturating_sub(1));
            let end = end_col
                .unwrap_or_else(|| line_width.saturating_sub(1))
                .min(line_width.saturating_sub(1));
            if end < start {
                continue;
            }

            let y = output_y.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            if y >= inner.bottom() {
                break;
            }

            let x_start = inner
                .x
                .saturating_add(u16::try_from(start).unwrap_or(u16::MAX));
            let x_end = inner
                .x
                .saturating_add(u16::try_from(end).unwrap_or(u16::MAX))
                .min(inner.right().saturating_sub(1));
            if x_start > x_end {
                continue;
            }

            for x in x_start..=x_end {
                if let Some(cell) = frame.buffer.get_mut(x, y) {
                    cell.bg = selection_bg;
                }
            }
        }
    }

    pub(super) fn selected_preview_text_lines(&self) -> Option<Vec<String>> {
        let (start, end) = self.preview_selection.bounds()?;
        let source_len = self
            .preview
            .lines
            .len()
            .max(self.preview.render_lines.len());
        if source_len == 0 {
            return None;
        }

        let start_line = start.line.min(source_len.saturating_sub(1));
        let end_line = end.line.min(source_len.saturating_sub(1));
        if end_line < start_line {
            return None;
        }

        let mut lines = self.preview_plain_lines_range(start_line, end_line.saturating_add(1));
        if lines.is_empty() {
            return None;
        }

        if lines.len() == 1 {
            lines[0] = visual_substring(&lines[0], start.col, Some(end.col));
            return Some(lines);
        }

        lines[0] = visual_substring(&lines[0], start.col, None);
        let last_idx = lines.len().saturating_sub(1);
        lines[last_idx] = visual_substring(&lines[last_idx], 0, Some(end.col));

        Some(lines)
    }

    fn visible_preview_output_lines(&self) -> Vec<String> {
        let Some((_, output_height)) = self.preview_output_dimensions() else {
            return Vec::new();
        };
        let (visible_start, visible_end) =
            self.preview_visible_range_for_height(usize::from(output_height));
        self.preview_plain_lines_range(visible_start, visible_end)
    }

    pub(super) fn copy_interactive_selection_or_visible(&mut self) {
        let selected_lines = self.selected_preview_text_lines();
        let copied_from_selection = selected_lines.is_some();
        let mut lines = selected_lines.unwrap_or_else(|| self.visible_preview_output_lines());
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            self.show_toast("No output to copy", true);
            return;
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            self.show_toast("No output to copy", true);
            return;
        }
        let text = lines.join("\n");
        self.event_log.log(
            LogEvent::new("selection", "interactive_copy_payload")
                .with_data("from_selection", Value::from(copied_from_selection))
                .with_data(
                    "line_count",
                    Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "char_count",
                    Value::from(u64::try_from(text.chars().count()).unwrap_or(u64::MAX)),
                )
                .with_data("preview", Value::from(truncate_for_log(&text, 240))),
        );
        self.copied_text = Some(text.clone());
        match self.clipboard.write_text(&text) {
            Ok(()) => {
                self.last_tmux_error = None;
                self.show_toast(format!("Copied {} line(s)", lines.len()), false);
            }
            Err(error) => {
                self.last_tmux_error = Some(format!("clipboard write failed: {error}"));
                self.show_toast(format!("Copy failed: {error}"), true);
            }
        }
        self.clear_preview_selection();
    }
}
