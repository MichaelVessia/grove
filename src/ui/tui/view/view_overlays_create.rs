use super::view_prelude::*;

impl GroveApp {
    pub(super) fn centered_modal_rect(area: Rect, width: u16, height: u16) -> Rect {
        let clamped_width = width.min(area.width);
        let clamped_height = height.min(area.height);
        let x = area
            .x
            .saturating_add(area.width.saturating_sub(clamped_width) / 2);
        let y = area
            .y
            .saturating_add(area.height.saturating_sub(clamped_height) / 2);
        Rect::new(x, y, clamped_width, clamped_height)
    }

    fn create_dialog_mode_tabs_row(
        content_width: usize,
        theme: UiTheme,
        selected_tab: CreateDialogTab,
    ) -> (FtLine<'static>, Vec<(CreateDialogTab, usize, usize)>) {
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut spans = Vec::new();
        let mut tab_ranges = Vec::new();
        let mut used_width = 0usize;
        for (index, tab) in [CreateDialogTab::Manual, CreateDialogTab::PullRequest]
            .iter()
            .copied()
            .enumerate()
        {
            if index > 0 {
                spans.push(FtSpan::styled(" ".to_string(), Style::new().bg(theme.base)));
                used_width = used_width.saturating_add(1);
            }
            let label = format!(" {} ", tab.label());
            let start = used_width;
            let width = text_display_width(label.as_str());
            used_width = used_width.saturating_add(width);
            tab_ranges.push((tab, start, width));
            spans.push(FtSpan::styled(
                label,
                if tab == selected_tab {
                    tab_active_style
                } else {
                    tab_inactive_style
                },
            ));
        }
        spans.push(FtSpan::styled(
            " ".repeat(content_width.saturating_sub(used_width)),
            Style::new().bg(theme.base),
        ));
        (FtLine::from_spans(spans), tab_ranges)
    }

    pub(super) fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 25u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let selected_project_label = self
            .projects
            .get(dialog.project_index)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "(missing project)".to_string());
        let selected_projects_label = if dialog.selected_repository_indices.is_empty() {
            "(none)".to_string()
        } else {
            dialog
                .selected_repository_indices
                .iter()
                .filter_map(|index| self.projects.get(*index))
                .map(|project| project.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        };
        let focused = |field| dialog.focused_field == field;
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };

        let (mode_tabs_row, mode_tab_ranges) =
            Self::create_dialog_mode_tabs_row(content_width, theme, dialog.tab);
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Task setup (create)"),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            mode_tabs_row,
            FtLine::from_spans(vec![FtSpan::styled(
                fit("  [Mode] click tab or Alt+[/Alt+]"),
                Style::new().fg(theme.overlay0),
            )]),
        ];
        if let Some(picker) = dialog.project_picker.as_ref() {
            let picker_hint = if dialog.tab == CreateDialogTab::Manual && !dialog.register_as_base {
                "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Space toggle included repos, Enter select, Esc back, need a project first? close and press p, then Ctrl+A"
            } else {
                "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Enter select, Esc back, need a project first? close and press p, then Ctrl+A"
            };
            let hint_rows = modal_wrapped_hint_rows(content_width, theme, picker_hint);
            let total_projects = self.projects.len();
            let filtered_projects = picker.filtered_project_indices.len();
            let dialog_area = Self::centered_modal_rect(area, dialog_width, dialog_height);
            let content_style = Style::new().fg(theme.text).bg(theme.base);
            Paragraph::new("")
                .style(content_style)
                .render(dialog_area, frame);

            let block = Block::new()
                .title("Choose Project")
                .title_alignment(BlockAlignment::Center)
                .borders(Borders::ALL)
                .style(content_style)
                .border_style(Style::new().fg(theme.mauve).bold());
            let inner = block.inner(dialog_area);
            block.render(dialog_area, frame);
            if inner.is_empty() {
                return;
            }

            let footer_height = u16::try_from(hint_rows.len().saturating_add(1))
                .unwrap_or(u16::MAX)
                .max(1);
            let rows = Flex::vertical()
                .constraints([
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Fixed(1),
                    Constraint::Min(1),
                    Constraint::Fixed(footer_height),
                ])
                .split(inner);

            for (row_area, line) in rows[..4].iter().zip(lines.iter()) {
                Paragraph::new(FtText::from_line(line.clone()))
                    .style(content_style)
                    .render(*row_area, frame);
            }

            Paragraph::new(FtText::from_line(modal_labeled_input_row(
                content_width,
                theme,
                "Filter",
                picker.filter.as_str(),
                "Type project name or path",
                true,
            )))
            .style(content_style)
            .render(rows[4], frame);

            Paragraph::new(FtText::from_line(FtLine::from_spans(vec![FtSpan::styled(
                fit(format!("{filtered_projects} of {total_projects} projects").as_str()),
                Style::new().fg(theme.overlay0),
            )])))
            .style(content_style)
            .render(rows[5], frame);

            Paragraph::new("")
                .style(content_style)
                .render(rows[6], frame);

            if picker.filtered_project_indices.is_empty() {
                let empty_label = if self.projects.is_empty() {
                    "No projects configured"
                } else {
                    "No matching projects"
                };
                Paragraph::new(FtText::from_lines(vec![
                    FtLine::from_spans(vec![FtSpan::styled(
                        fit(empty_label),
                        Style::new().fg(theme.subtext0),
                    )]),
                    FtLine::from_spans(vec![FtSpan::styled(
                        fit("Need a project first? Close this dialog, press p, then Ctrl+A"),
                        Style::new().fg(theme.overlay0),
                    )]),
                ]))
                .style(content_style)
                .render(rows[7], frame);
            } else {
                let items = picker
                    .filtered_project_indices
                    .iter()
                    .filter_map(|project_index| {
                        self.projects
                            .get(*project_index)
                            .map(|project| (project_index, project))
                    })
                    .map(|(project_index, project)| {
                        let label =
                            if dialog.tab == CreateDialogTab::Manual && !dialog.register_as_base {
                                let included =
                                    dialog.selected_repository_indices.contains(project_index);
                                let toggle = if included { "[x]" } else { "[ ]" };
                                format!("{toggle} {}  {}", project.name, project.path.display())
                            } else {
                                format!("{}  {}", project.name, project.path.display())
                            };
                        ListItem::new(label).style(Style::new().fg(theme.subtext1))
                    })
                    .collect::<Vec<_>>();
                let list = List::new(items)
                    .highlight_symbol("> ")
                    .highlight_style(Style::new().fg(theme.text).bg(theme.surface1).bold())
                    .style(content_style);
                let mut list_state = picker.project_list.clone();
                StatefulWidget::render(&list, rows[7], frame, &mut list_state);
            }

            Paragraph::new(FtText::from_lines({
                let mut footer_lines = vec![FtLine::raw("")];
                footer_lines.extend(hint_rows);
                footer_lines
            }))
            .style(content_style)
            .render(rows[8], frame);
            return;
        }
        match dialog.tab {
            CreateDialogTab::Manual => {
                if dialog.register_as_base {
                    lines.push(modal_static_badged_row(
                        content_width,
                        theme,
                        "Task",
                        &format!(
                            "auto: {}",
                            self.projects
                                .get(dialog.project_index)
                                .and_then(|project| project
                                    .path
                                    .file_name()
                                    .and_then(|name| name.to_str()))
                                .unwrap_or("(project)")
                        ),
                        theme.overlay0,
                        theme.subtext0,
                    ));
                } else {
                    lines.push(modal_labeled_input_row(
                        content_width,
                        theme,
                        "Task",
                        dialog.task_name.as_str(),
                        "feature-name",
                        focused(CreateDialogField::WorkspaceName),
                    ));
                }
                let base_toggle_label = if dialog.register_as_base {
                    "[x] register repo root as base task"
                } else {
                    "[ ] register repo root as base task"
                };
                lines.push(modal_focus_badged_row(
                    content_width,
                    theme,
                    "Base",
                    base_toggle_label,
                    focused(CreateDialogField::RegisterAsBase),
                    theme.overlay0,
                    theme.subtext0,
                ));
                if dialog.register_as_base {
                    lines.push(modal_labeled_input_row(
                        content_width,
                        theme,
                        "Project",
                        format!("{selected_project_label}  Enter browse").as_str(),
                        "Enter browse projects",
                        focused(CreateDialogField::Project),
                    ));
                } else {
                    lines.push(modal_focus_badged_row(
                        content_width,
                        theme,
                        "Included",
                        format!("{selected_projects_label}  Enter browse").as_str(),
                        focused(CreateDialogField::Project),
                        theme.blue,
                        theme.subtext0,
                    ));
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        fit("  [Defaults] base branch is implicit per project, configure in Project Defaults"),
                        Style::new().fg(theme.overlay0),
                    )]));
                }
            }
            CreateDialogTab::PullRequest => {
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Project",
                    format!("{selected_project_label}  Enter browse").as_str(),
                    "Enter browse projects",
                    focused(CreateDialogField::Project),
                ));
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "PR URL",
                    dialog.pr_url.as_str(),
                    "https://github.com/owner/repo/pull/123",
                    focused(CreateDialogField::PullRequestUrl),
                ));
                lines.push(modal_static_badged_row(
                    content_width,
                    theme,
                    "Name",
                    "auto: pr-<number>",
                    theme.overlay0,
                    theme.subtext0,
                ));
            }
        }
        if focused(CreateDialogField::Project)
            && (dialog.tab != CreateDialogTab::Manual || dialog.register_as_base)
            && let Some(project) = self.projects.get(dialog.project_index)
        {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                fit(format!("  [ProjectPath] {}", project.path.display()).as_str()),
                Style::new().fg(theme.overlay0),
            )]));
        }
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
        let hint_text = if dialog.tab == CreateDialogTab::Manual {
            "Tab/C-n next, S-Tab/C-p prev, click mode tab or Alt+[/Alt+], Space toggle base, Enter browse projects, base branch comes from Project Defaults or git, Enter create, Esc cancel"
        } else {
            "Tab/C-n next, S-Tab/C-p prev, click mode tab or Alt+[/Alt+], Enter browse projects, Enter create, Esc cancel"
        };
        lines.extend(modal_wrapped_hint_rows(content_width, theme, hint_text));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "New Task",
                theme,
                border_color: theme.mauve,
                hit_id: HIT_ID_CREATE_DIALOG,
            },
        );

        let modal_area = Self::centered_modal_rect(area, dialog_width, dialog_height);
        let inner = Block::new().borders(Borders::ALL).inner(modal_area);
        if inner.is_empty() {
            return;
        }

        let tab_row_y = inner.y.saturating_add(2);
        if tab_row_y >= inner.bottom() {
            return;
        }

        let tab_hit_height = if tab_row_y.saturating_add(1) < inner.bottom() {
            2
        } else {
            1
        };

        for (tab, start_col, width_cols) in mode_tab_ranges {
            let Some(start_u16) = u16::try_from(start_col).ok() else {
                continue;
            };
            let Some(width_u16) = u16::try_from(width_cols).ok() else {
                continue;
            };
            if width_u16 == 0 {
                continue;
            }

            let tab_x = inner.x.saturating_add(start_u16);
            if tab_x >= inner.right() {
                continue;
            }
            let visible_width = width_u16.min(inner.right().saturating_sub(tab_x));
            if visible_width == 0 {
                continue;
            }

            let _ = frame.register_hit(
                Rect::new(tab_x, tab_row_y, visible_width, tab_hit_height),
                HitId::new(HIT_ID_CREATE_DIALOG_TAB),
                FrameHitRegion::Content,
                encode_create_dialog_tab_hit_data(tab),
            );
        }
    }
}
