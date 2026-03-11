use super::view_prelude::*;

impl GroveApp {
    #[inline]
    pub(super) fn preview_line_display_width(line: &str) -> usize {
        ftui::text::display_width(line)
    }

    // ftui exposes width and grapheme primitives, but preview selection needs
    // inclusive cell-range slicing and grapheme-at-cell metadata.
    pub(super) fn preview_substring_by_cells(
        line: &str,
        start_col: usize,
        end_col_inclusive: Option<usize>,
    ) -> String {
        let mut out = String::new();
        let end_col_exclusive = end_col_inclusive.map(|end| end.saturating_add(1));
        let mut visual_col = 0usize;

        for grapheme in ftui::text::graphemes(line) {
            if end_col_exclusive.is_some_and(|end| visual_col >= end) {
                break;
            }

            let width = Self::preview_line_display_width(grapheme);
            let next_col = visual_col.saturating_add(width);
            let intersects = if width == 0 {
                visual_col >= start_col
            } else {
                next_col > start_col
            };

            if intersects {
                out.push_str(grapheme);
            }

            visual_col = next_col;
        }

        out
    }

    pub(super) fn preview_grapheme_at_col(
        line: &str,
        target_col: usize,
    ) -> Option<(String, usize, usize)> {
        let mut visual_col = 0usize;

        for grapheme in ftui::text::graphemes(line) {
            let width = Self::preview_line_display_width(grapheme);
            let start_col = visual_col;
            let end_col = if width == 0 {
                start_col
            } else {
                start_col.saturating_add(width.saturating_sub(1))
            };

            if (width == 0 && target_col == start_col) || (width > 0 && target_col <= end_col) {
                return Some((grapheme.to_string(), start_col, end_col));
            }

            visual_col = visual_col.saturating_add(width);
        }

        None
    }

    pub(super) fn prepare_preview_selection_drag(&mut self, x: u16, y: u16) {
        let point = self.preview_text_point_at(x, y);
        self.log_preview_drag_started(x, y, point);
        if let Some(point) = point {
            self.preview_selection.prepare_drag(point);
            return;
        }

        self.clear_preview_selection();
    }

    pub(super) fn update_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        let Some(point) = self.preview_text_point_at(x, y) else {
            return;
        };
        self.preview_selection.handle_drag(point);
    }

    pub(super) fn finish_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        let release_point = self.preview_text_point_at(x, y);
        if !self.preview_selection.has_selection()
            && let Some(point) = release_point
        {
            self.preview_selection.handle_drag(point);
        }
        self.log_preview_drag_finished(x, y, release_point);
        self.preview_selection.finish_drag();
    }

    pub(super) fn apply_preview_selection_highlight_cells(
        &self,
        frame: &mut Frame,
        inner: Rect,
        visible_plain_lines: &[String],
        visible_start: usize,
    ) {
        if !self.preview_selection.has_selection() {
            return;
        }

        let selection_bg = self.active_ui_theme().surface1;
        let output_y = inner.y.saturating_add(PREVIEW_METADATA_ROWS);
        for (offset, line) in visible_plain_lines.iter().enumerate() {
            let line_idx = visible_start.saturating_add(offset);
            let Some((start_col, end_col)) = self.preview_selection.line_selection_cols(line_idx)
            else {
                continue;
            };

            let line_width = Self::preview_line_display_width(line);
            if line_width == 0 {
                continue;
            }

            let start = start_col.min(line_width.saturating_sub(1));
            let end = end_col
                .unwrap_or_else(|| line_width.saturating_sub(1))
                .min(line_width.saturating_sub(1));
            if end < start {
                continue;
            }

            let y = output_y.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            if y >= inner.bottom() {
                break;
            }

            let x_start = inner
                .x
                .saturating_add(u16::try_from(start).unwrap_or(u16::MAX));
            let x_end = inner
                .x
                .saturating_add(u16::try_from(end).unwrap_or(u16::MAX))
                .min(inner.right().saturating_sub(1));
            if x_start > x_end {
                continue;
            }

            for x in x_start..=x_end {
                if let Some(cell) = frame.buffer.get_mut(x, y) {
                    cell.bg = selection_bg;
                }
            }
        }
    }

    pub(super) fn selected_preview_text_lines(&self) -> Option<Vec<String>> {
        let (start, end) = self.preview_selection.bounds()?;
        let source_len = self.preview.lines.len();
        if source_len == 0 {
            return None;
        }

        let start_line = start.line.min(source_len.saturating_sub(1));
        let end_line = end.line.min(source_len.saturating_sub(1));
        if end_line < start_line {
            return None;
        }

        let mut lines = self.preview_plain_lines_range(start_line, end_line.saturating_add(1));
        if lines.is_empty() {
            return None;
        }

        if lines.len() == 1 {
            lines[0] = Self::preview_substring_by_cells(&lines[0], start.col, Some(end.col));
            return Some(lines);
        }

        lines[0] = Self::preview_substring_by_cells(&lines[0], start.col, None);
        let last_idx = lines.len().saturating_sub(1);
        lines[last_idx] = Self::preview_substring_by_cells(&lines[last_idx], 0, Some(end.col));

        Some(lines)
    }

    fn visible_preview_output_lines(&self) -> Vec<String> {
        let Some((_, output_height)) = self.preview_output_dimensions() else {
            return Vec::new();
        };
        let (visible_start, visible_end) =
            self.preview_visible_range_for_height(usize::from(output_height));
        self.preview_plain_lines_range(visible_start, visible_end)
    }

    pub(super) fn copy_interactive_selection_or_visible(&mut self) {
        let selected_lines = self.selected_preview_text_lines();
        let copied_from_selection = selected_lines.is_some();
        let mut lines = selected_lines.unwrap_or_else(|| self.visible_preview_output_lines());
        if lines.is_empty() {
            self.session.last_tmux_error = Some("no output to copy".to_string());
            self.show_info_toast("No output to copy");
            return;
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            self.session.last_tmux_error = Some("no output to copy".to_string());
            self.show_info_toast("No output to copy");
            return;
        }
        let text = lines.join("\n");
        self.telemetry.event_log.log(
            LogEvent::new("selection", "interactive_copy_payload")
                .with_data("from_selection", Value::from(copied_from_selection))
                .with_data("line_count", Value::from(usize_to_u64(lines.len())))
                .with_data(
                    "char_count",
                    Value::from(usize_to_u64(text.chars().count())),
                )
                .with_data("preview", Value::from(text.clone())),
        );
        self.copied_text = Some(text.clone());
        match self.clipboard.write_text(&text) {
            Ok(()) => {
                self.session.last_tmux_error = None;
                self.show_success_toast(format!("Copied {} line(s)", lines.len()));
            }
            Err(error) => {
                self.session.last_tmux_error = Some(format!("clipboard write failed: {error}"));
                self.show_error_toast(format!("Copy failed: {error}"));
            }
        }
        self.clear_preview_selection();
    }
}
