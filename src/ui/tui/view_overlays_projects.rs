use super::*;

impl GroveApp {
    pub(super) fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
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
                "Enter focus, Up/Down or Tab/S-Tab navigate, Ctrl+A add, Ctrl+X/Del remove, Esc close",
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
}
