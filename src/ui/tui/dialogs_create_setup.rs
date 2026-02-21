use super::*;

impl GroveApp {
    fn selected_base_branch(&self) -> String {
        let selected = self.state.selected_workspace();
        if let Some(workspace) = selected
            && let Some(base_branch) = workspace.base_branch.as_ref()
            && !base_branch.trim().is_empty()
        {
            return base_branch.clone();
        }

        if let Some(workspace) = selected
            && !workspace.branch.trim().is_empty()
            && workspace.branch != "(detached)"
        {
            return workspace.branch.clone();
        }

        "main".to_string()
    }

    pub(super) fn selected_project_index(&self) -> usize {
        let Some(workspace) = self.state.selected_workspace() else {
            return 0;
        };
        self.project_index_for_workspace(workspace).unwrap_or(0)
    }

    fn create_dialog_selected_project(&self) -> Option<&ProjectConfig> {
        let dialog = self.create_dialog()?;
        self.projects.get(dialog.project_index)
    }

    fn project_default_base_branch(&self, project_index: usize) -> Option<String> {
        let project = self.projects.get(project_index)?;
        let base_branch = project.defaults.base_branch.trim();
        if base_branch.is_empty() {
            return None;
        }
        Some(base_branch.to_string())
    }

    fn project_default_setup_commands(&self, project_index: usize) -> String {
        let Some(project) = self.projects.get(project_index) else {
            return String::new();
        };
        format_setup_commands(&project.defaults.setup_commands)
    }

    fn project_default_auto_run_setup_commands(&self, project_index: usize) -> bool {
        self.projects
            .get(project_index)
            .is_none_or(|project| project.defaults.auto_run_setup_commands)
    }

    pub(super) fn apply_create_dialog_project_defaults(&mut self, project_index: usize) {
        let base_branch = self
            .project_default_base_branch(project_index)
            .or_else(|| {
                self.create_dialog()
                    .map(|dialog| dialog.base_branch.clone())
            })
            .unwrap_or_else(|| "main".to_string());
        let setup_commands = self.project_default_setup_commands(project_index);
        let auto_run_setup_commands = self.project_default_auto_run_setup_commands(project_index);

        if let Some(dialog) = self.create_dialog_mut() {
            dialog.project_index = project_index;
            dialog.base_branch = base_branch.clone();
            dialog.setup_commands = setup_commands;
            dialog.auto_run_setup_commands = auto_run_setup_commands;
        }

        self.refresh_create_dialog_branch_candidates(base_branch);
    }

    pub(super) fn refresh_create_dialog_branch_candidates(&mut self, selected_base_branch: String) {
        let branches = self
            .create_dialog_selected_project()
            .map(|project| load_local_branches(&project.path).unwrap_or_default())
            .unwrap_or_default();
        self.create_branch_all = branches;
        if !self
            .create_branch_all
            .iter()
            .any(|branch| branch == &selected_base_branch)
        {
            self.create_branch_all.insert(0, selected_base_branch);
        }
        self.refresh_create_branch_filtered();
    }

    pub(super) fn open_create_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.projects.is_empty() {
            self.show_info_toast("no projects configured, press p to add one");
            return;
        }

        let project_index = self.selected_project_index();
        let selected_base_branch = self
            .project_default_base_branch(project_index)
            .unwrap_or_else(|| self.selected_base_branch());
        let default_agent = self
            .state
            .selected_workspace()
            .map_or(AgentType::Claude, |workspace| workspace.agent);
        let setup_commands = self.project_default_setup_commands(project_index);
        let auto_run_setup_commands = self.project_default_auto_run_setup_commands(project_index);
        self.set_create_dialog(CreateDialogState {
            workspace_name: String::new(),
            project_index,
            agent: default_agent,
            base_branch: selected_base_branch.clone(),
            setup_commands,
            auto_run_setup_commands,
            start_config: StartAgentConfigState::new(
                String::new(),
                String::new(),
                self.launch_skip_permissions,
            ),
            focused_field: CreateDialogField::WorkspaceName,
        });
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
        self.log_dialog_event_with_fields(
            "create",
            "dialog_opened",
            [("agent".to_string(), Value::from(default_agent.label()))],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    pub(super) fn toggle_agent(agent: AgentType) -> AgentType {
        match agent {
            AgentType::Claude => AgentType::Codex,
            AgentType::Codex => AgentType::Claude,
        }
    }

    pub(super) fn clear_create_branch_picker(&mut self) {
        self.create_branch_all.clear();
        self.create_branch_filtered.clear();
        self.create_branch_index = 0;
    }

    pub(super) fn refresh_create_branch_filtered(&mut self) {
        let query = self
            .create_dialog()
            .map(|dialog| dialog.base_branch.clone())
            .unwrap_or_default();
        self.create_branch_filtered = filter_branches(&query, &self.create_branch_all);
        if self.create_branch_filtered.is_empty() {
            self.create_branch_index = 0;
            return;
        }
        if self.create_branch_index >= self.create_branch_filtered.len() {
            self.create_branch_index = self.create_branch_filtered.len().saturating_sub(1);
        }
    }
}
