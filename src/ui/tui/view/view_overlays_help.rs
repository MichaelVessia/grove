use super::*;

impl GroveApp {
    const KEYBIND_HELP_MIN_WIDTH: u16 = 56;
    const KEYBIND_HELP_MIN_HEIGHT: u16 = 16;
    const KEYBIND_HELP_HORIZONTAL_MARGIN: u16 = 1;
    const KEYBIND_HELP_VERTICAL_MARGIN: u16 = 0;
    const COMMAND_PALETTE_MIN_WIDTH: u16 = 44;
    const COMMAND_PALETTE_HORIZONTAL_MARGIN: u16 = 2;

    pub(super) fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }
}

include!("view_overlays_help/rows.rs");
include!("view_overlays_help/palette_overlay.rs");
include!("view_overlays_help/keybind_overlay.rs");
