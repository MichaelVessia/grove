use super::*;

impl GroveApp {
    fn preview_is_focused(&self) -> bool {
        self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview
    }

    pub(super) fn preview_agent_tab_is_focused(&self) -> bool {
        self.preview_is_focused() && self.preview_tab == PreviewTab::Agent
    }

    pub(super) fn preview_scroll_tab_is_focused(&self) -> bool {
        self.preview_is_focused()
            && matches!(self.preview_tab, PreviewTab::Agent | PreviewTab::Shell)
    }

    pub(super) fn open_keybind_help(&mut self) {
        if self.modal_open() {
            return;
        }
        self.keybind_help_open = true;
    }

    pub(super) fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.handle_workspace_selection_changed();
        }
    }

    pub(super) fn handle_workspace_selection_changed(&mut self) {
        if self.interactive.is_some() {
            self.exit_interactive_to_list();
        }
        self.preview.jump_to_bottom();
        self.clear_agent_activity_tracking();
        self.clear_preview_selection();
        if self.selected_live_preview_session_if_ready().is_none() {
            self.refresh_preview_summary();
        }
        self.poll_preview_prioritized();
    }
}
