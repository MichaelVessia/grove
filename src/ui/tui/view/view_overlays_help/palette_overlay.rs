impl GroveApp {
    pub(super) fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.dialogs.command_palette.is_visible() {
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
        let result_count = self.dialogs.command_palette.result_count();
        let selected_index = if result_count == 0 {
            0
        } else {
            self.dialogs.command_palette
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
        let query = self.dialogs.command_palette.query();
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

        let results: Vec<_> = self.dialogs.command_palette.results().collect();
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
}
