use super::*;

impl GroveApp {
    fn cycle_preview_tab(&mut self, direction: i8) {
        let next_tab = if direction.is_negative() {
            self.preview_tab.previous()
        } else {
            self.preview_tab.next()
        };
        if next_tab == self.preview_tab {
            return;
        }

        self.preview_tab = next_tab;
        self.clear_preview_selection();
        if self.preview_tab == PreviewTab::Git
            && let Some(workspace) = self.state.selected_workspace()
        {
            let session_name = git_session_name_for_workspace(workspace);
            self.lazygit_failed_sessions.remove(&session_name);
        }
        self.poll_preview();
    }
    pub(super) fn execute_ui_command(&mut self, command: UiCommand) -> bool {
        match command {
            UiCommand::ToggleFocus => {
                reduce(&mut self.state, Action::ToggleFocus);
                false
            }
            UiCommand::ToggleSidebar => {
                self.sidebar_hidden = !self.sidebar_hidden;
                if self.sidebar_hidden {
                    self.divider_drag_active = false;
                }
                false
            }
            UiCommand::OpenPreview => {
                self.enter_preview_or_interactive();
                false
            }
            UiCommand::EnterInteractive => {
                self.enter_interactive(Instant::now());
                false
            }
            UiCommand::FocusPreview => {
                let mode_before = self.state.mode;
                let focus_before = self.state.focus;
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode != mode_before || self.state.focus != focus_before {
                    self.poll_preview();
                }
                false
            }
            UiCommand::FocusList => {
                reduce(&mut self.state, Action::EnterListMode);
                false
            }
            UiCommand::MoveSelectionUp => {
                self.move_selection(Action::MoveSelectionUp);
                false
            }
            UiCommand::MoveSelectionDown => {
                self.move_selection(Action::MoveSelectionDown);
                false
            }
            UiCommand::ScrollUp => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(-1);
                }
                false
            }
            UiCommand::ScrollDown => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(1);
                }
                false
            }
            UiCommand::PageUp => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(-5);
                }
                false
            }
            UiCommand::PageDown => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(5);
                }
                false
            }
            UiCommand::ScrollBottom => {
                if self.preview_agent_tab_is_focused() {
                    self.jump_preview_to_bottom();
                }
                false
            }
            UiCommand::PreviousTab => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(-1);
                }
                false
            }
            UiCommand::NextTab => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(1);
                }
                false
            }
            UiCommand::NewWorkspace => {
                self.open_create_dialog();
                false
            }
            UiCommand::EditWorkspace => {
                self.open_edit_dialog();
                false
            }
            UiCommand::StartAgent => {
                if self.preview_agent_tab_is_focused() {
                    self.open_start_dialog();
                }
                false
            }
            UiCommand::StopAgent => {
                if self.preview_agent_tab_is_focused() {
                    self.stop_selected_workspace_agent();
                }
                false
            }
            UiCommand::DeleteWorkspace => {
                self.open_delete_dialog();
                false
            }
            UiCommand::MergeWorkspace => {
                self.open_merge_dialog();
                false
            }
            UiCommand::UpdateFromBase => {
                self.open_update_from_base_dialog();
                false
            }
            UiCommand::OpenProjects => {
                self.open_project_dialog();
                false
            }
            UiCommand::DeleteProject => {
                self.delete_selected_workspace_project();
                false
            }
            UiCommand::OpenSettings => {
                self.open_settings_dialog();
                false
            }
            UiCommand::ToggleUnsafe => {
                self.launch_skip_permissions = !self.launch_skip_permissions;
                false
            }
            UiCommand::OpenHelp => {
                self.open_keybind_help();
                false
            }
            UiCommand::OpenCommandPalette => {
                self.open_command_palette();
                false
            }
            UiCommand::Quit => true,
        }
    }
}
