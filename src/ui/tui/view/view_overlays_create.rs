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

        let (mode_tabs_row, mode_tab_ranges) =
            Self::create_dialog_mode_tabs_row(content_width, theme, dialog.tab);
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace setup (create)", content_width),
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
        match dialog.tab {
            CreateDialogTab::Manual => {
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Name",
                    dialog.workspace_name.as_str(),
                    "feature-name",
                    focused(CreateDialogField::WorkspaceName),
                ));
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Project",
                    selected_project_label.as_str(),
                    "j/k or C-n/C-p select",
                    focused(CreateDialogField::Project),
                ));
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "BaseBranch",
                    dialog.base_branch.as_str(),
                    "current branch (fallback: main/master)",
                    focused(CreateDialogField::BaseBranch),
                ));
            }
            CreateDialogTab::PullRequest => {
                lines.push(modal_labeled_input_row(
                    content_width,
                    theme,
                    "Project",
                    selected_project_label.as_str(),
                    "j/k or C-n/C-p select",
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
        if dialog.tab == CreateDialogTab::Manual && focused(CreateDialogField::BaseBranch) {
            if self.dialogs.create_branch_all.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] Loading branches...",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else if self.dialogs.create_branch_filtered.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] No matching branches",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else {
                let max_dropdown = 4usize;
                for (index, branch) in self.dialogs.create_branch_filtered.iter().enumerate() {
                    if index >= max_dropdown {
                        break;
                    }
                    let is_selected = index == self.dialogs.create_branch_index;
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
                if self.dialogs.create_branch_filtered.len() > max_dropdown {
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        pad_or_truncate_to_display_width(
                            format!(
                                "  [Branches] ... and {} more",
                                self.dialogs.create_branch_filtered.len() - max_dropdown
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
        for agent in AgentType::all() {
            lines.push(agent_row(*agent));
        }
        lines.push(FtLine::raw(""));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width("Agent startup (every start)", content_width),
            Style::new().fg(theme.overlay0),
        )]));
        let start_config_rows =
            modal_start_agent_config_rows(content_width, theme, &dialog.start_config, |field| {
                focused(CreateDialogField::StartConfig(field))
            });
        lines.push(start_config_rows[0].clone());
        lines.push(start_config_rows[1].clone());
        lines.push(start_config_rows[2].clone());
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
            "Tab/C-n next, S-Tab/C-p prev, click mode tab or Alt+[/Alt+], j/k adjust project/branch, Space toggles unsafe, Enter create, Esc cancel"
        } else {
            "Tab/C-n next, S-Tab/C-p prev, click mode tab or Alt+[/Alt+], j/k adjust project or agent, Space toggles unsafe, Enter create, Esc cancel"
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
                title: "New Workspace",
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
