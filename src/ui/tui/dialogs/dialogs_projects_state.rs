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
            Some(dialog) => dialog.filter().to_string(),
            None => return,
        };
        let filtered = self.filtered_project_indices(&query);
        let Some(dialog) = self.project_dialog_mut() else {
            return;
        };

        dialog.filtered_project_indices = filtered;
        if dialog.filtered_project_indices.is_empty() {
            dialog.project_list.select(None);
            return;
        }
        let selected = dialog.selected_filtered_index();
        if selected >= dialog.filtered_project_indices.len() {
            dialog.set_selected_filtered_index(
                dialog.filtered_project_indices.len().saturating_sub(1),
            );
        } else if dialog.project_list.selected().is_none() {
            dialog.set_selected_filtered_index(0);
        }
    }

    pub(super) fn selected_project_dialog_project_index(&self) -> Option<usize> {
        let dialog = self.project_dialog()?;
        if dialog.filtered_project_indices.is_empty() {
            return None;
        }
        dialog
            .filtered_project_indices
            .get(dialog.selected_filtered_index())
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
        let mut project_list = ListState::default();
        if !filtered_project_indices.is_empty() {
            project_list.select(Some(selected_filtered_index));
        }
        self.set_project_dialog(ProjectDialogState {
            filter_input: TextInput::new().with_focused(true),
            filtered_project_indices,
            project_list,
            add_dialog: None,
            defaults_dialog: None,
        });
    }

    pub(super) fn open_project_add_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        let mut add_dialog = ProjectAddDialogState {
            path_input: TextInput::new().with_focused(true),
            name_input: TextInput::new(),
            path_matches: Vec::new(),
            path_match_list: ListState::default(),
            cached_search_root: None,
            cached_repo_roots: Vec::new(),
        };
        add_dialog.sync_focus(Some(ProjectAddDialogField::Path));
        project_dialog.add_dialog = Some(add_dialog);
        self.open_project_add_dialog_focus_trap(ProjectAddDialogField::Path);
        self.sync_active_dialog_focus_field();
    }

    pub(super) fn close_project_add_dialog(&mut self) {
        if let Some(project_dialog) = self.project_dialog_mut() {
            project_dialog.add_dialog = None;
        }
        self.close_project_add_dialog_focus_trap();
        self.sync_active_dialog_focus_field();
    }
}
