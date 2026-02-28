use super::*;

impl GroveApp {
    pub(super) fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog() else {
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
            let mut lines = vec![
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
            ];
            lines.extend(modal_wrapped_hint_rows(
                content_width,
                theme,
                "Tab/C-n next, S-Tab/C-p prev, Enter confirm, Esc back",
            ));
            let body = FtText::from_lines(lines);
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
        if let Some(defaults_dialog) = dialog.defaults_dialog.as_ref() {
            let dialog_height = 20u16;
            let focused = |field| defaults_dialog.focused_field == field;
            let project_label = self
                .projects
                .get(defaults_dialog.project_index)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "(missing project)".to_string());
            let project_path = self
                .projects
                .get(defaults_dialog.project_index)
                .map(|project| project.path.display().to_string())
                .unwrap_or_else(|| "(missing path)".to_string());
            let mut lines = vec![
                modal_static_badged_row(
                    content_width,
                    theme,
                    "Project",
                    project_label.as_str(),
                    theme.teal,
                    theme.text,
                ),
                modal_static_badged_row(
                    content_width,
                    theme,
                    "Path",
                    project_path.as_str(),
                    theme.overlay0,
                    theme.subtext0,
                ),
                FtLine::raw(""),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "BaseBranch",
                    defaults_dialog.base_branch.as_str(),
                    "Optional override (empty uses selected branch)",
                    focused(ProjectDefaultsDialogField::BaseBranch),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "InitCmd",
                    defaults_dialog.workspace_init_command.as_str(),
                    "Runs once per workspace start (agent/shell/git share)",
                    focused(ProjectDefaultsDialogField::WorkspaceInitCommand),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "ClaudeEnv",
                    defaults_dialog.claude_env.as_str(),
                    "KEY=VALUE; KEY2=VALUE",
                    focused(ProjectDefaultsDialogField::ClaudeEnv),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "CodexEnv",
                    defaults_dialog.codex_env.as_str(),
                    "KEY=VALUE; KEY2=VALUE",
                    focused(ProjectDefaultsDialogField::CodexEnv),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "OpenCodeEnv",
                    defaults_dialog.opencode_env.as_str(),
                    "KEY=VALUE; KEY2=VALUE",
                    focused(ProjectDefaultsDialogField::OpenCodeEnv),
                ),
                FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "Note: env changes apply on next agent start/restart",
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
                    focused(ProjectDefaultsDialogField::SaveButton),
                    focused(ProjectDefaultsDialogField::CancelButton),
                ),
            ];
            lines.extend(modal_wrapped_hint_rows(
                content_width,
                theme,
                "Tab/C-n next, S-Tab/C-p prev, Enter confirm, Esc back",
            ));
            let body = FtText::from_lines(lines);
            let content = OverlayModalContent {
                title: "Project Defaults",
                body,
                theme,
                border_color: theme.peach,
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
                .hit_id(HitId::new(HIT_ID_PROJECT_DEFAULTS_DIALOG))
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
        let reorder_active = dialog.reorder.is_some();
        if reorder_active {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Reorder mode active", content_width),
                Style::new().fg(theme.peach).bold(),
            )]));
        }
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
                let marker = if selected && reorder_active {
                    "↕"
                } else if selected {
                    ">"
                } else {
                    " "
                };
                let name_style = if selected {
                    Style::new().fg(theme.mauve).bold()
                } else {
                    Style::new().fg(theme.text)
                };
                let position = project_index.saturating_add(1);
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("{marker} {position:>2}. {}", project.name),
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
        let hints = if reorder_active {
            "Reorder mode, j/k or Up/Down or Tab/S-Tab/C-n/C-p move, Enter save, Esc cancel"
        } else {
            "Enter focus, Up/Down or Tab/S-Tab/C-n/C-p navigate, Ctrl+R reorder, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Esc close"
        };
        lines.extend(modal_wrapped_hint_rows(content_width, theme, hints));

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
}
