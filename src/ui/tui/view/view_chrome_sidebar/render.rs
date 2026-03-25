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

        let theme = self.active_ui_theme();

        if self.projects.is_empty() {
            Paragraph::new(FtText::from_lines(vec![
                FtLine::from_spans(vec![FtSpan::styled(
                    "No projects configured",
                    Style::new().fg(packed(theme.text_subtle)),
                )]),
                FtLine::raw(""),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press Ctrl+K for command palette",
                    Style::new().fg(packed(theme.text)).bold(),
                )]),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Type help",
                    Style::new().fg(packed(theme.text)),
                )]),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press p to add a project",
                    Style::new().fg(packed(theme.text)),
                )]),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press n to create a task",
                    Style::new().fg(packed(theme.text)),
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
                        Style::new().fg(packed(theme.error)).bold(),
                    )]),
                    FtLine::from_spans(vec![FtSpan::styled(
                        message.as_str(),
                        Style::new().fg(packed(theme.accent)),
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

    }
}
