impl GroveApp {
    fn keybind_help_hints(context: HelpHintContext) -> Vec<&'static str> {
        UiCommand::help_hints_for(context)
            .iter()
            .filter_map(|command| command.help_hint_label(context))
            .collect()
    }

    fn keybind_help_join_hint_indexes(hints: &[&str], indexes: &[usize]) -> String {
        indexes
            .iter()
            .filter_map(|index| hints.get(*index).copied())
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn keybind_help_section_title(content_width: usize, theme: UiTheme, title: &str) -> FtLine {
        let title_text = format!("[{title}]");
        let title_width = text_display_width(title_text.as_str());
        let remaining = content_width.saturating_sub(title_width);
        let fill = if remaining > 0 {
            format!(" {}", "-".repeat(remaining.saturating_sub(1)))
        } else {
            String::new()
        };
        FtLine::from_spans(vec![
            FtSpan::styled(title_text, Style::new().fg(theme.blue).bold()),
            FtSpan::styled(fill, Style::new().fg(theme.overlay0)),
        ])
    }

    fn keybind_help_push_labeled_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
        value_fg: PackedRgba,
    ) {
        let label_cell =
            pad_or_truncate_to_display_width(format!("  {label}:").as_str(), label_width);
        let continuation_label = " ".repeat(label_width);
        if content_width <= label_width.saturating_add(1) {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(label_cell.as_str(), content_width),
                Style::new().fg(theme.lavender).bold(),
            )]));
            return;
        }
        let text_width = content_width.saturating_sub(label_width.saturating_add(1));
        let wrapped = ftui::text::wrap_text(text, text_width, ftui::text::WrapMode::Word);
        for (line_index, segment) in wrapped.iter().enumerate() {
            let row_label = if line_index == 0 {
                label_cell.as_str()
            } else {
                continuation_label.as_str()
            };
            lines.push(FtLine::from_spans(vec![
                FtSpan::styled(
                    row_label,
                    Style::new()
                        .fg(if line_index == 0 {
                            theme.lavender
                        } else {
                            theme.overlay0
                        })
                        .bold(),
                ),
                FtSpan::styled(" ", Style::new().fg(theme.overlay0)),
                FtSpan::styled(
                    pad_or_truncate_to_display_width(segment.as_str(), text_width),
                    Style::new().fg(value_fg),
                ),
            ]));
        }
    }

    fn keybind_help_push_section_gap(lines: &mut Vec<FtLine>, section_gap: usize) {
        if section_gap > 0 {
            lines.extend(
                std::iter::repeat_with(|| FtLine::raw(""))
                    .take(section_gap)
                    .collect::<Vec<FtLine>>(),
            );
        }
    }

    fn keybind_help_push_modal_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.subtext0,
        );
    }

    fn keybind_help_push_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.text,
        );
    }

    fn keybind_help_push_palette_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.subtext0,
        );
    }
}
