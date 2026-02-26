use super::*;

impl GroveApp {
    fn pull_request_status_icon(status: crate::domain::PullRequestStatus) -> &'static str {
        match status {
            crate::domain::PullRequestStatus::Open => "",
            crate::domain::PullRequestStatus::Merged => "",
            crate::domain::PullRequestStatus::Closed => "",
        }
    }

    fn pull_request_status_style(
        status: crate::domain::PullRequestStatus,
        secondary_style: Style,
        theme: UiTheme,
    ) -> Style {
        match status {
            crate::domain::PullRequestStatus::Open => secondary_style.fg(theme.teal).bold(),
            crate::domain::PullRequestStatus::Merged => secondary_style.fg(theme.mauve).bold(),
            crate::domain::PullRequestStatus::Closed => secondary_style.fg(theme.red).bold(),
        }
    }

    pub(super) fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
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
        let mut animated_labels: Vec<(String, AgentType, u16, u16, u16)> = Vec::new();

        if self.projects.is_empty() {
            Paragraph::new(FtText::from_lines(vec![
                FtLine::from_spans(vec![FtSpan::styled(
                    "No projects configured",
                    Style::new().fg(theme.subtext0),
                )]),
                FtLine::raw(""),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press 'p' to add a project",
                    Style::new().fg(theme.text).bold(),
                )]),
            ]))
            .render(inner, frame);
            return;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                Paragraph::new(FtText::from_lines(vec![
                    FtLine::from_spans(vec![FtSpan::styled(
                        "Discovery error",
                        Style::new().fg(theme.red).bold(),
                    )]),
                    FtLine::from_spans(vec![FtSpan::styled(
                        message.as_str(),
                        Style::new().fg(theme.peach),
                    )]),
                ]))
                .render(inner, frame);
            }
            return;
        }

        let mut current_y = inner.y;
        for (project_index, project) in self.projects.iter().enumerate() {
            if current_y >= inner.bottom() {
                break;
            }
            if project_index > 0 {
                current_y = current_y.saturating_add(1);
            }
            if current_y >= inner.bottom() {
                break;
            }

            Paragraph::new(FtText::from_lines(vec![FtLine::from_spans(vec![
                FtSpan::styled(
                    format!("▾ {}", project.name),
                    Style::new().fg(theme.overlay0).bold(),
                ),
            ])]))
            .render(Rect::new(inner.x, current_y, inner.width, 1), frame);
            current_y = current_y.saturating_add(1);
            if current_y >= inner.bottom() {
                break;
            }

            let project_workspaces: Vec<(usize, &Workspace)> = self
                .state
                .workspaces
                .iter()
                .enumerate()
                .filter(|(_, workspace)| {
                    workspace.project_path.as_deref() == Some(project.path.as_path())
                })
                .collect();

            if project_workspaces.is_empty() {
                Paragraph::new(FtText::from_lines(vec![FtLine::from_spans(vec![
                    FtSpan::styled("  (no workspaces)", Style::new().fg(theme.subtext0)),
                ])]))
                .render(Rect::new(inner.x, current_y, inner.width, 1), frame);
                current_y = current_y.saturating_add(1);
                continue;
            }

            for (idx, workspace) in project_workspaces {
                if current_y.saturating_add(WORKSPACE_ITEM_HEIGHT) > inner.bottom() {
                    break;
                }

                let row_rect = Rect::new(inner.x, current_y, inner.width, WORKSPACE_ITEM_HEIGHT);
                let is_selected = idx == self.state.selected_index;
                let is_working = self.status_is_visually_working(
                    Some(workspace.path.as_path()),
                    workspace.status,
                    is_selected,
                );
                let (attention_symbol, attention_color) = if is_working {
                    (" ", theme.overlay0)
                } else {
                    self.workspace_attention_indicator(workspace.path.as_path())
                        .unwrap_or((" ", theme.overlay0))
                };
                let row_background = if is_selected {
                    if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                        Some(theme.surface1)
                    } else {
                        Some(theme.surface0)
                    }
                } else {
                    None
                };

                let mut border_style = if is_selected {
                    Style::new().fg(theme.blue)
                } else {
                    Style::new().fg(theme.surface1)
                };
                if let Some(bg) = row_background {
                    border_style = border_style.bg(bg);
                }
                if is_selected {
                    border_style = border_style.bold();
                }

                let row_block = Block::new()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_style(border_style)
                    .style(row_background.map_or_else(Style::new, |bg| Style::new().bg(bg)))
                    .padding(ftui::core::geometry::Sides::new(0, 1, 1, 1));
                let row_inner = row_block.inner(row_rect);
                row_block.render(row_rect, frame);

                if row_inner.width == 0 || row_inner.height < 2 {
                    current_y = current_y.saturating_add(WORKSPACE_ITEM_HEIGHT);
                    continue;
                }

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

                let line1_prefix = "   ".to_string();
                let line1_attention_gap = " ";
                let line1_prefix_width = text_display_width(&line1_prefix)
                    .saturating_add(text_display_width(attention_symbol))
                    .saturating_add(text_display_width(line1_attention_gap));
                let mut line1_spans = vec![
                    FtSpan::styled(line1_prefix, primary_style),
                    FtSpan::styled(
                        attention_symbol.to_string(),
                        primary_style.fg(attention_color).bold(),
                    ),
                    FtSpan::styled(line1_attention_gap.to_string(), primary_style),
                    FtSpan::styled(workspace_name.clone(), workspace_label_style),
                ];
                if !branch_text.is_empty() {
                    line1_spans.push(FtSpan::styled(branch_text, secondary_style));
                }

                let line2_prefix = "     ";
                let line2_prefix_width = text_display_width(line2_prefix);
                let mut line2_spans =
                    vec![FtSpan::styled(line2_prefix.to_string(), secondary_style)];
                let agent_label = workspace.agent.label().to_string();
                line2_spans.push(FtSpan::styled(
                    agent_label.clone(),
                    secondary_style
                        .fg(self.workspace_agent_color(workspace.agent))
                        .bold(),
                ));
                let mut line2_width = line2_prefix_width + text_display_width(&agent_label);
                let mut pr_hit_targets: Vec<(u16, u16, u64)> = Vec::new();
                if !workspace.is_main && !workspace.pull_requests.is_empty() {
                    line2_spans.push(FtSpan::styled(" · PRs:".to_string(), secondary_style));
                    line2_width = line2_width.saturating_add(text_display_width(" · PRs:"));
                    for (pull_request_index, pull_request) in
                        workspace.pull_requests.iter().enumerate()
                    {
                        line2_spans.push(FtSpan::styled(" ".to_string(), secondary_style));
                        line2_width = line2_width.saturating_add(1);
                        let pull_request_label = format!(
                            "{} #{}",
                            Self::pull_request_status_icon(pull_request.status),
                            pull_request.number
                        );
                        let token_width = text_display_width(&pull_request_label);
                        let token_x = row_inner
                            .x
                            .saturating_add(u16::try_from(line2_width).unwrap_or(u16::MAX));
                        if let Some(hit_data) =
                            encode_workspace_pr_hit_data(idx, pull_request_index)
                        {
                            pr_hit_targets.push((
                                token_x,
                                u16::try_from(token_width).unwrap_or(u16::MAX),
                                hit_data,
                            ));
                        }
                        line2_spans.push(FtSpan::styled(
                            pull_request_label,
                            Self::pull_request_status_style(
                                pull_request.status,
                                secondary_style,
                                theme,
                            )
                            .underline(),
                        ));
                        line2_width = line2_width.saturating_add(token_width);
                    }
                }
                if self.delete_requested_workspaces.contains(&workspace.path) {
                    let deleting_text = " · Deleting...";
                    line2_spans.push(FtSpan::styled(
                        deleting_text,
                        secondary_style.fg(theme.peach).bold(),
                    ));
                    line2_width = line2_width.saturating_add(text_display_width(deleting_text));
                }
                if workspace.is_orphaned {
                    let orphaned_text = " · session ended";
                    line2_spans.push(FtSpan::styled(
                        orphaned_text,
                        secondary_style.fg(theme.peach),
                    ));
                    line2_width = line2_width.saturating_add(text_display_width(orphaned_text));
                }

                Paragraph::new(FtText::from_lines(vec![
                    FtLine::from_spans(line1_spans),
                    FtLine::from_spans(line2_spans),
                ]))
                .render(row_inner, frame);

                if is_working {
                    animated_labels.push((
                        workspace_name,
                        workspace.agent,
                        row_inner
                            .x
                            .saturating_add(u16::try_from(line1_prefix_width).unwrap_or(u16::MAX)),
                        row_inner.y,
                        row_inner.right(),
                    ));
                    animated_labels.push((
                        workspace.agent.label().to_string(),
                        workspace.agent,
                        row_inner
                            .x
                            .saturating_add(u16::try_from(line2_prefix_width).unwrap_or(u16::MAX)),
                        row_inner.y.saturating_add(1),
                        row_inner.right(),
                    ));
                }

                if let Ok(data) = u64::try_from(idx) {
                    let _ = frame.register_hit(
                        row_rect,
                        HitId::new(HIT_ID_WORKSPACE_ROW),
                        FrameHitRegion::Content,
                        data,
                    );
                }
                for (token_x, token_width, data) in pr_hit_targets {
                    if token_x >= row_inner.right() {
                        continue;
                    }
                    let visible_width = token_width.min(row_inner.right().saturating_sub(token_x));
                    if visible_width == 0 {
                        continue;
                    }
                    let _ = frame.register_hit(
                        Rect::new(token_x, row_inner.y.saturating_add(1), visible_width, 1),
                        HitId::new(HIT_ID_WORKSPACE_PR_LINK),
                        FrameHitRegion::Content,
                        data,
                    );
                }

                current_y = current_y.saturating_add(WORKSPACE_ITEM_HEIGHT);
                let _ = line2_width;
            }
        }

        for (label, agent, x, y, max_x) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = max_x.saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
    }
}
