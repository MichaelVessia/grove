use super::*;

impl GroveApp {
    fn create_dialog_mode_tabs_row(
        content_width: usize,
        theme: UiTheme,
        selected_tab: CreateDialogTab,
    ) -> FtLine {
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut spans = Vec::new();
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
            used_width = used_width.saturating_add(text_display_width(label.as_str()));
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
        FtLine::from_spans(spans)
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
                pad_or_truncate_to_display_width("Workspace setup (create)", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            Self::create_dialog_mode_tabs_row(content_width, theme, dialog.tab),
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
            "Tab/C-n next, S-Tab/C-p prev, Alt+[/Alt+] switch mode, j/k adjust project/branch, Space toggles unsafe, Enter create, Esc cancel"
        } else {
            "Tab/C-n next, S-Tab/C-p prev, Alt+[/Alt+] switch mode, j/k adjust project or agent, Space toggles unsafe, Enter create, Esc cancel"
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
    }
}
