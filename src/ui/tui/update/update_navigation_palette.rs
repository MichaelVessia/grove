use super::update_prelude::*;

impl GroveApp {
    const COMMAND_PALETTE_MAX_VISIBLE_ROWS: usize = 30;

    fn command_palette_style(&self) -> PaletteStyle {
        let theme = self.active_ui_theme();
        PaletteStyle {
            border: Style::new().fg(theme.blue).bg(theme.base).bold(),
            input: Style::new().fg(theme.text).bg(theme.base),
            item: Style::new().fg(theme.subtext0).bg(theme.base),
            item_selected: Style::new().fg(theme.text).bg(theme.surface0).bold(),
            match_highlight: Style::new().fg(theme.yellow).bg(theme.base).bold(),
            description: Style::new().fg(theme.overlay0).bg(theme.base),
            category: Style::new().fg(theme.blue).bg(theme.base).bold(),
            hint: Style::new().fg(theme.overlay0).bg(theme.base),
        }
    }

    fn has_non_palette_modal_open(&self) -> bool {
        self.dialogs.active_dialog.is_some() || self.dialogs.keybind_help_open
    }

    fn can_open_command_palette(&self) -> bool {
        !self.has_non_palette_modal_open() && self.session.interactive.is_none()
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
        self.dialogs
            .command_palette
            .replace_actions(self.build_command_palette_actions());
    }

    pub(super) fn command_palette_max_visible_for_height(viewport_height: u16) -> usize {
        let top_offset = viewport_height / 6;
        usize::from(
            viewport_height
                .saturating_sub(top_offset)
                .saturating_sub(3)
                .max(1),
        )
        .min(Self::COMMAND_PALETTE_MAX_VISIBLE_ROWS)
    }

    fn command_palette_max_visible(&self) -> usize {
        Self::command_palette_max_visible_for_height(self.viewport_height)
    }

    pub(super) fn open_command_palette(&mut self) {
        if !self.can_open_command_palette() {
            return;
        }

        self.dialogs.command_palette = CommandPalette::new()
            .with_max_visible(self.command_palette_max_visible())
            .with_style(self.command_palette_style());
        self.refresh_command_palette_actions();
        self.dialogs.command_palette.open();
    }

    fn palette_command_enabled(&self, command: UiCommand) -> bool {
        if command.palette_spec().is_none() {
            return false;
        }
        match command {
            UiCommand::ToggleFocus
            | UiCommand::ToggleSidebar
            | UiCommand::ResizeSidebarNarrower
            | UiCommand::ResizeSidebarWider
            | UiCommand::NewWorkspace
            | UiCommand::EditWorkspace
            | UiCommand::OpenProjects
            | UiCommand::ReorderTasks
            | UiCommand::OpenSettings
            | UiCommand::ToggleMouseCapture
            | UiCommand::ToggleUnsafe
            | UiCommand::CleanupSessions
            | UiCommand::OpenHelp
            | UiCommand::Quit => true,
            UiCommand::OpenPreview => self.state.focus == PaneFocus::WorkspaceList,
            UiCommand::EnterInteractive => {
                self.state.focus == PaneFocus::Preview && self.can_enter_interactive_session()
            }
            UiCommand::FocusList => self.state.focus == PaneFocus::Preview,
            UiCommand::MoveSelectionUp | UiCommand::MoveSelectionDown => true,
            UiCommand::ScrollUp
            | UiCommand::ScrollDown
            | UiCommand::PageUp
            | UiCommand::PageDown
            | UiCommand::ScrollBottom => self.preview_scroll_tab_is_focused(),
            UiCommand::PreviousTab | UiCommand::NextTab => {
                self.state.selected_workspace().is_some()
            }
            UiCommand::AddWorktree => {
                self.state.focus == PaneFocus::WorkspaceList
                    && self
                        .state
                        .selected_task()
                        .is_some_and(|task| !task.has_base_worktree())
            }
            UiCommand::StartAgent => {
                !self.dialogs.start_in_flight
                    && !self.dialogs.restart_in_flight
                    && self.state.selected_workspace().is_some()
                    && self.state.focus == PaneFocus::Preview
            }
            UiCommand::StartParentAgent => {
                !self.dialogs.start_in_flight
                    && !self.dialogs.restart_in_flight
                    && self.selected_home_tab_targets_task_root()
            }
            UiCommand::OpenShellTab => self.state.selected_workspace().is_some(),
            UiCommand::OpenGitTab => self.state.selected_workspace().is_some(),
            UiCommand::RenameActiveTab => self
                .selected_active_tab()
                .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home),
            UiCommand::StopAgent => self.active_tab_session_name().is_some(),
            UiCommand::RestartAgent => {
                self.state.selected_workspace().is_some()
                    && self
                        .selected_active_tab()
                        .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home)
            }
            UiCommand::DeleteWorkspace => self.state.selected_task().is_some_and(|task| {
                self.state.focus == PaneFocus::WorkspaceList && !self.task_delete_requested(task)
            }),
            UiCommand::DeleteWorktree => self.state.selected_worktree().is_some_and(|worktree| {
                self.state.focus == PaneFocus::WorkspaceList
                    && !self.workspace_delete_requested(worktree.path.as_path())
            }),
            UiCommand::DeleteProject => {
                !self.dialogs.project_delete_in_flight
                    && self.state.selected_workspace().is_some_and(|workspace| {
                        workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|workspace_path| {
                                self.projects.iter().any(|project| {
                                    refer_to_same_location(&project.path, workspace_path)
                                })
                            })
                    })
            }
            UiCommand::MergeWorkspace => {
                !self.dialogs.merge_in_flight
                    && self
                        .state
                        .selected_workspace()
                        .is_some_and(|workspace| !workspace.is_main)
            }
            UiCommand::UpdateFromBase => {
                !self.dialogs.update_from_base_in_flight
                    && self.state.selected_workspace().is_some()
            }
            UiCommand::PullUpstream => {
                !self.dialogs.pull_upstream_in_flight
                    && self
                        .state
                        .selected_workspace()
                        .is_some_and(|workspace| workspace.is_main)
            }
            UiCommand::RefreshWorkspaces => !self.dialogs.refresh_in_flight,
            UiCommand::FocusPreview | UiCommand::OpenCommandPalette => false,
        }
    }
    pub(super) fn execute_command_palette_action(&mut self, id: &str) -> bool {
        let Some(command) = UiCommand::from_palette_id(id) else {
            return false;
        };
        self.execute_ui_command(command)
    }

    pub(super) fn modal_open(&self) -> bool {
        self.has_non_palette_modal_open() || self.dialogs.command_palette.is_visible()
    }
}
