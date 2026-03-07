use super::view_prelude::*;

impl GroveApp {
    fn create_dialog_project_picker_scroll_offset(
        selected_index: usize,
        total_count: usize,
        visible_count: usize,
    ) -> usize {
        if total_count <= visible_count {
            return 0;
        }
        let half = visible_count / 2;
        let max_offset = total_count.saturating_sub(visible_count);
        selected_index.saturating_sub(half).min(max_offset)
    }

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
    ) -> (FtLine, Vec<(CreateDialogTab, usize, usize)>) {
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

        let (mode_tabs_row, mode_tab_ranges) =
            Self::create_dialog_mode_tabs_row(content_width, theme, dialog.tab);
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Task setup (create)", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            mode_tabs_row,
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  [Mode] click tab or Alt+[/Alt+]",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ];
        if let Some(picker) = dialog.project_picker.as_ref() {
            let dialog_height = 25u16;
            let inner_height = usize::from(dialog_height.saturating_sub(2));
            let picker_hint = if dialog.tab == CreateDialogTab::Manual {
                "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Space toggle included repos, Enter select, Esc back, need a project first? close and press p, then Ctrl+A"
            } else {
                "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Enter select, Esc back, need a project first? close and press p, then Ctrl+A"
            };
            let hint_rows = modal_wrapped_hint_rows(content_width, theme, picker_hint);
            let list_header_count = 5usize;
            let footer_count = 1usize.saturating_add(hint_rows.len());
            let list_line_budget = inner_height
                .saturating_sub(lines.len().saturating_add(list_header_count + footer_count));
            let visible_projects = (list_line_budget / 2).max(1);
            let total_projects = self.projects.len();
            let filtered_projects = picker.filtered_project_indices.len();

            lines.push(modal_labeled_input_row(
                content_width,
                theme,
                "Filter",
                picker.filter.as_str(),
                "Type project name or path",
                true,
            ));
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("{filtered_projects} of {total_projects} projects").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]));
            lines.push(FtLine::raw(""));

            if picker.filtered_project_indices.is_empty() {
                let empty_label = if self.projects.is_empty() {
                    "No projects configured"
                } else {
                    "No matching projects"
                };
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(empty_label, content_width),
                    Style::new().fg(theme.subtext0),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "Need a project first? Close this dialog, press p, then Ctrl+A",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else {
                let scroll_offset = Self::create_dialog_project_picker_scroll_offset(
                    picker.selected_filtered_index,
                    picker.filtered_project_indices.len(),
                    visible_projects,
                );
                let visible_end = scroll_offset
                    .saturating_add(visible_projects)
                    .min(picker.filtered_project_indices.len());

                for filtered_index in scroll_offset..visible_end {
                    let Some(project_index) = picker.filtered_project_indices.get(filtered_index)
                    else {
                        continue;
                    };
                    let Some(project) = self.projects.get(*project_index) else {
                        continue;
                    };
                    let selected = filtered_index == picker.selected_filtered_index;
                    let marker = if selected { ">" } else { " " };
                    let row_bg = if selected { theme.surface1 } else { theme.base };
                    let name = if dialog.tab == CreateDialogTab::Manual {
                        let included = dialog.selected_repository_indices.contains(project_index);
                        let toggle = if included { "[x]" } else { "[ ]" };
                        pad_or_truncate_to_display_width(
                            format!("{marker} {toggle} {}", project.name).as_str(),
                            content_width,
                        )
                    } else {
                        pad_or_truncate_to_display_width(
                            format!("{marker} {}", project.name).as_str(),
                            content_width,
                        )
                    };
                    let path = pad_or_truncate_to_display_width(
                        format!("    {}", project.path.display()).as_str(),
                        content_width,
                    );
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        name,
                        Style::new()
                            .fg(if selected { theme.text } else { theme.subtext1 })
                            .bg(row_bg)
                            .bold(),
                    )]));
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        path,
                        Style::new().fg(theme.overlay0).bg(row_bg),
                    )]));
                }
            }

            lines.push(FtLine::raw(""));
            lines.extend(hint_rows);

            let body = FtText::from_lines(lines);
            render_modal_dialog(
                frame,
                area,
                body,
                ModalDialogSpec {
                    dialog_width,
                    dialog_height,
                    title: "Choose Project",
                    theme,
                    border_color: theme.mauve,
                    hit_id: HIT_ID_CREATE_DIALOG,
                },
            );
            return;
        }
        match dialog.tab {
            CreateDialogTab::Manual => {
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Task",
                    dialog.task_name.as_str(),
                    "feature-name",
                    focused(CreateDialogField::WorkspaceName),
                ));
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Project",
                    format!("{selected_project_label}  Enter browse").as_str(),
                    "Enter browse projects",
                    focused(CreateDialogField::Project),
                ));
                lines.push(modal_static_badged_row(
                    content_width,
                    theme,
                    "Included",
                    selected_projects_label.as_str(),
                    theme.overlay0,
                    theme.subtext0,
                ));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Defaults] base branch is implicit per project, configure in Project Defaults",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
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
            "Tab/C-n next, S-Tab/C-p prev, click mode tab or Alt+[/Alt+], Enter browse projects, base branch comes from Project Defaults or git, Enter create, Esc cancel"
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
