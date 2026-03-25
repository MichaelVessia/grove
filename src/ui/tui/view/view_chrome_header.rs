use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_header(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = self.active_ui_theme();
        let base_style = Style::new()
            .bg(packed(theme.overlay))
            .fg(packed(theme.text));
        let mut line = StatusLine::new()
            .style(base_style)
            .separator("  ")
            .left(StatusItem::text("[Grove]"))
            .left(StatusItem::text(self.repo_name.as_str()));
        if self.dialogs.command_palette.is_visible() {
            line = line.left(StatusItem::text("[Palette]"));
        }

        line.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_HEADER));
    }
}
