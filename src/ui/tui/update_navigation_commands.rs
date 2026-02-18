use super::*;

impl GroveApp {
    const SIDEBAR_KEYBOARD_RESIZE_STEP_PCT: i16 = 2;

    fn preview_page_scroll_delta(&self) -> i32 {
        let viewport_height = self
            .preview_output_dimensions()
            .map_or(1, |(_, height)| i32::from(height));
        viewport_height.saturating_sub(1).max(1)
    }

    pub(super) fn set_sidebar_ratio(&mut self, ratio: u16) {
        let clamped = clamp_sidebar_ratio(ratio);
        if clamped == self.sidebar_width_pct {
            return;
        }

        self.sidebar_width_pct = clamped;
        self.persist_sidebar_ratio();
        self.sync_interactive_session_geometry();
    }

    fn resize_sidebar_by_keyboard(&mut self, delta_pct: i16) {
        if self.sidebar_hidden {
            return;
        }

        let next = i32::from(self.sidebar_width_pct).saturating_add(i32::from(delta_pct));
        let min_pct = i32::from(clamp_sidebar_ratio(0));
        let max_pct = i32::from(clamp_sidebar_ratio(u16::MAX));
        let bounded = next.clamp(min_pct, max_pct);
        let Some(ratio) = u16::try_from(bounded).ok() else {
            return;
        };
        self.set_sidebar_ratio(ratio);
    }

    pub(super) fn select_preview_tab(&mut self, next_tab: PreviewTab) {
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
        if matches!(self.preview_tab, PreviewTab::Agent | PreviewTab::Shell)
            && let Some(workspace) = self.state.selected_workspace()
        {
            let session_name = shell_session_name_for_workspace(workspace);
            self.shell_failed_sessions.remove(&session_name);
        }
        self.poll_preview();
    }

    fn cycle_preview_tab(&mut self, direction: i8) {
        let next_tab = if direction.is_negative() {
            self.preview_tab.previous()
        } else {
            self.preview_tab.next()
        };
        self.select_preview_tab(next_tab);
    }

    fn toggle_mouse_capture(&mut self) {
        self.mouse_capture_enabled = !self.mouse_capture_enabled;
        self.divider_drag_active = false;
        self.divider_drag_pointer_offset = 0;
        self.queue_cmd(Cmd::set_mouse_capture(self.mouse_capture_enabled));
        if self.mouse_capture_enabled {
            self.show_toast("mouse capture enabled", false);
        } else {
            self.show_toast(
                "mouse capture disabled, terminal URL clicks restored",
                false,
            );
        }
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
                    self.divider_drag_pointer_offset = 0;
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
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(-1);
                }
                false
            }
            UiCommand::ScrollDown => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(1);
                }
                false
            }
            UiCommand::PageUp => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(-self.preview_page_scroll_delta());
                }
                false
            }
            UiCommand::PageDown => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(self.preview_page_scroll_delta());
                }
                false
            }
            UiCommand::ScrollBottom => {
                if self.preview_scroll_tab_is_focused() {
                    self.jump_preview_to_bottom();
                }
                false
            }
            UiCommand::PreviousTab => {
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(-1);
                }
                false
            }
            UiCommand::NextTab => {
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(1);
                }
                false
            }
            UiCommand::ResizeSidebarNarrower => {
                self.resize_sidebar_by_keyboard(-Self::SIDEBAR_KEYBOARD_RESIZE_STEP_PCT);
                false
            }
            UiCommand::ResizeSidebarWider => {
                self.resize_sidebar_by_keyboard(Self::SIDEBAR_KEYBOARD_RESIZE_STEP_PCT);
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
            UiCommand::ToggleMouseCapture => {
                self.toggle_mouse_capture();
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
