use super::*;

impl GroveApp {
    pub(super) fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
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
}
