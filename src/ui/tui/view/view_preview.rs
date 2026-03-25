use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let preview_focused = self.state.focus == PaneFocus::Preview && !self.modal_open();
        let interactive_input_active = self.session.interactive.is_some() && !self.modal_open();
        let theme = self.active_ui_theme();
        let (title, border_style) = if interactive_input_active {
            (
                "Preview · INSERT",
                Style::new().fg(packed(theme.info)).bold(),
            )
        } else {
            ("Preview", self.pane_border_style(preview_focused))
        };
        let block = Block::new()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_PREVIEW));

        if inner.is_empty() {
            return;
        }

        let selected_workspace = self.state.selected_workspace();

        let metadata_rows = usize::from(PREVIEW_METADATA_ROWS);
        let preview_height = usize::from(inner.height)
            .saturating_sub(metadata_rows)
            .max(1);

        let (mut text_lines, animated_labels) =
            self.preview_metadata_lines_and_labels(inner, selected_workspace);

        let visible_range = self.preview_visible_range_for_height(preview_height);
        let visible_start = visible_range.0;
        let visible_end = visible_range.1;
        let visible_plain_lines = self.preview_plain_lines_range(visible_start, visible_end);

        text_lines.extend(self.preview_tab_content_lines(
            selected_workspace,
            true,
            &visible_plain_lines,
            visible_start,
            visible_end,
            preview_height,
        ));

        Paragraph::new(FtText::from_lines(text_lines))
            .style(
                Style::new()
                    .fg(packed(theme.text))
                    .bg(packed(theme.background)),
            )
            .render(inner, frame);
        for (label, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_preview_activity_effect_label(&label, Rect::new(x, y, width, 1), frame);
        }
        self.apply_preview_selection_highlight_cells(
            frame,
            inner,
            &visible_plain_lines,
            visible_start,
        );
        if interactive_input_active {
            let output_x = inner.x;
            let output_y = inner.y.saturating_add(PREVIEW_METADATA_ROWS);
            if let Some((cursor_x, cursor_y)) =
                self.interactive_cursor_screen_position(output_x, output_y, preview_height)
                && cursor_x < inner.right()
                && cursor_y < inner.bottom()
            {
                frame.set_cursor(Some((cursor_x, cursor_y)));
                frame.set_cursor_visible(true);
            }
        }
    }
}
