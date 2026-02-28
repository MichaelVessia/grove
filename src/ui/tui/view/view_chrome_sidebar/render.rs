impl GroveApp {
    pub(super) fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Workspaces")
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(
                self.state.focus == PaneFocus::WorkspaceList && !self.modal_open(),
            ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_WORKSPACE_LIST));

        if inner.is_empty() {
            return;
        }

        let theme = ui_theme();

        if self.projects.is_empty() {
            Paragraph::new(FtText::from_lines(vec![
                FtLine::from_spans(vec![FtSpan::styled(
                    "No projects configured",
                    Style::new().fg(theme.subtext0),
                )]),
                FtLine::raw(""),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press 'p' to add a project",
                    Style::new().fg(theme.text).bold(),
                )]),
            ]))
            .render(inner, frame);
            return;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                Paragraph::new(FtText::from_lines(vec![
                    FtLine::from_spans(vec![FtSpan::styled(
                        "Discovery error",
                        Style::new().fg(theme.red).bold(),
                    )]),
                    FtLine::from_spans(vec![FtSpan::styled(
                        message.as_str(),
                        Style::new().fg(theme.peach),
                    )]),
                ]))
                .render(inner, frame);
            }
            return;
        }

        let (lines, selected_line) = self.build_sidebar_lines(theme);
        if lines.is_empty() {
            return;
        }

        let mut list_state = self.sidebar_list_state.borrow_mut();
        if selected_line.is_some_and(|line| line <= 1) && inner.height > 1 {
            list_state.scroll_to_top();
        }
        list_state.select(selected_line);
        let list = VirtualizedList::new(lines.as_slice())
            .fixed_height(1)
            .show_scrollbar(true)
            .highlight_style(Style::new());
        ftui::widgets::StatefulWidget::render(&list, inner, frame, &mut *list_state);

        let scroll_offset = list_state.scroll_offset();
        let visible_count = list_state.visible_count();
        drop(list_state);

        let row_width = if lines.len() > visible_count {
            inner.width.saturating_sub(1)
        } else {
            inner.width
        };
        let content_x = inner.x.saturating_add(2);
        let content_width = row_width.saturating_sub(4);
        if content_width == 0 {
            return;
        }
        let max_x = content_x.saturating_add(content_width);
        let visible_end = scroll_offset
            .saturating_add(usize::from(inner.height))
            .min(lines.len());

        for (row_index, line) in lines
            .iter()
            .enumerate()
            .take(visible_end)
            .skip(scroll_offset)
        {
            let Some(activity) = line.activity() else {
                continue;
            };
            let Some(y_offset) = u16::try_from(row_index.saturating_sub(scroll_offset)).ok() else {
                continue;
            };
            let y = inner.y.saturating_add(y_offset);
            if y >= inner.bottom() {
                continue;
            }
            let Some(start_col) = u16::try_from(activity.start_col).ok() else {
                continue;
            };
            let x = content_x.saturating_add(start_col);
            if x >= max_x {
                continue;
            }
            let width = max_x.saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(
                activity.label.as_str(),
                activity.agent,
                Rect::new(x, y, width, 1),
                frame,
            );
        }
    }
}
