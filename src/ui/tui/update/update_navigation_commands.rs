use super::update_prelude::*;

impl GroveApp {
    const SIDEBAR_KEYBOARD_RESIZE_STEP_PCT: i16 = 2;

    pub(super) fn maybe_focus_attention_inbox_on_startup(&mut self) {
        if !self.startup_attention_focus_pending || self.attention_items.is_empty() {
            return;
        }
        self.focus_attention_inbox();
        self.startup_attention_focus_pending = false;
    }

    fn focus_attention_inbox(&mut self) {
        if self.attention_items.is_empty() {
            return;
        }

        if self.session.interactive.is_some() {
            self.exit_interactive_to_list();
        } else {
            reduce(&mut self.state, Action::EnterListMode);
        }
        self.sync_focus_manager_to_state();
        self.select_attention_item(0);
    }

    fn select_workspace_by_path(&mut self, workspace_path: &Path) {
        if let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == workspace_path)
        {
            let changed = self.state.select_index(workspace_index);
            if changed {
                self.handle_workspace_selection_changed();
            }
        }
    }

    pub(super) fn acknowledge_selected_attention_item(&mut self) {
        let Some((selected_index, workspace_path)) =
            self.selected_attention_item.and_then(|index| {
                self.selected_attention_item()
                    .map(|item| (index, item.workspace_path.clone()))
            })
        else {
            return;
        };
        self.clear_attention_for_workspace_path(workspace_path.as_path());
        if self.attention_items.get(selected_index).is_some() {
            self.select_attention_item(selected_index);
        } else {
            self.select_workspace_by_path(workspace_path.as_path());
        }
    }

    pub(super) fn clear_startup_attention_focus_pending(&mut self) {
        self.startup_attention_focus_pending = false;
    }

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
        let _ = self.panes.set_sidebar_ratio_pct(clamped);
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

    fn cycle_preview_tab(&mut self, direction: i8) {
        self.cycle_selected_workspace_tabs(direction);
        self.clear_preview_selection();
        self.refresh_preview_summary();
        self.poll_preview();
    }

    fn reorder_preview_tab(&mut self, direction: i8) {
        if self.move_selected_workspace_tab_by(direction) {
            self.clear_preview_selection();
            self.poll_preview();
        }
    }

    fn toggle_mouse_capture(&mut self) {
        self.mouse_capture_enabled = !self.mouse_capture_enabled;
        let _ = self.divider_resize.force_cancel();
        self.divider_resize_anchor_x = 0;
        self.queue_cmd(Cmd::set_mouse_capture(self.mouse_capture_enabled));
        if self.mouse_capture_enabled {
            self.show_info_toast("mouse capture enabled");
        } else {
            self.show_info_toast("mouse capture disabled, terminal URL clicks restored");
        }
    }

    pub(super) fn execute_ui_command(&mut self, command: UiCommand) -> bool {
        match command {
            UiCommand::ToggleFocus => {
                let mode_before = self.state.mode;
                let focus_before = self.state.focus;
                reduce(&mut self.state, Action::ToggleFocus);
                self.sync_focus_manager_to_state();
                if self.state.mode != mode_before || self.state.focus != focus_before {
                    self.acknowledge_selected_workspace_attention_for_preview_focus();
                }
            }
            UiCommand::ToggleSidebar => {
                self.sidebar_hidden = !self.sidebar_hidden;
                if self.sidebar_hidden {
                    let _ = self.divider_resize.force_cancel();
                    self.divider_resize_anchor_x = 0;
                }
            }
            UiCommand::OpenPreview => {
                self.enter_preview_or_interactive();
            }
            UiCommand::EnterInteractive => {
                self.enter_interactive(Instant::now());
            }
            UiCommand::FocusPreview => {
                let mode_before = self.state.mode;
                let focus_before = self.state.focus;
                reduce(&mut self.state, Action::EnterPreviewMode);
                self.sync_focus_manager_to_state();
                if self.state.mode != mode_before || self.state.focus != focus_before {
                    self.acknowledge_selected_workspace_attention_for_preview_focus();
                    self.poll_preview();
                }
            }
            UiCommand::FocusList => {
                reduce(&mut self.state, Action::EnterListMode);
                self.sync_focus_manager_to_state();
            }
            UiCommand::MoveSelectionUp => {
                self.move_selection(Action::MoveSelectionUp);
            }
            UiCommand::MoveSelectionDown => {
                self.move_selection(Action::MoveSelectionDown);
            }
            UiCommand::ScrollUp => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(-1);
                }
            }
            UiCommand::ScrollDown => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(1);
                }
            }
            UiCommand::PageUp => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(-self.preview_page_scroll_delta());
                }
            }
            UiCommand::PageDown => {
                if self.preview_scroll_tab_is_focused() {
                    self.scroll_preview(self.preview_page_scroll_delta());
                }
            }
            UiCommand::ScrollBottom => {
                if self.preview_scroll_tab_is_focused() {
                    self.jump_preview_to_bottom();
                }
            }
            UiCommand::PreviousTab => {
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(-1);
                }
            }
            UiCommand::NextTab => {
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(1);
                }
            }
            UiCommand::MoveTabLeft => {
                self.reorder_preview_tab(-1);
            }
            UiCommand::MoveTabRight => {
                self.reorder_preview_tab(1);
            }
            UiCommand::ResizeSidebarNarrower => {
                self.resize_sidebar_by_keyboard(-Self::SIDEBAR_KEYBOARD_RESIZE_STEP_PCT);
            }
            UiCommand::ResizeSidebarWider => {
                self.resize_sidebar_by_keyboard(Self::SIDEBAR_KEYBOARD_RESIZE_STEP_PCT);
            }
            UiCommand::NewWorkspace => {
                self.open_create_dialog();
            }
            UiCommand::AddWorktree => {
                self.open_add_worktree_dialog();
            }
            UiCommand::EditWorkspace => {
                self.open_edit_dialog();
            }
            UiCommand::StartAgent => {
                self.open_start_dialog();
            }
            UiCommand::StartParentAgent => {
                self.open_start_parent_agent_dialog();
            }
            UiCommand::OpenShellTab => {
                self.open_new_shell_tab();
            }
            UiCommand::OpenGitTab => {
                self.open_or_focus_git_tab();
            }
            UiCommand::OpenDiffTab => {
                self.open_or_focus_diff_tab();
            }
            UiCommand::RenameActiveTab => {
                self.open_rename_tab_dialog();
            }
            UiCommand::StopAgent => {
                self.kill_active_tab_session();
            }
            UiCommand::RestartAgent => {
                self.close_active_tab_or_confirm();
            }
            UiCommand::DeleteWorkspace => {
                self.open_delete_task_dialog();
            }
            UiCommand::DeleteWorktree => {
                self.open_delete_worktree_dialog();
            }
            UiCommand::MergeWorkspace => {
                self.open_merge_dialog();
            }
            UiCommand::UpdateFromBase => {
                self.open_update_from_base_dialog();
            }
            UiCommand::PullUpstream => {
                self.open_pull_upstream_dialog();
            }
            UiCommand::RefreshWorkspaces => {
                self.request_manual_workspace_refresh();
            }
            UiCommand::OpenProjects => {
                self.open_project_dialog();
            }
            UiCommand::ReorderTasks => {
                self.open_task_reorder_mode();
            }
            UiCommand::DeleteProject => {
                self.delete_selected_workspace_project();
            }
            UiCommand::OpenSettings => {
                self.open_settings_dialog();
            }
            UiCommand::ToggleMouseCapture => {
                self.toggle_mouse_capture();
            }
            UiCommand::ToggleUnsafe => {
                self.launch_permission_mode = self.launch_permission_mode.next_global();
            }
            UiCommand::FocusAttentionInbox => {
                self.focus_attention_inbox();
            }
            UiCommand::AcknowledgeAttention => {
                self.acknowledge_selected_attention_item();
            }
            UiCommand::CleanupSessions => {
                self.open_session_cleanup_dialog();
            }
            UiCommand::OpenHelp => {
                self.open_keybind_help();
            }
            UiCommand::OpenCommandPalette => {
                self.open_command_palette();
            }
            UiCommand::Quit => {
                self.open_quit_dialog();
            }
            UiCommand::OpenPerformance => {
                self.open_performance_dialog();
            }
        }

        false
    }
}
