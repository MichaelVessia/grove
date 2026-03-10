use ftui::widgets::help::KeybindingHints;

#[derive(Debug, Clone)]
struct KeybindHelpModalContent {
    hints: KeybindingHints,
    theme: UiTheme,
}

impl Widget for KeybindHelpModalContent {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new().bg(self.theme.base).fg(self.theme.text);
        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title("Keybind Help")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(self.theme.blue).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let rows = Flex::vertical()
            .constraints([Constraint::Min(1), Constraint::Fixed(1)])
            .split(inner);

        Widget::render(&self.hints, rows[0], frame);
        Paragraph::new("Close help: Esc, Enter, or ?")
            .style(Style::new().fg(self.theme.lavender).bg(self.theme.base).bold())
            .render(rows[1], frame);
    }
}

impl GroveApp {
    pub(super) fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.dialogs.keybind_help_open {
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
        let theme = self.active_ui_theme();
        let content = KeybindHelpModalContent {
            hints: self.build_keybind_help_hints(),
            theme,
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
