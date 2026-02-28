use super::*;

impl GroveApp {
    pub(super) fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let preview_focused = self.state.focus == PaneFocus::Preview && !self.modal_open();
        let interactive_input_active = self.interactive.is_some() && !self.modal_open();
        let theme = ui_theme();
        let (title, border_style) = if interactive_input_active {
            ("Preview · INSERT", Style::new().fg(theme.teal).bold())
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
        let selected_agent = selected_workspace.map(|workspace| workspace.agent);
        let allow_cursor_overlay = self.preview_tab != PreviewTab::Agent
            || match selected_agent {
                Some(agent) => agent.allows_cursor_overlay(),
                None => true,
            };

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
            allow_cursor_overlay,
            &visible_plain_lines,
            visible_start,
            visible_end,
            preview_height,
        ));

        let paragraph = Paragraph::new(FtText::from_lines(text_lines));
        if self.preview_tab == PreviewTab::Agent {
            paragraph.render(inner, frame);
        } else {
            paragraph
                .style(Style::new().bg(ui_theme().base))
                .render(inner, frame);
        }
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
        self.apply_preview_selection_highlight_cells(
            frame,
            inner,
            &visible_plain_lines,
            visible_start,
        );
    }
}
