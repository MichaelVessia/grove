use super::view_prelude::*;

#[derive(Clone)]
struct ProjectListModalContent<'a> {
    dialog: &'a ProjectDialogState,
    projects: &'a [ProjectConfig],
    theme: UiTheme,
}

impl Widget for ProjectListModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new().bg(self.theme.base).fg(self.theme.text);
        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title("Projects")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(self.theme.teal).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Min(1),
                Constraint::Fixed(2),
            ])
            .split(inner);

        let filter_input = self
            .dialog
            .filter_input
            .clone()
            .with_style(Style::new().fg(self.theme.text).bg(self.theme.surface0))
            .with_placeholder("Type project name or path")
            .with_placeholder_style(Style::new().fg(self.theme.overlay0))
            .with_cursor_style(Style::new().fg(self.theme.base).bg(self.theme.mauve))
            .with_selection_style(Style::new().fg(self.theme.text).bg(self.theme.surface1));
        Widget::render(&filter_input, rows[0], frame);
        if filter_input.focused() {
            frame.set_cursor(Some(filter_input.cursor_position(rows[0])));
            frame.set_cursor_visible(true);
        }

        Paragraph::new(format!("{} projects", self.projects.len()))
            .style(Style::new().fg(self.theme.overlay0))
            .render(rows[1], frame);

        if self.dialog.filtered_project_indices.is_empty() {
            Paragraph::new("No matches")
                .style(Style::new().fg(self.theme.subtext0))
                .render(rows[2], frame);
        } else {
            let items = self
                .dialog
                .filtered_project_indices
                .iter()
                .filter_map(|project_index| {
                    self.projects
                        .get(*project_index)
                        .map(|project| (project_index, project))
                })
                .map(|(project_index, project)| {
                    let label = format!(
                        "{:>2}. {}  {}",
                        project_index.saturating_add(1),
                        project.name,
                        project.path.display()
                    );
                    ListItem::new(label).style(Style::new().fg(self.theme.subtext1))
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .highlight_symbol("> ")
                .highlight_style(
                    Style::new()
                        .fg(self.theme.text)
                        .bg(self.theme.surface1)
                        .bold(),
                )
                .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG_LIST));
            let mut list_state = self.dialog.project_list.clone();
            StatefulWidget::render(&list, rows[2], frame, &mut list_state);
        }

        Paragraph::new("Enter focus, Up/Down or Tab/S-Tab/C-n/C-p navigate, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Esc close")
            .style(Style::new().fg(self.theme.overlay0))
            .render(rows[3], frame);
    }
}

impl GroveApp {
    pub(super) fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog() else {
            return;
        };
        if area.width < 44 || area.height < 14 {
            return;
        }

        let theme = self.active_ui_theme();
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
                    add_dialog.name_input.value(),
                    "Optional, defaults to directory name",
                    focused(ProjectAddDialogField::Name),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Path",
                    add_dialog.path_input.value(),
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
                    defaults_dialog.base_branch_input.value(),
                    "Optional override (empty uses selected branch)",
                    focused(ProjectDefaultsDialogField::BaseBranch),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "InitCmd",
                    defaults_dialog.workspace_init_command_input.value(),
                    "Runs once per workspace start (agent/shell/git share)",
                    focused(ProjectDefaultsDialogField::WorkspaceInitCommand),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "ClaudeEnv",
                    defaults_dialog.claude_env_input.value(),
                    "KEY=VALUE; KEY2=VALUE",
                    focused(ProjectDefaultsDialogField::ClaudeEnv),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "CodexEnv",
                    defaults_dialog.codex_env_input.value(),
                    "KEY=VALUE; KEY2=VALUE",
                    focused(ProjectDefaultsDialogField::CodexEnv),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "OpenCodeEnv",
                    defaults_dialog.opencode_env_input.value(),
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

        let dialog_height = area.height.min(20);
        let content = ProjectListModalContent {
            dialog,
            projects: self.projects.as_slice(),
            theme,
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
            .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG))
            .render(area, frame);
    }
}
