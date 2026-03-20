use super::view_prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreviewCopyLineSegment {
    logical_line: usize,
    logical_col_start: usize,
}

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
        self.preview_text_lines_from_bounds(start, end)
    }

    fn preview_text_lines_from_bounds(
        &self,
        start: TextSelectionPoint,
        end: TextSelectionPoint,
    ) -> Option<Vec<String>> {
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

    fn copy_target_session(&self) -> Option<String> {
        self.interactive_target_session()
            .or_else(|| self.selected_live_preview_session_if_ready())
    }

    fn capture_joined_preview_lines_for_copy(&self) -> Option<Vec<String>> {
        let session_name = self.copy_target_session()?;
        let joined_output = self
            .tmux_input
            .capture_joined_output(&session_name, LIVE_PREVIEW_FULL_SCROLLBACK_LINES, false)
            .ok()?;
        Some(crate::application::preview::split_output_lines(
            &joined_output,
        ))
    }

    fn visible_preview_output_bounds(&self) -> Option<(TextSelectionPoint, TextSelectionPoint)> {
        let (_, output_height) = self.preview_output_dimensions()?;
        let (visible_start, visible_end) =
            self.preview_visible_range_for_height(usize::from(output_height));
        if visible_start >= visible_end {
            return None;
        }

        let end_line = visible_end.saturating_sub(1);
        let end_col = self
            .preview_plain_line(end_line)
            .map(|line| Self::preview_line_display_width(&line).saturating_sub(1))
            .unwrap_or(0);

        Some((
            TextSelectionPoint {
                line: visible_start,
                col: 0,
            },
            TextSelectionPoint {
                line: end_line,
                col: end_col,
            },
        ))
    }

    fn preview_copy_bounds(&self) -> Option<(TextSelectionPoint, TextSelectionPoint)> {
        self.preview_selection
            .bounds()
            .or_else(|| self.visible_preview_output_bounds())
    }

    fn preview_copy_line_segments(
        raw_lines: &[String],
        joined_lines: &[String],
    ) -> Option<Vec<PreviewCopyLineSegment>> {
        let mut segments = Vec::with_capacity(raw_lines.len());
        let mut raw_index = 0usize;

        for (logical_line, joined_line) in joined_lines.iter().enumerate() {
            if raw_index >= raw_lines.len() {
                return None;
            }

            let mut remaining = joined_line.as_str();
            let mut logical_col_start = 0usize;

            loop {
                let raw_line = raw_lines.get(raw_index)?;
                if raw_line.is_empty() && !remaining.is_empty() {
                    return None;
                }
                if !remaining.starts_with(raw_line.as_str()) {
                    return None;
                }

                segments.push(PreviewCopyLineSegment {
                    logical_line,
                    logical_col_start,
                });
                logical_col_start =
                    logical_col_start.saturating_add(Self::preview_line_display_width(raw_line));
                remaining = &remaining[raw_line.len()..];
                raw_index = raw_index.saturating_add(1);

                if remaining.is_empty() {
                    break;
                }
                if raw_index >= raw_lines.len() {
                    return None;
                }
            }
        }

        (raw_index == raw_lines.len()).then_some(segments)
    }

    fn preview_copy_lines_from_joined_capture(&self) -> Option<Vec<String>> {
        let (start, end) = self.preview_copy_bounds()?;
        let joined_lines = self.capture_joined_preview_lines_for_copy()?;
        let segments =
            Self::preview_copy_line_segments(self.preview.lines.as_slice(), &joined_lines)?;
        let start_segment = *segments.get(start.line)?;
        let end_segment = *segments.get(end.line)?;
        if end_segment.logical_line < start_segment.logical_line {
            return None;
        }

        let start_col = start_segment.logical_col_start.saturating_add(start.col);
        let end_col = end_segment.logical_col_start.saturating_add(end.col);
        let start_line = start_segment
            .logical_line
            .min(joined_lines.len().saturating_sub(1));
        let end_line = end_segment
            .logical_line
            .min(joined_lines.len().saturating_sub(1));
        let mut lines = joined_lines[start_line..=end_line].to_vec();
        if lines.is_empty() {
            return None;
        }

        if lines.len() == 1 {
            lines[0] = Self::preview_substring_by_cells(&lines[0], start_col, Some(end_col));
            return Some(lines);
        }

        lines[0] = Self::preview_substring_by_cells(&lines[0], start_col, None);
        let last_idx = lines.len().saturating_sub(1);
        lines[last_idx] = Self::preview_substring_by_cells(&lines[last_idx], 0, Some(end_col));
        Some(lines)
    }

    fn preview_copy_lines_from_wrapped_raw_rows(&self) -> Option<Vec<String>> {
        let pane_width = self
            .session
            .interactive
            .as_ref()
            .map(|interactive| usize::from(interactive.pane_width.max(1)))
            .or_else(|| {
                self.preview_output_dimensions()
                    .map(|(width, _)| usize::from(width.max(1)))
            })?;
        let (start, end) = self.preview_copy_bounds()?;
        let start_line = start.line.min(self.preview.lines.len().saturating_sub(1));
        let end_line = end.line.min(self.preview.lines.len().saturating_sub(1));
        if end_line < start_line {
            return None;
        }

        let lines = self.preview_text_lines_from_bounds(start, end)?;
        if lines.len() <= 1 {
            return Some(lines);
        }

        let mut merged_lines: Vec<String> = Vec::with_capacity(lines.len());
        for (offset, line) in lines.into_iter().enumerate() {
            if offset > 0 {
                let previous_source_line = start_line.saturating_add(offset.saturating_sub(1));
                let should_join = self
                    .preview_plain_line(previous_source_line)
                    .map(|source_line| Self::preview_line_display_width(&source_line) >= pane_width)
                    .unwrap_or(false);
                if should_join && let Some(previous_line) = merged_lines.last_mut() {
                    previous_line.push_str(&line);
                    continue;
                }
            }
            merged_lines.push(line);
        }

        Some(merged_lines)
    }

    fn visible_preview_output_lines(&self) -> Vec<String> {
        let Some((_, output_height)) = self.preview_output_dimensions() else {
            return Vec::new();
        };
        let (visible_start, visible_end) =
            self.preview_visible_range_for_height(usize::from(output_height));
        self.preview_plain_lines_range(visible_start, visible_end)
    }

    fn reflow_preview_copy_lines(&self, lines: Vec<String>) -> Vec<String> {
        if !matches!(self.preview_tab, PreviewTab::Home | PreviewTab::Agent) {
            return lines;
        }

        let mut out = Vec::with_capacity(lines.len());
        let mut paragraph = String::new();
        let mut in_fence = false;

        for line in lines {
            let trimmed = line.trim();
            let trimmed_start = line.trim_start();
            let is_fence = trimmed_start.starts_with("```");
            if is_fence {
                if !paragraph.is_empty() {
                    out.push(std::mem::take(&mut paragraph));
                }
                out.push(line);
                in_fence = !in_fence;
                continue;
            }

            if in_fence || trimmed.is_empty() || Self::is_structured_copy_line(line.as_str()) {
                if !paragraph.is_empty() {
                    out.push(std::mem::take(&mut paragraph));
                }
                out.push(line);
                continue;
            }

            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }

        if !paragraph.is_empty() {
            out.push(paragraph);
        }

        out
    }

    fn is_structured_copy_line(line: &str) -> bool {
        let trimmed_start = line.trim_start();
        trimmed_start.starts_with('#')
            || trimmed_start.starts_with('>')
            || trimmed_start.starts_with("- ")
            || trimmed_start.starts_with("* ")
            || trimmed_start.starts_with("+ ")
            || trimmed_start.starts_with("|")
            || line.starts_with("    ")
            || line.starts_with('\t')
            || trimmed_start
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
                && trimmed_start.contains(". ")
    }

    pub(super) fn copy_interactive_selection_or_visible(&mut self) {
        let copied_from_selection = self.preview_selection.has_selection();
        let joined_lines = self.preview_copy_lines_from_joined_capture();
        let wrapped_raw_lines = self.preview_copy_lines_from_wrapped_raw_rows();
        let mut lines = match (joined_lines, wrapped_raw_lines) {
            (Some(joined_lines), Some(wrapped_raw_lines))
                if wrapped_raw_lines.len() < joined_lines.len() =>
            {
                wrapped_raw_lines
            }
            (Some(joined_lines), _) => joined_lines,
            (None, Some(wrapped_raw_lines)) => wrapped_raw_lines,
            (None, None) => self
                .selected_preview_text_lines()
                .unwrap_or_else(|| self.visible_preview_output_lines()),
        };
        lines = self.reflow_preview_copy_lines(lines);
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
