use super::update_prelude::*;

impl GroveApp {
    const COMMAND_PALETTE_MAX_VISIBLE_ROWS: usize = 30;

    fn command_palette_style(&self) -> PaletteStyle {
        let theme = self.active_ui_theme();
        PaletteStyle {
            border: Style::new()
                .fg(packed(theme.primary))
                .bg(packed(theme.background))
                .bold(),
            input: Style::new()
                .fg(packed(theme.text))
                .bg(packed(theme.background)),
            item: Style::new()
                .fg(packed(theme.text_subtle))
                .bg(packed(theme.background)),
            item_selected: Style::new()
                .fg(packed(theme.text))
                .bg(packed(theme.surface))
                .bold(),
            match_highlight: Style::new()
                .fg(packed(theme.warning))
                .bg(packed(theme.background))
                .bold(),
            description: Style::new()
                .fg(packed(theme.border))
                .bg(packed(theme.background)),
            category: Style::new()
                .fg(packed(theme.primary))
                .bg(packed(theme.background))
                .bold(),
            hint: Style::new()
                .fg(packed(theme.border))
                .bg(packed(theme.background)),
        }
    }

    fn has_non_palette_modal_open(&self) -> bool {
        self.dialogs.active_dialog.is_some() || self.dialogs.keybind_help_open
    }

    pub(super) fn can_open_palette(&self) -> bool {
        !self.has_non_palette_modal_open()
            && !self.dialogs.command_palette.is_visible()
            && self.session.interactive.is_none()
    }

    pub(super) fn active_palette_label(&self) -> &'static str {
        match self.dialogs.palette_mode {
            Some(PaletteMode::WorkspaceJump) => "Jump",
            _ => "Palette",
        }
    }

    pub(super) fn active_palette_badge_label(&self) -> &'static str {
        match self.dialogs.palette_mode {
            Some(PaletteMode::WorkspaceJump) => "[Jump]",
            _ => "[Palette]",
        }
    }

    fn palette_action(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        tags: &[&str],
        category: impl Into<String>,
    ) -> PaletteActionItem {
        PaletteActionItem::new(id, title)
            .with_description(description)
            .with_tags(tags)
            .with_category(category)
    }

    fn workspace_jump_action_title(&self, workspace: &Workspace) -> String {
        let task = workspace
            .task_slug
            .as_deref()
            .and_then(|task_slug| self.state.tasks.iter().find(|task| task.slug == task_slug));
        let mut terms = Vec::new();

        // ftui's command palette scorer indexes `title`, so keep every
        // searchable workspace term in the title, not just the description.
        for term in [
            workspace.task_slug.as_deref(),
            task.map(|task| task.name.as_str()),
            Some(workspace.name.as_str()),
            workspace.project_name.as_deref(),
            Some(workspace.branch.as_str()),
            workspace
                .path
                .file_name()
                .and_then(|file_name| file_name.to_str()),
        ]
        .into_iter()
        .flatten()
        {
            let term = term.to_string();
            if !terms.iter().any(|existing| existing == &term) {
                terms.push(term);
            }
        }

        terms.join(" · ")
    }

    fn build_workspace_jump_actions(&mut self) -> Vec<PaletteActionItem> {
        let mut actions = Vec::with_capacity(self.state.workspaces.len());
        let mut action_targets = HashMap::new();

        let mut next_id = 0usize;
        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.clone());

        if let Some(selected_workspace_path) = selected_workspace_path.as_ref()
            && let Some(workspace) = self.state.workspaces.iter().find(|workspace| {
                refer_to_same_location(workspace.path.as_path(), selected_workspace_path.as_path())
            })
        {
            let id = format!("workspace-jump-{next_id}");
            next_id = next_id.saturating_add(1);
            action_targets.insert(id.clone(), workspace.path.clone());
            let title = self.workspace_jump_action_title(workspace);
            actions.push(Self::palette_action(
                id,
                title,
                workspace.path.display().to_string(),
                &["jump", "workspace", "switch", "path", "branch"],
                "Workspace",
            ));
        }

        let current_title_len = actions
            .first()
            .map(|action| action.title.len())
            .unwrap_or(0);

        for workspace in &self.state.workspaces {
            if selected_workspace_path
                .as_ref()
                .is_some_and(|selected_workspace_path| {
                    refer_to_same_location(
                        workspace.path.as_path(),
                        selected_workspace_path.as_path(),
                    )
                })
            {
                continue;
            }
            let id = format!("workspace-jump-{next_id}");
            next_id = next_id.saturating_add(1);
            action_targets.insert(id.clone(), workspace.path.clone());
            let title = self.workspace_jump_action_title(workspace);
            let title = if title.len() <= current_title_len {
                let mut padded_title = title;
                padded_title.push_str(
                    " ".repeat(current_title_len + 1 - padded_title.len())
                        .as_str(),
                );
                padded_title
            } else {
                title
            };
            actions.push(Self::palette_action(
                id,
                title,
                workspace.path.display().to_string(),
                &["jump", "workspace", "switch", "path", "branch"],
                "Workspace",
            ));
        }

        self.dialogs.workspace_jump_action_targets = action_targets;
        actions
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

    fn open_shared_palette(&mut self, palette_mode: PaletteMode, actions: Vec<PaletteActionItem>) {
        if !self.can_open_palette() {
            return;
        }

        self.dialogs.command_palette = CommandPalette::new()
            .with_max_visible(self.command_palette_max_visible())
            .with_style(self.command_palette_style());
        self.dialogs.command_palette.replace_actions(actions);
        self.dialogs.palette_mode = Some(palette_mode);
        self.dialogs.command_palette.open();
    }

    pub(super) fn open_command_palette(&mut self) {
        self.open_shared_palette(PaletteMode::Command, self.build_command_palette_actions());
    }

    pub(super) fn open_workspace_jump_palette(&mut self) {
        let actions = self.build_workspace_jump_actions();
        self.open_shared_palette(PaletteMode::WorkspaceJump, actions);
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
            | UiCommand::OpenPerformance
            | UiCommand::ToggleMouseCapture
            | UiCommand::ToggleUnsafe
            | UiCommand::CleanupSessions
            | UiCommand::OpenHelp
            | UiCommand::Quit => true,
            UiCommand::OpenPreview => self.workspace_list_focused(),
            UiCommand::EnterInteractive => {
                self.preview_focused() && self.can_enter_interactive_session()
            }
            UiCommand::FocusList => self.preview_focused(),
            UiCommand::MoveSelectionUp | UiCommand::MoveSelectionDown => true,
            UiCommand::ScrollUp
            | UiCommand::ScrollDown
            | UiCommand::PageUp
            | UiCommand::PageDown
            | UiCommand::ScrollBottom => self.preview_scroll_tab_is_focused(),
            UiCommand::PreviousTab | UiCommand::NextTab => {
                self.state.selected_workspace().is_some()
            }
            UiCommand::MoveTabLeft | UiCommand::MoveTabRight => {
                self.preview_focused()
                    && self
                        .selected_active_tab()
                        .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home)
            }
            UiCommand::AddWorktree => {
                self.workspace_list_focused()
                    && self
                        .state
                        .selected_task()
                        .is_some_and(|task| !task.has_base_worktree())
            }
            UiCommand::OpenRepository => {
                self.workspace_list_focused() && self.state.selected_workspace().is_some()
            }
            UiCommand::StartAgent => {
                !self.dialogs.start_in_flight
                    && !self.dialogs.restart_in_flight
                    && self.state.selected_workspace().is_some()
                    && self.preview_focused()
            }
            UiCommand::StartParentAgent => {
                !self.dialogs.start_in_flight
                    && !self.dialogs.restart_in_flight
                    && self.selected_home_tab_targets_task_root()
            }
            UiCommand::OpenShellTab => self.state.selected_workspace().is_some(),
            UiCommand::OpenGitTab => self.state.selected_workspace().is_some(),
            UiCommand::OpenDiffTab => self.state.selected_workspace().is_some(),
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
                self.workspace_list_focused() && !self.task_delete_requested(task)
            }),
            UiCommand::DeleteWorktree => self.state.selected_worktree().is_some_and(|worktree| {
                self.workspace_list_focused()
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
            UiCommand::FocusAttentionInbox => !self.attention_items.is_empty(),
            UiCommand::AcknowledgeAttention => self.selected_attention_item().is_some(),
            UiCommand::FocusPreview
            | UiCommand::OpenCommandPalette
            | UiCommand::OpenWorkspaceJump => false,
        }
    }
    pub(super) fn execute_command_palette_action(&mut self, id: &str) -> bool {
        let Some(command) = UiCommand::from_palette_id(id) else {
            return false;
        };
        self.execute_ui_command(command)
    }

    pub(super) fn execute_workspace_jump_action(&mut self, id: &str) -> bool {
        let Some(workspace_path) = self.dialogs.workspace_jump_action_targets.get(id) else {
            return false;
        };
        let already_selected = self.state.selected_workspace().is_some_and(|workspace| {
            refer_to_same_location(workspace.path.as_path(), workspace_path.as_path())
        });
        self.selected_attention_item = None;
        if already_selected {
            let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
            self.poll_preview();
            return true;
        }

        if !self.state.select_workspace_path(workspace_path.as_path()) {
            return false;
        }

        self.handle_workspace_selection_changed();
        let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
        true
    }

    pub(super) fn execute_visible_palette_action(&mut self, id: &str) -> bool {
        match self.dialogs.palette_mode {
            Some(PaletteMode::WorkspaceJump) => self.execute_workspace_jump_action(id),
            Some(PaletteMode::Command) => self.execute_command_palette_action(id),
            None => false,
        }
    }

    pub(super) fn modal_open(&self) -> bool {
        self.has_non_palette_modal_open() || self.dialogs.command_palette.is_visible()
    }
}
