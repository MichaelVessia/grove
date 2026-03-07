use super::*;

impl GroveApp {
    fn filtered_create_dialog_project_indices(&self, query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..self.projects.len()).collect();
        }

        let query_lower = query.to_ascii_lowercase();
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| {
                project.name.to_ascii_lowercase().contains(&query_lower)
                    || project
                        .path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query_lower)
            })
            .map(|(index, _)| index)
            .collect()
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

    pub(super) fn selected_create_dialog_projects(&self) -> Vec<ProjectConfig> {
        let Some(dialog) = self.create_dialog() else {
            return Vec::new();
        };

        match dialog.tab {
            CreateDialogTab::Manual => dialog
                .selected_repository_indices
                .iter()
                .filter_map(|index| self.projects.get(*index).cloned())
                .collect(),
            CreateDialogTab::PullRequest => self
                .projects
                .get(dialog.project_index)
                .cloned()
                .into_iter()
                .collect(),
        }
    }

    pub(super) fn toggle_create_dialog_project_selection(&mut self) {
        if self
            .create_dialog()
            .is_some_and(|dialog| dialog.tab == CreateDialogTab::PullRequest)
        {
            return;
        }

        let Some(dialog) = self.create_dialog_mut() else {
            return;
        };
        let project_index = dialog.project_index;
        if let Some(selected_index) = dialog
            .selected_repository_indices
            .iter()
            .position(|index| *index == project_index)
        {
            dialog.selected_repository_indices.remove(selected_index);
            return;
        }

        dialog.selected_repository_indices.push(project_index);
        dialog.selected_repository_indices.sort_unstable();
    }

    pub(super) fn apply_create_dialog_project_defaults(&mut self, project_index: usize) {
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.project_index = project_index;
        }
    }

    fn build_create_project_picker_state(
        &self,
        selected_project_index: usize,
    ) -> CreateProjectPickerState {
        let filtered_project_indices = self.filtered_create_dialog_project_indices("");
        let selected_filtered_index = filtered_project_indices
            .iter()
            .position(|index| *index == selected_project_index)
            .unwrap_or(0);
        CreateProjectPickerState {
            filter: String::new(),
            filtered_project_indices,
            selected_filtered_index,
        }
    }

    pub(super) fn create_project_picker_open(&self) -> bool {
        self.create_dialog()
            .and_then(|dialog| dialog.project_picker.as_ref())
            .is_some()
    }

    pub(super) fn open_create_project_picker(&mut self) {
        if self.projects.is_empty() {
            self.show_info_toast("no projects configured, press p, then Ctrl+A to add one");
            return;
        }

        let selected_project_index = self
            .create_dialog()
            .map(|dialog| dialog.project_index)
            .unwrap_or(0);
        let picker = self.build_create_project_picker_state(selected_project_index);
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.project_picker = Some(picker);
        }
    }

    pub(super) fn close_create_project_picker(&mut self) {
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.project_picker = None;
        }
    }

    pub(super) fn refresh_create_project_picker_filtered(&mut self) {
        let query = self
            .create_dialog()
            .and_then(|dialog| dialog.project_picker.as_ref())
            .map(|picker| picker.filter.clone())
            .unwrap_or_default();
        let filtered_project_indices = self.filtered_create_dialog_project_indices(&query);

        let Some(dialog) = self.create_dialog_mut() else {
            return;
        };
        let Some(picker) = dialog.project_picker.as_mut() else {
            return;
        };

        picker.filtered_project_indices = filtered_project_indices;
        if picker.filtered_project_indices.is_empty() {
            picker.selected_filtered_index = 0;
            return;
        }
        if picker.selected_filtered_index >= picker.filtered_project_indices.len() {
            picker.selected_filtered_index =
                picker.filtered_project_indices.len().saturating_sub(1);
        }
    }

    pub(super) fn selected_create_project_picker_project_index(&self) -> Option<usize> {
        let dialog = self.create_dialog()?;
        let picker = dialog.project_picker.as_ref()?;
        if picker.filtered_project_indices.is_empty() {
            return None;
        }
        picker
            .filtered_project_indices
            .get(picker.selected_filtered_index)
            .copied()
    }

    pub(super) fn open_create_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.projects.is_empty() {
            self.show_info_toast("no projects configured, press p, then Ctrl+A to add one");
            return;
        }

        let project_index = self.selected_project_index();
        self.set_create_dialog(CreateDialogState {
            tab: CreateDialogTab::Manual,
            task_name: String::new(),
            pr_url: String::new(),
            project_index,
            selected_repository_indices: vec![project_index],
            project_picker: None,
            focused_field: CreateDialogField::first_for_tab(CreateDialogTab::Manual),
        });
        self.log_dialog_event("create", "dialog_opened");
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.session.last_tmux_error = None;
    }
}
