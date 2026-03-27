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
                    let is_working =
                        self.status_is_visually_working(Some(workspace.path.as_path()), true);
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
                            Style::new().fg(packed(theme.primary)).bold()
                        } else {
                            Style::new().fg(packed(theme.text)).bold()
                        },
                    )];
                    if let Some(branch_label) = branch_label {
                        spans.push(FtSpan::styled(
                            " · ",
                            Style::new().fg(packed(theme.text_subtle)),
                        ));
                        spans.push(FtSpan::styled(
                            branch_label.clone(),
                            Style::new().fg(packed(theme.text_subtle)),
                        ));
                    }
                    if !age_label.is_empty() {
                        spans.push(FtSpan::styled(
                            " · ",
                            Style::new().fg(packed(theme.text_subtle)),
                        ));
                        spans.push(FtSpan::styled(
                            age_label.clone(),
                            Style::new().fg(packed(theme.border)),
                        ));
                    }
                    if let Some(diff_stat) = selected_workspace
                        .and_then(|ws| self.diff_stat_for_workspace(ws.path.as_path()))
                    {
                        spans.push(FtSpan::styled(
                            " · ",
                            Style::new().fg(packed(theme.text_subtle)),
                        ));
                        spans.push(FtSpan::styled(
                            format!("+{}", diff_stat.insertions),
                            Style::new().fg(packed(theme.success)).bold(),
                        ));
                        spans.push(FtSpan::styled(
                            format!(" -{}", diff_stat.deletions),
                            Style::new().fg(packed(theme.error)).bold(),
                        ));
                    }
                    if *is_orphaned {
                        spans.push(FtSpan::styled(
                            " · ",
                            Style::new().fg(packed(theme.text_subtle)),
                        ));
                        spans.push(FtSpan::styled(
                            "session ended",
                            Style::new().fg(packed(theme.accent)),
                        ));
                    }
                    FtLine::from_spans(spans)
                } else {
                    FtLine::from_spans(vec![FtSpan::styled(
                        "none selected",
                        Style::new().fg(packed(theme.text_subtle)),
                    )])
                },
            ];
        let tab_active_style = Style::new()
            .fg(packed(theme.background))
            .bg(packed(theme.primary))
            .bold();
        let tab_inactive_style = Style::new()
            .fg(packed(theme.text_subtle))
            .bg(packed(theme.surface));
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
                tab_spans.push(FtSpan::styled(
                    " · ",
                    Style::new().fg(packed(theme.text_subtle)),
                ));
                tab_spans.push(FtSpan::styled(
                    task.root_path.display().to_string(),
                    Style::new().fg(packed(theme.border)),
                ));
            }
        } else if let Some(workspace) = selected_workspace {
            tab_spans.push(FtSpan::styled(
                " · ",
                Style::new().fg(packed(theme.text_subtle)),
            ));
            tab_spans.push(FtSpan::styled(
                workspace.path.display().to_string(),
                Style::new().fg(packed(theme.border)),
            ));
        } else {
            tab_spans.push(FtSpan::styled(
                " · ",
                Style::new().fg(packed(theme.text_subtle)),
            ));
            tab_spans.push(FtSpan::styled(
                "no workspace",
                Style::new().fg(packed(theme.border)),
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
        _preview_height: usize,
        _allow_cursor_overlay: bool,
    ) -> Vec<PreviewParsedLine> {
        let active_parsed_lines = self.preview.active_parsed_lines();
        let parsed_start = visible_start.min(active_parsed_lines.len());
        let parsed_end = visible_end.min(active_parsed_lines.len());
        let visible_parsed_slice = if parsed_start < parsed_end {
            &active_parsed_lines[parsed_start..parsed_end]
        } else {
            &[]
        };
        visible_plain_lines
            .iter()
            .enumerate()
            .map(|(index, plain_line)| {
                visible_parsed_slice
                    .get(index)
                    .filter(|line| preview_parsed_line_plain_text(line) == plain_line.as_str())
                    .cloned()
                    .unwrap_or_else(|| plain_preview_line(plain_line))
            })
            .collect::<Vec<_>>()
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
        if self.interactive_preview_reset_pending {
            return Vec::new();
        }

        let theme = self.active_ui_theme();
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
                PreviewTab::Diff => FtLine::raw("(no diff output)"),
            }];
        }

        visible_parsed_lines
            .iter()
            .map(|line| parsed_preview_line_to_ft_line(line, theme))
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

fn parsed_preview_line_to_ft_line(
    line: &PreviewParsedLine,
    theme: ftui::ResolvedTheme,
) -> FtLine<'static> {
    if line.spans.is_empty() {
        return FtLine::raw("");
    }
    if line.spans.len() == 1 && preview_style_is_plain(&line.spans[0].style) {
        return FtLine::raw(line.spans[0].text.clone());
    }

    FtLine::from_spans(
        line.spans
            .iter()
            .map(|span| parsed_preview_span_to_ft_span(span, theme)),
    )
}

fn parsed_preview_span_to_ft_span(
    span: &PreviewParsedSpan,
    theme: ftui::ResolvedTheme,
) -> FtSpan<'static> {
    if let Some(style) = parsed_preview_style_to_ft_style(&span.style, theme) {
        FtSpan::styled(span.text.clone(), style)
    } else {
        FtSpan::raw(span.text.clone())
    }
}

fn parsed_preview_style_to_ft_style(
    style: &PreviewParsedStyle,
    theme: ftui::ResolvedTheme,
) -> Option<Style> {
    let mut ft_style = Style::new();

    let foreground = preview_foreground_color(style, theme);
    let background = preview_background_color(style, theme);

    if let Some(color) = foreground {
        ft_style = ft_style.fg(color);
    }
    if let Some(color) = background {
        ft_style = ft_style.bg(color);
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

    if preview_style_is_plain(style) {
        None
    } else {
        Some(ft_style)
    }
}

fn preview_foreground_color(
    style: &PreviewParsedStyle,
    theme: ftui::ResolvedTheme,
) -> Option<PackedRgba> {
    let (r, g, b) = style.foreground_rgb?;
    Some(ansi16_theme_color(r, g, b, theme).unwrap_or(PackedRgba::rgb(r, g, b)))
}

fn preview_background_color(
    style: &PreviewParsedStyle,
    theme: ftui::ResolvedTheme,
) -> Option<PackedRgba> {
    let (r, g, b) = style.background_rgb?;
    Some(ansi16_theme_color(r, g, b, theme).unwrap_or(PackedRgba::rgb(r, g, b)))
}

/// Map the fixed RGB values that upstream ftui-pty produces for ANSI 16
/// colors back to theme-appropriate colors. Truecolor values pass through.
fn ansi16_theme_color(r: u8, g: u8, b: u8, theme: ftui::ResolvedTheme) -> Option<PackedRgba> {
    let base = match (r, g, b) {
        (0, 0, 0) => packed(theme.overlay),
        (170, 0, 0) => packed(theme.error),
        (0, 170, 0) => packed(theme.success),
        (170, 170, 0) => packed(theme.warning),
        (0, 0, 170) => packed(theme.primary),
        (170, 0, 170) => packed(theme.secondary),
        (0, 170, 170) => packed(theme.info),
        (170, 170, 170) => packed(theme.text_muted),
        (85, 85, 85) => packed(theme.border),
        (255, 85, 85) => bright_variant(packed(theme.error), theme),
        (85, 255, 85) => bright_variant(packed(theme.success), theme),
        (255, 255, 85) => bright_variant(packed(theme.warning), theme),
        (85, 85, 255) => bright_variant(packed(theme.primary), theme),
        (255, 85, 255) => bright_variant(packed(theme.secondary), theme),
        (85, 255, 255) => bright_variant(packed(theme.info), theme),
        (255, 255, 255) => packed(theme.text),
        _ => return None,
    };
    Some(base)
}

fn bright_variant(color: PackedRgba, theme: ftui::ResolvedTheme) -> PackedRgba {
    let target = if is_dark_theme(theme) {
        packed(theme.text)
    } else {
        packed(theme.text_subtle)
    };
    blend(color, target, 0.22)
}

fn is_dark_theme(theme: ftui::ResolvedTheme) -> bool {
    luminance(packed(theme.background)) < luminance(packed(theme.text))
}

fn blend(source: PackedRgba, target: PackedRgba, amount: f32) -> PackedRgba {
    let mix = |from: u8, to: u8| -> u8 {
        let from = from as f32;
        let to = to as f32;
        (from + (to - from) * amount).round().clamp(0.0, 255.0) as u8
    };
    PackedRgba::rgb(
        mix(source.r(), target.r()),
        mix(source.g(), target.g()),
        mix(source.b(), target.b()),
    )
}

fn luminance(color: PackedRgba) -> f32 {
    let channel = |value: u8| -> f32 {
        let normalized = value as f32 / 255.0;
        if normalized <= 0.04045 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

fn preview_parsed_line_plain_text(line: &PreviewParsedLine) -> String {
    line.spans.iter().map(|span| span.text.as_str()).collect()
}

fn preview_style_is_plain(style: &PreviewParsedStyle) -> bool {
    style.foreground_rgb.is_none()
        && style.background_rgb.is_none()
        && !style.bold
        && !style.dim
        && !style.italic
        && !style.underline
        && !style.blink
        && !style.reverse
        && !style.strikethrough
}

#[cfg(test)]
mod tests {
    use super::{
        PreviewParsedLine, PreviewParsedSpan, PreviewParsedStyle, parsed_preview_line_to_ft_line,
    };
    use crate::ui::tui::shared::ui_theme;

    #[test]
    fn plain_preview_line_preserves_unicode_text() {
        let line = PreviewParsedLine {
            spans: vec![PreviewParsedSpan {
                text: "render-check 🧪".to_string(),
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
        };

        assert_eq!(
            parsed_preview_line_to_ft_line(&line, ui_theme()).to_plain_text(),
            "render-check 🧪"
        );
    }
}
