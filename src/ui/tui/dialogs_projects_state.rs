use super::*;

impl GroveApp {
    fn filtered_project_indices(&self, query: &str) -> Vec<usize> {
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

    pub(super) fn refresh_project_dialog_filtered(&mut self) {
        let query = match self.project_dialog() {
            Some(dialog) => dialog.filter.clone(),
            None => return,
        };
        let filtered = self.filtered_project_indices(&query);
        let Some(dialog) = self.project_dialog_mut() else {
            return;
        };

        dialog.filtered_project_indices = filtered;
        if dialog.filtered_project_indices.is_empty() {
            dialog.selected_filtered_index = 0;
            return;
        }
        if dialog.selected_filtered_index >= dialog.filtered_project_indices.len() {
            dialog.selected_filtered_index =
                dialog.filtered_project_indices.len().saturating_sub(1);
        }
    }

    pub(super) fn selected_project_dialog_project_index(&self) -> Option<usize> {
        let dialog = self.project_dialog()?;
        if dialog.filtered_project_indices.is_empty() {
            return None;
        }
        dialog
            .filtered_project_indices
            .get(dialog.selected_filtered_index)
            .copied()
    }

    pub(super) fn focus_project_by_index(&mut self, project_index: usize) {
        let Some(project) = self.projects.get(project_index) else {
            return;
        };

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace.is_main
                        && workspace
                            .project_name
                            .as_deref()
                            .is_some_and(|name| name == project.name)
                        && workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| refer_to_same_location(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
            return;
        }

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace
                        .project_name
                        .as_deref()
                        .is_some_and(|name| name == project.name)
                        && workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| refer_to_same_location(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
            return;
        }

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| refer_to_same_location(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
        }
    }

    pub(super) fn open_project_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let selected_project_index = self.selected_project_index();
        let filtered_project_indices: Vec<usize> = (0..self.projects.len()).collect();
        let selected_filtered_index = filtered_project_indices
            .iter()
            .position(|index| *index == selected_project_index)
            .unwrap_or(0);
        self.set_project_dialog(ProjectDialogState {
            filter: String::new(),
            filtered_project_indices,
            selected_filtered_index,
            add_dialog: None,
            defaults_dialog: None,
        });
    }

    pub(super) fn open_project_add_dialog(&mut self) {
        let remote_profile = self
            .active_remote_profile
            .clone()
            .or_else(|| {
                self.remote_profiles
                    .first()
                    .map(|profile| profile.name.clone())
            })
            .unwrap_or_default();
        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        project_dialog.add_dialog = Some(ProjectAddDialogState {
            name: String::new(),
            path: String::new(),
            target_is_remote: false,
            remote_profile,
            focused_field: ProjectAddDialogField::Name,
        });
    }
}
