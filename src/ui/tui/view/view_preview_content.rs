use super::view_prelude::*;
use crate::application::preview::{PreviewParsedLine, PreviewParsedSpan, PreviewParsedStyle};

type AnimatedPreviewLabels = Vec<(String, u16, u16)>;

impl GroveApp {
    pub(super) fn preview_metadata_lines_and_labels(
        &self,
        inner: Rect,
        selected_workspace: Option<&Workspace>,
    ) -> (Vec<FtLine<'static>>, AnimatedPreviewLabels) {
        let theme = self.active_ui_theme();
        let mut animated_labels: AnimatedPreviewLabels = Vec::new();
        let selected_workspace_header =
            if self.preview_tab == PreviewTab::Home && self.selected_task_supports_parent_agent() {
                self.state.selected_task().map(|task| {
                    let is_working = self.selected_task_preview_session_if_ready().is_some();
                    let branch_label = (task.branch != task.name).then_some(task.branch.clone());
                    (
                        task.name.clone(),
                        branch_label,
                        String::new(),
                        is_working,
                        false,
                    )
                })
            } else {
                selected_workspace.map(|workspace| {
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
                        workspace.is_orphaned,
                    )
                })
            };

        let mut text_lines =
            vec![
                if let Some((name_label, branch_label, age_label, is_working, is_orphaned)) =
                    selected_workspace_header.as_ref()
                {
                    let mut spans = vec![FtSpan::styled(
                        name_label.clone(),
                        if *is_working {
                            Style::new().fg(theme.blue).bold()
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
                },
            ];
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut tab_spans = Vec::new();
        if let Some(workspace) = selected_workspace
            && let Some(tabs) = self.workspace_tabs.get(workspace.path.as_path())
        {
            for (index, tab) in tabs.tabs.iter().enumerate() {
                if index > 0 {
                    tab_spans.push(FtSpan::raw(" ".to_string()));
                }
                let style = if tab.id == tabs.active_tab_id {
                    tab_active_style
                } else {
                    tab_inactive_style
                };
                tab_spans.push(FtSpan::styled(format!(" {} ", tab.title), style));
            }
        }
        if self.preview_tab == PreviewTab::Home && self.selected_task_supports_parent_agent() {
            if let Some(task) = self.state.selected_task() {
                tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                tab_spans.push(FtSpan::styled(
                    task.root_path.display().to_string(),
                    Style::new().fg(theme.overlay0),
                ));
            }
        } else if let Some(workspace) = selected_workspace {
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
        if let Some((name_label, _, _, true, _)) = selected_workspace_header.as_ref() {
            animated_labels.push((name_label.clone(), inner.x, inner.y));
        }

        (text_lines, animated_labels)
    }

    fn preview_visible_parsed_lines(
        &self,
        visible_plain_lines: &[String],
        visible_start: usize,
        visible_end: usize,
        preview_height: usize,
        allow_cursor_overlay: bool,
    ) -> Vec<PreviewParsedLine> {
        let mut visible_parsed_lines = if self.preview.parsed_lines.is_empty() {
            Vec::new()
        } else {
            let parsed_start = visible_start.min(self.preview.parsed_lines.len());
            let parsed_end = visible_end.min(self.preview.parsed_lines.len());
            if parsed_start < parsed_end {
                self.preview.parsed_lines[parsed_start..parsed_end].to_vec()
            } else {
                Vec::new()
            }
        };
        if visible_parsed_lines.len() < visible_plain_lines.len() {
            visible_parsed_lines.extend(
                visible_plain_lines[visible_parsed_lines.len()..]
                    .iter()
                    .map(|line| plain_preview_line(line)),
            );
        }
        if visible_parsed_lines.is_empty() && !visible_plain_lines.is_empty() {
            visible_parsed_lines = visible_plain_lines
                .iter()
                .map(|line| plain_preview_line(line))
                .collect();
        }
        if allow_cursor_overlay {
            self.apply_interactive_cursor_overlay_parsed(&mut visible_parsed_lines, preview_height);
        }
        visible_parsed_lines
    }

    fn preview_git_fallback_line(&self, selected_workspace: Option<&Workspace>) -> FtLine<'static> {
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

    fn preview_shell_fallback_line(
        &self,
        selected_workspace: Option<&Workspace>,
    ) -> FtLine<'static> {
        let fallback = if selected_workspace.is_some() {
            if let Some(session_name) = self.selected_shell_tab_session_name() {
                if self.session.shell_sessions.is_failed(&session_name) {
                    "(shell launch failed)"
                } else if self.session.shell_sessions.is_ready(&session_name) {
                    "(no shell output yet)"
                } else {
                    "(launching shell...)"
                }
            } else {
                "(no shell tab selected)"
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
    ) -> Vec<FtLine<'static>> {
        let visible_parsed_lines = self.preview_visible_parsed_lines(
            visible_plain_lines,
            visible_start,
            visible_end,
            preview_height,
            allow_cursor_overlay,
        );

        if visible_parsed_lines.is_empty() {
            return vec![match self.preview_tab {
                PreviewTab::Home => FtLine::raw("(home)"),
                PreviewTab::Agent => FtLine::raw("(no preview output)"),
                PreviewTab::Shell => self.preview_shell_fallback_line(selected_workspace),
                PreviewTab::Git => self.preview_git_fallback_line(selected_workspace),
            }];
        }

        visible_parsed_lines
            .iter()
            .map(parsed_preview_line_to_ft_line)
            .collect()
    }
}

fn plain_preview_line(line: &str) -> PreviewParsedLine {
    PreviewParsedLine {
        spans: vec![PreviewParsedSpan {
            text: line.to_string(),
            style: PreviewParsedStyle {
                foreground_rgb: None,
                background_rgb: None,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                blink: false,
                reverse: false,
                strikethrough: false,
            },
        }],
    }
}

fn parsed_preview_line_to_ft_line(line: &PreviewParsedLine) -> FtLine<'static> {
    if line.spans.is_empty() {
        return FtLine::raw("");
    }

    FtLine::from_spans(line.spans.iter().map(parsed_preview_span_to_ft_span))
}

fn parsed_preview_span_to_ft_span(span: &PreviewParsedSpan) -> FtSpan<'static> {
    if let Some(style) = parsed_preview_style_to_ft_style(&span.style) {
        FtSpan::styled(span.text.clone(), style)
    } else {
        FtSpan::raw(span.text.clone())
    }
}

fn parsed_preview_style_to_ft_style(style: &PreviewParsedStyle) -> Option<Style> {
    let mut ft_style = Style::new();

    if let Some((r, g, b)) = style.foreground_rgb {
        ft_style = ft_style.fg(PackedRgba::rgb(r, g, b));
    }
    if let Some((r, g, b)) = style.background_rgb {
        ft_style = ft_style.bg(PackedRgba::rgb(r, g, b));
    }
    if style.bold {
        ft_style = ft_style.bold();
    }
    if style.dim {
        ft_style = ft_style.dim();
    }
    if style.italic {
        ft_style = ft_style.italic();
    }
    if style.underline {
        ft_style = ft_style.underline();
    }
    if style.blink {
        ft_style = ft_style.blink();
    }
    if style.reverse {
        ft_style = ft_style.reverse();
    }
    if style.strikethrough {
        ft_style = ft_style.strikethrough();
    }

    if style.foreground_rgb.is_none()
        && style.background_rgb.is_none()
        && !style.bold
        && !style.dim
        && !style.italic
        && !style.underline
        && !style.blink
        && !style.reverse
        && !style.strikethrough
    {
        None
    } else {
        Some(ft_style)
    }
}
