use super::view_prelude::*;

impl GroveApp {
    pub(super) fn clear_preview_selection(&mut self) {
        self.preview_selection.clear();
    }

    pub(super) fn preview_visible_range_for_height(&self, preview_height: usize) -> (usize, usize) {
        if preview_height == 0 {
            return (0, 0);
        }

        let total_lines = self.preview.lines.len();
        let mut preview_scroll = self.preview_scroll.borrow_mut();
        preview_scroll.set_external_len(total_lines);
        let viewport_height = u16::try_from(preview_height).unwrap_or(u16::MAX);
        let visible = preview_scroll.visible_range(viewport_height);
        (visible.start, visible.end)
    }

    pub(super) fn preview_content_viewport(&self) -> Option<PreviewContentViewport> {
        let (_, _, preview_rect) = self.effective_workspace_rects();
        if preview_rect.is_empty() {
            return None;
        }
        let inner = Block::new().borders(Borders::ALL).inner(preview_rect);
        if inner.is_empty() {
            return None;
        }

        let preview_height = usize::from(inner.height)
            .saturating_sub(usize::from(PREVIEW_METADATA_ROWS))
            .max(1);
        let (visible_start, visible_end) = self.preview_visible_range_for_height(preview_height);

        Some(PreviewContentViewport {
            output_x: inner.x,
            output_y: inner.y.saturating_add(PREVIEW_METADATA_ROWS),
            visible_start,
            visible_end,
        })
    }

    pub(super) fn preview_text_point_at(&self, x: u16, y: u16) -> Option<TextSelectionPoint> {
        if self.preview_tab == PreviewTab::Git {
            return None;
        }

        let viewport = self.preview_content_viewport()?;
        if y < viewport.output_y {
            return None;
        }

        let visible_row = usize::from(y - viewport.output_y);
        let visible_count = viewport.visible_end.saturating_sub(viewport.visible_start);
        if visible_row >= visible_count {
            return None;
        }

        let line_idx = viewport.visible_start.saturating_add(visible_row);
        let line = self.preview_plain_line(line_idx)?;
        let line_width = line_visual_width(&line);
        if x < viewport.output_x {
            return Some(TextSelectionPoint {
                line: line_idx,
                col: 0,
            });
        }

        let relative_x = usize::from(x - viewport.output_x);
        let col = if line_width == 0 {
            0
        } else {
            relative_x.min(line_width.saturating_sub(1))
        };

        Some(TextSelectionPoint {
            line: line_idx,
            col,
        })
    }

    pub(super) fn preview_plain_line(&self, line_idx: usize) -> Option<String> {
        self.preview
            .render_lines
            .get(line_idx)
            .map(|line| ansi_line_to_plain_text(line))
            .or_else(|| self.preview.lines.get(line_idx).cloned())
    }

    pub(super) fn preview_plain_lines_range(&self, start: usize, end: usize) -> Vec<String> {
        (start..end)
            .map_while(|line_idx| self.preview_plain_line(line_idx))
            .collect()
    }
}
