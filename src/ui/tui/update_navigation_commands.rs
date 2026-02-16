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

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                if workspace.is_main && !workspace.status.has_session() {
                    return self.main_worktree_splash();
                }
                format!(
                    "Workspace: {}\nBranch: {}\nPath: {}\nAgent: {}\nOrphaned session: {}",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display(),
                    workspace.agent.label(),
                    if workspace.is_orphaned { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn main_worktree_splash(&self) -> String {
        const G: &str = "\x1b[38;2;166;227;161m";
        const T: &str = "\x1b[38;2;250;179;135m";
        const R: &str = "\x1b[0m";

        [
            String::new(),
            format!("{G}                    .@@@.{R}"),
            format!("{G}                 .@@@@@@@@@.{R}"),
            format!("{G}               .@@@@@@@@@@@@@.{R}"),
            format!("{G}    .@@@.     @@@@@@@@@@@@@@@@@        .@@.{R}"),
            format!("{G}  .@@@@@@@.  @@@@@@@@@@@@@@@@@@@    .@@@@@@@@.{R}"),
            format!("{G} @@@@@@@@@@@ @@@@@@@@@@@@@@@@@@@@  @@@@@@@@@@@@@{R}"),
            format!("{G} @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@'{R}"),
            format!("{G}    '@@@@@@@@  '@@@@@@@@@@@@@@@' @@@@@@@@@@@@@@'{R}"),
            format!("{G}      '@@@@'     '@@@@@@@@@@@'    '@@@@@@@@@@'{R}"),
            format!("         {T}||{R}        {G}'@@@@@@@'{R}        {G}'@@@@'{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("        {T}/||\\{R}         {T}/|||\\{R}            {T}/||\\{R}"),
            String::new(),
            "Base Worktree".to_string(),
            String::new(),
            "This is your repo root.".to_string(),
            "Create focused workspaces from here when you start new work.".to_string(),
            String::new(),
            "--------------------------------------------------".to_string(),
            String::new(),
            "Press 'n' to create a workspace".to_string(),
            String::new(),
            "Each workspace has its own directory and branch.".to_string(),
            "Run agents in parallel without branch hopping.".to_string(),
        ]
        .join("\n")
    }

    fn has_non_palette_modal_open(&self) -> bool {
        self.launch_dialog.is_some()
            || self.create_dialog.is_some()
            || self.edit_dialog.is_some()
            || self.delete_dialog.is_some()
            || self.merge_dialog.is_some()
            || self.update_from_base_dialog.is_some()
            || self.settings_dialog.is_some()
            || self.project_dialog.is_some()
            || self.keybind_help_open
    }

    fn can_open_command_palette(&self) -> bool {
        !self.has_non_palette_modal_open() && self.interactive.is_none()
    }

    fn palette_action(
        id: &'static str,
        title: &'static str,
        description: &'static str,
        tags: &[&str],
        category: &'static str,
    ) -> PaletteActionItem {
        PaletteActionItem::new(id, title)
            .with_description(description)
            .with_tags(tags)
            .with_category(category)
    }

    pub(super) fn build_command_palette_actions(&self) -> Vec<PaletteActionItem> {
        let mut actions = Vec::new();
        for command in UiCommand::all() {
            if !self.palette_command_enabled(*command) {
                continue;
            }
            let Some(spec) = command.palette_spec() else {
                continue;
            };
            actions.push(Self::palette_action(
                spec.id,
                spec.title,
                spec.description,
                spec.tags,
                spec.category,
            ));
        }
        actions
    }

    fn refresh_command_palette_actions(&mut self) {
        self.command_palette
            .replace_actions(self.build_command_palette_actions());
    }

    pub(super) fn open_command_palette(&mut self) {
        if !self.can_open_command_palette() {
            return;
        }

        self.refresh_command_palette_actions();
        self.command_palette.open();
    }

    fn palette_command_enabled(&self, command: UiCommand) -> bool {
        if command.palette_spec().is_none() {
            return false;
        }
        match command {
            UiCommand::ToggleFocus
            | UiCommand::ToggleSidebar
            | UiCommand::NewWorkspace
            | UiCommand::EditWorkspace
            | UiCommand::OpenProjects
            | UiCommand::OpenSettings
            | UiCommand::ToggleUnsafe
            | UiCommand::OpenHelp
            | UiCommand::Quit => true,
            UiCommand::OpenPreview => self.state.focus == PaneFocus::WorkspaceList,
            UiCommand::EnterInteractive => {
                self.state.focus == PaneFocus::Preview
                    && workspace_can_enter_interactive(
                        self.state.selected_workspace(),
                        self.preview_tab == PreviewTab::Git,
                    )
            }
            UiCommand::FocusList => self.state.focus == PaneFocus::Preview,
            UiCommand::MoveSelectionUp | UiCommand::MoveSelectionDown => {
                self.state.focus == PaneFocus::WorkspaceList
            }
            UiCommand::ScrollUp
            | UiCommand::ScrollDown
            | UiCommand::PageUp
            | UiCommand::PageDown
            | UiCommand::ScrollBottom => self.preview_agent_tab_is_focused(),
            UiCommand::PreviousTab | UiCommand::NextTab => {
                self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview
            }
            UiCommand::StartAgent => {
                self.preview_agent_tab_is_focused()
                    && !self.start_in_flight
                    && workspace_can_start_agent(self.state.selected_workspace())
            }
            UiCommand::StopAgent => {
                self.preview_agent_tab_is_focused()
                    && !self.stop_in_flight
                    && workspace_can_stop_agent(self.state.selected_workspace())
            }
            UiCommand::DeleteWorkspace => {
                !self.delete_in_flight
                    && self
                        .state
                        .selected_workspace()
                        .is_some_and(|workspace| !workspace.is_main)
            }
            UiCommand::MergeWorkspace => {
                !self.merge_in_flight
                    && self
                        .state
                        .selected_workspace()
                        .is_some_and(|workspace| !workspace.is_main)
            }
            UiCommand::UpdateFromBase => {
                !self.update_from_base_in_flight
                    && self
                        .state
                        .selected_workspace()
                        .is_some_and(|workspace| !workspace.is_main)
            }
            UiCommand::FocusPreview | UiCommand::OpenCommandPalette => false,
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

    pub(super) fn execute_command_palette_action(&mut self, id: &str) -> bool {
        let Some(command) = UiCommand::from_palette_id(id) else {
            return false;
        };
        self.execute_ui_command(command)
    }

    pub(super) fn modal_open(&self) -> bool {
        self.has_non_palette_modal_open() || self.command_palette.is_visible()
    }

    pub(super) fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }
}
