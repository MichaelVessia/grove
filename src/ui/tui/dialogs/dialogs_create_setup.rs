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
            .position(|project| refer_to_same_location(&project.path, workspace_project_path))
            .unwrap_or(0)
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

    pub(super) fn apply_create_dialog_project_defaults(&mut self, project_index: usize) {
        let base_branch = self
            .project_default_base_branch(project_index)
            .or_else(|| {
                self.create_dialog()
                    .map(|dialog| dialog.base_branch.clone())
            })
            .unwrap_or_else(|| "main".to_string());

        if let Some(dialog) = self.create_dialog_mut() {
            dialog.project_index = project_index;
            dialog.base_branch = base_branch.clone();
        }

        self.refresh_create_dialog_branch_candidates(base_branch);
    }

    pub(super) fn refresh_create_dialog_branch_candidates(&mut self, selected_base_branch: String) {
        let branches = self
            .create_dialog_selected_project()
            .map(|project| load_local_branches(&project.path).unwrap_or_default())
            .unwrap_or_default();
        self.dialogs.create_branch_all = branches;
        if !self
            .dialogs
            .create_branch_all
            .iter()
            .any(|branch| branch == &selected_base_branch)
        {
            self.dialogs
                .create_branch_all
                .insert(0, selected_base_branch);
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
        self.set_create_dialog(CreateDialogState {
            tab: CreateDialogTab::Manual,
            workspace_name: String::new(),
            pr_url: String::new(),
            project_index,
            base_branch: selected_base_branch.clone(),
            focused_field: CreateDialogField::first_for_tab(CreateDialogTab::Manual),
        });
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
        self.log_dialog_event("create", "dialog_opened");
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.session.last_tmux_error = None;
    }

    pub(super) fn clear_create_branch_picker(&mut self) {
        self.dialogs.create_branch_all.clear();
        self.dialogs.create_branch_filtered.clear();
        self.dialogs.create_branch_index = 0;
    }

    pub(super) fn refresh_create_branch_filtered(&mut self) {
        let query = self
            .create_dialog()
            .map(|dialog| dialog.base_branch.clone())
            .unwrap_or_default();
        self.dialogs.create_branch_filtered =
            filter_branches(&query, &self.dialogs.create_branch_all);
        if self.dialogs.create_branch_filtered.is_empty() {
            self.dialogs.create_branch_index = 0;
            return;
        }
        if self.dialogs.create_branch_index >= self.dialogs.create_branch_filtered.len() {
            self.dialogs.create_branch_index =
                self.dialogs.create_branch_filtered.len().saturating_sub(1);
        }
    }
}
