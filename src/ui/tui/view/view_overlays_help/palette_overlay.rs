impl GroveApp {
    pub(super) fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        self.dialogs.command_palette.render(area, frame);
    }
}
