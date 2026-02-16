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
        let Some(workspace_project_path) = workspace.project_path.as_ref() else {
            return 0;
        };
        self.projects
            .iter()
            .position(|project| project_paths_equal(&project.path, workspace_project_path))
            .unwrap_or(0)
    }

    fn create_dialog_selected_project(&self) -> Option<&ProjectConfig> {
        let dialog = self.create_dialog.as_ref()?;
        self.projects.get(dialog.project_index)
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
            self.show_toast("no projects configured, press p to add one", true);
            return;
        }

        let selected_base_branch = self.selected_base_branch();
        let default_agent = self
            .state
            .selected_workspace()
            .map_or(AgentType::Claude, |workspace| workspace.agent);
        let project_index = self.selected_project_index();
        self.create_dialog = Some(CreateDialogState {
            workspace_name: String::new(),
            project_index,
            agent: default_agent,
            base_branch: selected_base_branch.clone(),
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
            .create_dialog
            .as_ref()
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
