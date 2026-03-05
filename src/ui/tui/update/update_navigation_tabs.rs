use super::update_prelude::*;

impl GroveApp {
    pub(super) fn sync_workspace_tab_maps(&mut self) {
        let workspace_paths = self
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.path.clone())
            .collect::<std::collections::HashSet<PathBuf>>();

        self.workspace_tabs
            .retain(|path, _| workspace_paths.contains(path));
        self.last_agent_selection
            .retain(|path, _| workspace_paths.contains(path));

        for workspace in &self.state.workspaces {
            self.workspace_tabs
                .entry(workspace.path.clone())
                .or_default()
                .ensure_home_tab();
            self.last_agent_selection
                .entry(workspace.path.clone())
                .or_insert(workspace.agent);
        }

        self.sync_preview_tab_from_active_workspace_tab();
    }

    pub(super) fn selected_workspace_tabs_state(&self) -> Option<&WorkspaceTabsState> {
        let workspace = self.state.selected_workspace()?;
        self.workspace_tabs.get(workspace.path.as_path())
    }

    pub(super) fn selected_workspace_tabs_state_mut(&mut self) -> Option<&mut WorkspaceTabsState> {
        let workspace_path = self.state.selected_workspace()?.path.clone();
        self.workspace_tabs.get_mut(workspace_path.as_path())
    }

    pub(super) fn selected_active_tab(&self) -> Option<&WorkspaceTab> {
        self.selected_workspace_tabs_state()?.active_tab()
    }

    pub(super) fn selected_active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.selected_workspace_tabs_state_mut()?.active_tab_mut()
    }

    pub(super) fn selected_active_tab_kind(&self) -> PreviewTab {
        self.selected_active_tab()
            .map(|tab| PreviewTab::from(tab.kind))
            .unwrap_or(PreviewTab::Home)
    }

    pub(super) fn sync_preview_tab_from_active_workspace_tab(&mut self) {
        self.preview_tab = self.selected_active_tab_kind();
    }

    pub(super) fn cycle_selected_workspace_tabs(&mut self, direction: i8) {
        let workspace_path = match self.state.selected_workspace() {
            Some(workspace) => workspace.path.clone(),
            None => return,
        };
        let Some(tabs) = self.workspace_tabs.get_mut(workspace_path.as_path()) else {
            return;
        };
        let Some(active_index) = tabs.active_index() else {
            return;
        };
        if tabs.tabs.is_empty() {
            return;
        }
        let next_index = if direction.is_negative() {
            if active_index == 0 {
                tabs.tabs.len().saturating_sub(1)
            } else {
                active_index.saturating_sub(1)
            }
        } else {
            (active_index + 1) % tabs.tabs.len()
        };
        if let Some(next_tab) = tabs.tabs.get(next_index) {
            tabs.active_tab_id = next_tab.id;
        }
        self.sync_preview_tab_from_active_workspace_tab();
    }

    pub(super) fn select_tab_by_id_for_selected_workspace(&mut self, tab_id: u64) -> bool {
        let Some(tabs) = self.selected_workspace_tabs_state_mut() else {
            return false;
        };
        if !tabs.set_active(tab_id) {
            return false;
        }
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
        true
    }

    fn next_tab_ordinal(tabs: &WorkspaceTabsState, kind: WorkspaceTabKind) -> u64 {
        let count = tabs.tabs.iter().filter(|tab| tab.kind == kind).count();
        let count_u64 = u64::try_from(count).unwrap_or(0);
        count_u64.saturating_add(1)
    }

    fn new_session_name_for_tab(
        workspace: &Workspace,
        kind: WorkspaceTabKind,
        ordinal: u64,
    ) -> Option<String> {
        match kind {
            WorkspaceTabKind::Home => None,
            WorkspaceTabKind::Git => Some(git_session_name_for_workspace(workspace)),
            WorkspaceTabKind::Agent => {
                let workspace_name = format!("{}-agent-{ordinal}", workspace.name);
                Some(session_name_for_workspace_in_project(
                    workspace.project_name.as_deref(),
                    workspace_name.as_str(),
                ))
            }
            WorkspaceTabKind::Shell => {
                let workspace_name = format!("{}-shell-{ordinal}", workspace.name);
                Some(session_name_for_workspace_in_project(
                    workspace.project_name.as_deref(),
                    workspace_name.as_str(),
                ))
            }
        }
    }

    fn ensure_selected_workspace_tab_kind(
        &mut self,
        kind: WorkspaceTabKind,
    ) -> Option<(PathBuf, u64)> {
        self.sync_workspace_tab_maps();
        let workspace = self.state.selected_workspace()?.clone();
        let workspace_path = workspace.path.clone();
        let selected_tab_id = {
            let tabs = self.workspace_tabs.get_mut(workspace_path.as_path())?;
            if let Some(existing_id) = tabs.find_kind(kind).map(|tab| tab.id) {
                tabs.active_tab_id = existing_id;
                existing_id
            } else {
                let ordinal = Self::next_tab_ordinal(tabs, kind);
                let session_name = Self::new_session_name_for_tab(&workspace, kind, ordinal);
                let title = match kind {
                    WorkspaceTabKind::Agent => {
                        let agent = self
                            .last_agent_selection
                            .get(workspace.path.as_path())
                            .copied()
                            .unwrap_or(workspace.agent);
                        format!("{} {ordinal}", agent.label())
                    }
                    WorkspaceTabKind::Shell => format!("Shell {ordinal}"),
                    WorkspaceTabKind::Git => "Git".to_string(),
                    WorkspaceTabKind::Home => "Home".to_string(),
                };
                tabs.insert_tab_adjacent(WorkspaceTab {
                    id: 0,
                    kind,
                    title,
                    session_name,
                    agent_type: None,
                    state: WorkspaceTabRuntimeState::Stopped,
                })
            }
        };
        self.sync_preview_tab_from_active_workspace_tab();
        Some((workspace_path, selected_tab_id))
    }

    pub(super) fn open_or_focus_git_tab(&mut self) {
        let Some((_, tab_id)) = self.ensure_selected_workspace_tab_kind(WorkspaceTabKind::Git)
        else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let _ = self.select_tab_by_id_for_selected_workspace(tab_id);
        let _ = self.ensure_lazygit_session_for_selected_workspace();
        if let Some(tab) = self.selected_active_tab_mut() {
            tab.state = WorkspaceTabRuntimeState::Running;
        }
        self.poll_preview();
    }

    fn set_tab_state_by_id(
        &mut self,
        workspace_path: &Path,
        tab_id: u64,
        state: WorkspaceTabRuntimeState,
    ) {
        if let Some(tabs) = self.workspace_tabs.get_mut(workspace_path)
            && let Some(tab) = tabs.tab_by_id_mut(tab_id)
        {
            tab.state = state;
        }
    }

    pub(super) fn open_new_shell_tab(&mut self) {
        self.sync_workspace_tab_maps();
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let Some(tabs) = self.workspace_tabs.get_mut(workspace.path.as_path()) else {
            return;
        };
        let ordinal = Self::next_tab_ordinal(tabs, WorkspaceTabKind::Shell);
        let Some(session_name) =
            Self::new_session_name_for_tab(&workspace, WorkspaceTabKind::Shell, ordinal)
        else {
            return;
        };
        let tab_id = tabs.insert_tab_adjacent(WorkspaceTab {
            id: 0,
            kind: WorkspaceTabKind::Shell,
            title: format!("Shell {ordinal}"),
            session_name: Some(session_name.clone()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Starting,
        });
        self.sync_preview_tab_from_active_workspace_tab();
        self.session
            .shell_sessions
            .mark_in_flight(session_name.clone());
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let workspace_init_command = self.workspace_init_command_for_workspace(&workspace);
        let request = shell_launch_request_for_workspace(
            &workspace,
            session_name.clone(),
            String::new(),
            workspace_init_command,
            Some(capture_cols),
            Some(capture_rows),
        );
        let (_, result) = execute_shell_launch_request_for_mode(
            &request,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        if let Err(error) = &result
            && !tmux_launch_error_indicates_duplicate_session(error)
        {
            self.session
                .shell_sessions
                .mark_failed(session_name.clone());
            self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Failed);
            self.session.last_tmux_error = Some(error.clone());
            self.show_error_toast("shell tab launch failed");
            return;
        }
        self.session.shell_sessions.mark_ready(session_name);
        self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Running);
        self.session.last_tmux_error = None;
        self.poll_preview();
    }

    fn agent_env_for_workspace_agent(
        &self,
        workspace: &Workspace,
        agent: AgentType,
    ) -> Result<Vec<(String, String)>, String> {
        let Some(workspace_project_path) = workspace.project_path.as_ref() else {
            return Ok(Vec::new());
        };
        let Some(project) = self
            .projects
            .iter()
            .find(|project| refer_to_same_location(&project.path, workspace_project_path))
        else {
            return Ok(Vec::new());
        };
        let entries = match agent {
            AgentType::Claude => &project.defaults.agent_env.claude,
            AgentType::Codex => &project.defaults.agent_env.codex,
            AgentType::OpenCode => &project.defaults.agent_env.opencode,
        };
        parse_agent_env_vars_from_entries(entries).map(|vars| {
            vars.into_iter()
                .map(|entry| (entry.key, entry.value))
                .collect()
        })
    }

    pub(super) fn launch_new_agent_tab(
        &mut self,
        agent: AgentType,
        options: StartOptions,
    ) -> Result<(), String> {
        self.sync_workspace_tab_maps();
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            return Err("no workspace selected".to_string());
        };
        self.last_agent_selection
            .insert(workspace.path.clone(), agent);
        self.launch_skip_permissions = options.skip_permissions;
        let _ = write_workspace_skip_permissions(&workspace.path, options.skip_permissions);
        let _ = write_workspace_init_command(&workspace.path, options.init_command.as_deref());

        let Some(tabs) = self.workspace_tabs.get_mut(workspace.path.as_path()) else {
            return Err("workspace tabs unavailable".to_string());
        };
        let ordinal = Self::next_tab_ordinal(tabs, WorkspaceTabKind::Agent);
        let Some(session_name) =
            Self::new_session_name_for_tab(&workspace, WorkspaceTabKind::Agent, ordinal)
        else {
            return Err("failed to build agent session name".to_string());
        };
        let tab_id = tabs.insert_tab_adjacent(WorkspaceTab {
            id: 0,
            kind: WorkspaceTabKind::Agent,
            title: format!("{} {ordinal}", agent.label()),
            session_name: Some(session_name.clone()),
            agent_type: Some(agent),
            state: WorkspaceTabRuntimeState::Starting,
        });
        self.sync_preview_tab_from_active_workspace_tab();

        let agent_env = self.agent_env_for_workspace_agent(&workspace, agent)?;
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let mut launch_workspace = workspace.clone();
        launch_workspace.name = format!("{}-agent-{ordinal}", workspace.name);
        launch_workspace.agent = agent;
        let request = launch_request_for_workspace(
            &launch_workspace,
            options.prompt,
            options
                .init_command
                .or_else(|| self.workspace_init_command_for_workspace(&workspace)),
            options.skip_permissions,
            agent_env,
            Some(capture_cols),
            Some(capture_rows),
        );
        self.session
            .agent_sessions
            .mark_in_flight(session_name.clone());
        let completion = execute_launch_request_with_result_for_mode(
            &request,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        if let Err(error) = completion.result
            && !tmux_launch_error_indicates_duplicate_session(&error)
        {
            self.session.agent_sessions.mark_failed(session_name);
            self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Failed);
            self.session.last_tmux_error = Some(error.clone());
            return Err(error);
        }
        self.session.agent_sessions.mark_ready(session_name);
        self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Running);
        self.session.last_tmux_error = None;
        self.poll_preview();
        Ok(())
    }

    fn session_exists(&self, session_name: &str) -> bool {
        let command = vec![
            "tmux".to_string(),
            "has-session".to_string(),
            "-t".to_string(),
            session_name.to_string(),
        ];
        self.tmux_input.execute(&command).is_ok()
    }

    pub(super) fn active_tab_session_name(&self) -> Option<String> {
        self.selected_active_tab()?.session_name.clone()
    }

    pub(super) fn kill_active_tab_session(&mut self) {
        let Some(session_name) = self.active_tab_session_name() else {
            self.show_info_toast("home tab has no live session");
            return;
        };
        let command = vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            session_name.clone(),
        ];
        if let Err(error) = self.execute_tmux_command(&command) {
            let message = error.to_string();
            self.session.last_tmux_error = Some(message.clone());
            self.show_error_toast(format!("kill failed: {message}"));
            return;
        }
        self.session.agent_sessions.remove_ready(&session_name);
        self.session.shell_sessions.remove_ready(&session_name);
        self.session.lazygit_sessions.remove_ready(&session_name);
        if let Some(tab) = self.selected_active_tab_mut() {
            tab.state = WorkspaceTabRuntimeState::Stopped;
        }
        self.session.last_tmux_error = None;
        self.poll_preview();
    }

    pub(super) fn close_active_tab_or_confirm(&mut self) {
        let Some(tab) = self.selected_active_tab().cloned() else {
            return;
        };
        if tab.kind == WorkspaceTabKind::Home {
            self.show_info_toast("home tab cannot be closed");
            return;
        }
        if let Some(session_name) = tab.session_name.as_deref()
            && self.session_exists(session_name)
        {
            let Some(workspace) = self.state.selected_workspace() else {
                return;
            };
            self.set_confirm_dialog(ConfirmDialogState {
                action: ConfirmDialogAction::CloseActiveTab {
                    workspace_path: workspace.path.clone(),
                    tab_id: tab.id,
                    session_name: session_name.to_string(),
                },
                focused_field: ConfirmDialogField::CancelButton,
            });
            return;
        }
        self.close_tab_for_selected_workspace(tab.id);
    }

    pub(super) fn close_tab_for_selected_workspace(&mut self, tab_id: u64) {
        let Some(tabs) = self.selected_workspace_tabs_state_mut() else {
            return;
        };
        let _ = tabs.close_tab(tab_id);
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
    }

    pub(super) fn force_close_active_tab_and_session(
        &mut self,
        workspace_path: &Path,
        tab_id: u64,
        session_name: &str,
    ) {
        let command = vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            session_name.to_string(),
        ];
        let _ = self.execute_tmux_command(&command);
        self.session.agent_sessions.remove_ready(session_name);
        self.session.shell_sessions.remove_ready(session_name);
        self.session.lazygit_sessions.remove_ready(session_name);
        if let Some(tabs) = self.workspace_tabs.get_mut(workspace_path) {
            let _ = tabs.close_tab(tab_id);
        }
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
    }

    pub(super) fn active_tab_is_scrollable(&self) -> bool {
        matches!(
            self.selected_active_tab_kind(),
            PreviewTab::Agent | PreviewTab::Shell
        )
    }
}
