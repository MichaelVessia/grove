use super::*;

impl GroveApp {
    const KEYBIND_HELP_MIN_WIDTH: u16 = 56;
    const KEYBIND_HELP_MIN_HEIGHT: u16 = 16;
    const KEYBIND_HELP_HORIZONTAL_MARGIN: u16 = 1;
    const KEYBIND_HELP_VERTICAL_MARGIN: u16 = 0;
    const COMMAND_PALETTE_MIN_WIDTH: u16 = 44;
    const COMMAND_PALETTE_HORIZONTAL_MARGIN: u16 = 2;

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

    pub(super) fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }

    pub(super) fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.command_palette.is_visible() {
            return;
        }
        if area.width < 16 || area.height < 7 {
            return;
        }

        let theme = ui_theme();
        let max_dialog_width = area.width.saturating_sub(2);
        if max_dialog_width == 0 {
            return;
        }
        let preferred_width = area
            .width
            .saturating_sub(Self::COMMAND_PALETTE_HORIZONTAL_MARGIN.saturating_mul(2));
        let dialog_width = preferred_width
            .max(Self::COMMAND_PALETTE_MIN_WIDTH)
            .min(max_dialog_width);
        let max_visible_from_height = usize::from(area.height.saturating_sub(5).max(1));
        let visible_window = Self::command_palette_max_visible_for_height(self.viewport_height)
            .max(1)
            .min(max_visible_from_height);
        let result_count = self.command_palette.result_count();
        let selected_index = if result_count == 0 {
            0
        } else {
            self.command_palette
                .selected_index()
                .min(result_count.saturating_sub(1))
        };
        let scroll_offset = selected_index
            .saturating_add(1)
            .saturating_sub(visible_window);
        let list_rows = result_count
            .saturating_sub(scroll_offset)
            .min(visible_window)
            .max(1);
        let dialog_height = u16::try_from(list_rows)
            .unwrap_or(u16::MAX)
            .saturating_add(3)
            .min(area.height.saturating_sub(2))
            .max(5);
        let dialog_x = area.x + area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.y + area.height.saturating_sub(dialog_height) / 3;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        let content_style = Style::new().fg(theme.text).bg(theme.base);
        Paragraph::new("")
            .style(content_style)
            .render(dialog_area, frame);

        let block = Block::new()
            .title("Command Palette")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(theme.blue).bold());
        let inner = block.inner(dialog_area);
        block.render(dialog_area, frame);
        if inner.is_empty() {
            return;
        }

        let query_area = Rect::new(inner.x, inner.y, inner.width, 1);
        let query = self.command_palette.query();
        let mut query_spans = vec![FtSpan::styled(
            "> ",
            Style::new().fg(theme.blue).bg(theme.base).bold(),
        )];
        if query.is_empty() {
            query_spans.push(FtSpan::styled(
                "Type to search...",
                Style::new().fg(theme.overlay0).bg(theme.base),
            ));
        } else {
            query_spans.push(FtSpan::styled(
                query,
                Style::new().fg(theme.text).bg(theme.base),
            ));
        }
        Paragraph::new(FtLine::from_spans(query_spans))
            .style(content_style)
            .render(query_area, frame);

        let prompt_width = 2usize;
        let query_max_col = usize::from(query_area.width.saturating_sub(1));
        let query_cursor_col = prompt_width
            .saturating_add(text_display_width(query))
            .min(query_max_col);
        let cursor_x = query_area
            .x
            .saturating_add(u16::try_from(query_cursor_col).unwrap_or(u16::MAX));
        frame.cursor_position = Some((cursor_x, query_area.y));
        frame.cursor_visible = true;

        let list_area = Rect::new(
            inner.x,
            inner.y.saturating_add(1),
            inner.width,
            inner.height.saturating_sub(1),
        );
        if list_area.is_empty() {
            return;
        }

        if result_count == 0 {
            let message = if query.is_empty() {
                "No actions registered"
            } else {
                "No results"
            };
            let line = pad_or_truncate_to_display_width(message, usize::from(list_area.width));
            Paragraph::new(FtLine::from_spans(vec![FtSpan::styled(
                line,
                Style::new().fg(theme.overlay0).bg(theme.base),
            )]))
            .style(content_style)
            .render(
                Rect::new(list_area.x, list_area.y, list_area.width, 1),
                frame,
            );
            return;
        }

        let results: Vec<_> = self.command_palette.results().collect();
        let visible_rows =
            usize::from(list_area.height).min(results.len().saturating_sub(scroll_offset));
        let visible_end = scroll_offset
            .saturating_add(visible_rows)
            .min(results.len());
        for (row_index, palette_match) in results[scroll_offset..visible_end].iter().enumerate() {
            let is_selected = scroll_offset.saturating_add(row_index) == selected_index;
            let row_y = list_area
                .y
                .saturating_add(u16::try_from(row_index).unwrap_or(u16::MAX));
            let row_bg = if is_selected {
                theme.surface0
            } else {
                theme.base
            };
            let row_fg = if is_selected {
                theme.text
            } else {
                theme.subtext0
            };
            let marker_style = if is_selected {
                Style::new().fg(theme.yellow).bg(row_bg).bold()
            } else {
                Style::new().fg(theme.overlay0).bg(row_bg)
            };
            let text_style = Style::new().fg(row_fg).bg(row_bg);
            let keybind_style = if is_selected {
                Style::new().fg(theme.peach).bg(row_bg).bold()
            } else {
                Style::new().fg(theme.overlay0).bg(row_bg)
            };

            let category_label =
                Self::command_palette_category_label(palette_match.action.category.as_deref());
            let mut title = if category_label.is_empty() {
                palette_match.action.title.clone()
            } else {
                format!("[{category_label}] {}", palette_match.action.title)
            };
            let (summary, keybind) = palette_match
                .action
                .description
                .as_deref()
                .map(Self::command_palette_split_description)
                .unwrap_or(("", None));
            if !summary.is_empty() {
                title.push(' ');
                title.push_str(summary);
            }
            let keybind_label = keybind.map(|value| format!("[{value}]"));
            let mut spans = Vec::new();
            spans.push(FtSpan::styled(
                if is_selected { ">" } else { " " },
                marker_style,
            ));
            spans.push(FtSpan::styled(" ", text_style));

            let content_width = usize::from(list_area.width);
            let body_width = content_width.saturating_sub(2);
            if let Some(keybind_value) = keybind_label {
                let bounded_keybind =
                    truncate_to_display_width(keybind_value.as_str(), body_width.saturating_sub(1));
                let keybind_width = text_display_width(bounded_keybind.as_str());
                let title_width = body_width.saturating_sub(keybind_width.saturating_add(1));
                spans.push(FtSpan::styled(
                    pad_or_truncate_to_display_width(title.as_str(), title_width),
                    text_style,
                ));
                spans.push(FtSpan::styled(" ", text_style));
                spans.push(FtSpan::styled(bounded_keybind, keybind_style));
            } else {
                spans.push(FtSpan::styled(
                    pad_or_truncate_to_display_width(title.as_str(), body_width),
                    text_style,
                ));
            }

            Paragraph::new(FtLine::from_spans(spans))
                .style(Style::new().fg(row_fg).bg(row_bg))
                .render(Rect::new(list_area.x, row_y, list_area.width, 1), frame);
        }
    }

    fn command_palette_category_label(category: Option<&str>) -> &str {
        match category {
            Some("Navigation") => "Nav",
            Some("Workspace") => "Ws",
            Some("Preview") => "Prev",
            Some("System") => "Sys",
            Some("List") => "List",
            Some(value) => value,
            None => "",
        }
    }

    fn command_palette_split_description(description: &str) -> (&str, Option<&str>) {
        let trimmed = description.trim();
        let Some(without_suffix) = trimmed.strip_suffix(')') else {
            return (trimmed, None);
        };
        let Some(open_index) = without_suffix.rfind('(') else {
            return (trimmed, None);
        };
        let summary = without_suffix[..open_index].trim_end();
        let keybind = without_suffix[open_index.saturating_add(1)..].trim();
        if keybind.is_empty() {
            return (trimmed, None);
        }
        (summary, Some(keybind))
    }

    pub(super) fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.keybind_help_open {
            return;
        }
        if area.width < Self::KEYBIND_HELP_MIN_WIDTH.saturating_add(2)
            || area.height < Self::KEYBIND_HELP_MIN_HEIGHT.saturating_add(2)
        {
            return;
        }

        let dialog_width = area
            .width
            .saturating_sub(Self::KEYBIND_HELP_HORIZONTAL_MARGIN.saturating_mul(2))
            .max(Self::KEYBIND_HELP_MIN_WIDTH);
        let dialog_height = area
            .height
            .saturating_sub(Self::KEYBIND_HELP_VERTICAL_MARGIN.saturating_mul(2))
            .max(Self::KEYBIND_HELP_MIN_HEIGHT);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let section_gap = if dialog_height >= 42 { 1 } else { 0 };
        let label_width = if content_width >= 132 {
            18
        } else if content_width >= 96 {
            15
        } else {
            12
        };
        let global_hints = Self::keybind_help_hints(HelpHintContext::Global);
        let workspace_hints = Self::keybind_help_hints(HelpHintContext::Workspace);
        let list_hints = Self::keybind_help_hints(HelpHintContext::List);
        let preview_agent_hints = Self::keybind_help_hints(HelpHintContext::PreviewAgent);
        let preview_shell_hints = Self::keybind_help_hints(HelpHintContext::PreviewShell);
        let preview_git_hints = Self::keybind_help_hints(HelpHintContext::PreviewGit);
        let global_core =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[0, 1, 11]);
        let global_focus =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[2, 6, 10]);
        let global_layout =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[3, 4, 5]);
        let global_tabs = Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[7, 8, 9]);
        let workspace_actions = workspace_hints.join(", ");

        let mut lines = vec![Self::keybind_help_section_title(
            content_width,
            theme,
            "Global",
        )];
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Core",
            global_core.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Focus",
            global_focus.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Layout",
            global_layout.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Workspace nav",
            global_tabs.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Workspace",
            workspace_actions.as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Palette",
        ));
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Search",
            "[Palette] Type search",
            theme,
        );
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Navigate",
            "Up/Down or C-n/C-p move selection",
            theme,
        );
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Run/Close",
            "Enter run, Esc close",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "List",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Move",
            list_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Preview",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Agent tab",
            preview_agent_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Shell tab",
            preview_shell_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Git tab",
            preview_git_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Interactive",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Input",
            "typing sends input to attached session, includes Shift+Tab and Shift+Enter",
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Reserved",
            "Ctrl+K palette, Esc Esc/Ctrl+\\ exit, Alt+J/K browse, Alt+[/] tabs, Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback), Alt+C copy, Alt+V paste",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Modals",
        ));
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Create",
            "Tab/S-Tab or C-n/C-p fields, j/k adjust controls, ';' separates setup commands, Space toggles auto-run, h/l buttons, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Edit",
            "Tab/S-Tab or C-n/C-p fields, type/backspace base, h/l or Space toggle agent, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Start",
            "Tab/S-Tab or C-n/C-p fields, Space toggle unsafe, h/l buttons, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Delete",
            "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/D confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Merge",
            "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/m confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Update",
            "Tab/S-Tab or C-n/C-p fields, h/l buttons, Enter/u confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Projects",
            "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Ctrl+R reorder, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);
        lines.extend(modal_wrapped_rows(
            content_width,
            "Close help: Esc, Enter, or ?",
            Style::new().fg(theme.lavender).bg(theme.base).bold(),
        ));

        let content = OverlayModalContent {
            title: "Keybind Help",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.blue,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_KEYBIND_HELP_DIALOG))
            .render(area, frame);
    }
}
