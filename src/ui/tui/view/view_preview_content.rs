use super::view_prelude::*;

type AnimatedPreviewLabels = Vec<(String, AgentType, u16, u16)>;

impl GroveApp {
    pub(super) fn preview_metadata_lines_and_labels(
        &self,
        inner: Rect,
        selected_workspace: Option<&Workspace>,
    ) -> (Vec<FtLine>, AnimatedPreviewLabels) {
        let theme = ui_theme();
        let mut animated_labels: AnimatedPreviewLabels = Vec::new();
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
        for (index, tab) in [PreviewTab::Agent, PreviewTab::Shell, PreviewTab::Git]
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

        (text_lines, animated_labels)
    }

    fn preview_visible_render_lines(
        &self,
        visible_plain_lines: &[String],
        visible_start: usize,
        visible_end: usize,
        preview_height: usize,
        allow_cursor_overlay: bool,
    ) -> Vec<String> {
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
            visible_render_lines = visible_plain_lines.to_vec();
        }
        if allow_cursor_overlay {
            self.apply_interactive_cursor_overlay_render(
                visible_plain_lines,
                &mut visible_render_lines,
                preview_height,
            );
        }
        visible_render_lines
    }

    fn preview_git_fallback_line(&self, selected_workspace: Option<&Workspace>) -> FtLine {
        let fallback = if let Some(workspace) = selected_workspace {
            let session_name = git_session_name_for_workspace(workspace);
            if self.session.lazygit_sessions.is_failed(&session_name) {
                "(lazygit launch failed)"
            } else if self.session.lazygit_sessions.is_ready(&session_name) {
                "(no lazygit output yet)"
            } else {
                "(launching lazygit...)"
            }
        } else {
            "(no workspace selected)"
        };
        FtLine::raw(fallback.to_string())
    }

    fn preview_shell_fallback_line(&self, selected_workspace: Option<&Workspace>) -> FtLine {
        let fallback = if let Some(workspace) = selected_workspace {
            let session_name = shell_session_name_for_workspace(workspace);
            if self.session.shell_sessions.is_failed(&session_name) {
                "(shell launch failed)"
            } else if self.session.shell_sessions.is_ready(&session_name) {
                "(no shell output yet)"
            } else {
                "(launching shell...)"
            }
        } else {
            "(no workspace selected)"
        };
        FtLine::raw(fallback.to_string())
    }

    pub(super) fn preview_tab_content_lines(
        &self,
        selected_workspace: Option<&Workspace>,
        allow_cursor_overlay: bool,
        visible_plain_lines: &[String],
        visible_start: usize,
        visible_end: usize,
        preview_height: usize,
    ) -> Vec<FtLine> {
        let visible_render_lines = self.preview_visible_render_lines(
            visible_plain_lines,
            visible_start,
            visible_end,
            preview_height,
            allow_cursor_overlay,
        );

        if visible_render_lines.is_empty() {
            return vec![match self.preview_tab {
                PreviewTab::Agent => FtLine::raw("(no preview output)"),
                PreviewTab::Shell => self.preview_shell_fallback_line(selected_workspace),
                PreviewTab::Git => self.preview_git_fallback_line(selected_workspace),
            }];
        }

        ansi_lines_to_styled_lines(&visible_render_lines)
    }
}
