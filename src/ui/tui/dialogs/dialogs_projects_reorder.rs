use super::*;

impl GroveApp {
    pub(super) fn project_reorder_active(&self) -> bool {
        self.project_dialog()
            .and_then(|dialog| dialog.reorder.as_ref())
            .is_some()
    }

    pub(super) fn open_project_reorder_mode(&mut self) {
        if self.project_reorder_active() {
            return;
        }
        let Some(dialog) = self.project_dialog() else {
            return;
        };
        if !dialog.filter.trim().is_empty() {
            self.show_info_toast("clear filter before reordering projects");
            return;
        }
        let Some(selected_project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        let Some(selected_project) = self.projects.get(selected_project_index) else {
            self.show_info_toast("project not found");
            return;
        };

        let original_projects = self.projects.clone();
        let moving_project_path = selected_project.path.clone();
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = Some(ProjectReorderState {
                original_projects,
                moving_project_path,
            });
        }
        self.show_info_toast("reorder mode, j/k or Up/Down move, Enter save, Esc cancel");
    }

    pub(super) fn move_selected_project_in_dialog(&mut self, direction: i8) {
        if !self.project_reorder_active() {
            return;
        }

        let Some(selected_project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        if self.projects.is_empty() {
            return;
        }

        let next_project_index = if direction.is_negative() {
            selected_project_index.saturating_sub(1)
        } else {
            selected_project_index
                .saturating_add(1)
                .min(self.projects.len().saturating_sub(1))
        };
        if next_project_index == selected_project_index {
            return;
        }

        self.projects
            .swap(selected_project_index, next_project_index);
        let moving_project_path = self
            .projects
            .get(next_project_index)
            .map(|project| project.path.clone());
        self.refresh_project_dialog_filtered();

        if let Some(path) = moving_project_path.as_ref() {
            self.select_project_dialog_project_by_path(path.as_path());
            if let Some(dialog) = self.project_dialog_mut()
                && let Some(reorder) = dialog.reorder.as_mut()
            {
                reorder.moving_project_path = path.clone();
            }
        }
    }

    fn reorder_workspaces_for_project_order(&mut self) {
        if self.state.workspaces.is_empty() {
            return;
        }

        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.clone());
        let projects = &self.projects;
        self.state.workspaces.sort_by_key(|workspace| {
            workspace
                .project_path
                .as_ref()
                .and_then(|workspace_project_path| {
                    projects.iter().position(|project| {
                        refer_to_same_location(
                            project.path.as_path(),
                            workspace_project_path.as_path(),
                        )
                    })
                })
                .unwrap_or(usize::MAX)
        });

        if let Some(selected_workspace_path) = selected_workspace_path
            && let Some(index) = self.state.workspaces.iter().position(|workspace| {
                refer_to_same_location(workspace.path.as_path(), selected_workspace_path.as_path())
            })
        {
            self.state.selected_index = index;
        }
    }

    pub(super) fn save_project_reorder_from_dialog(&mut self) {
        if !self.project_reorder_active() {
            return;
        }
        if let Err(error) = self.save_projects_config() {
            self.show_error_toast(format!("project order save failed: {error}"));
            return;
        }

        self.reorder_workspaces_for_project_order();
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = None;
        }
        self.show_success_toast("project order saved");
    }

    pub(super) fn cancel_project_reorder_from_dialog(&mut self) {
        let Some(reorder) = self
            .project_dialog()
            .and_then(|dialog| dialog.reorder.as_ref().cloned())
        else {
            return;
        };

        self.projects = reorder.original_projects;
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = None;
        }
        self.refresh_project_dialog_filtered();
        self.select_project_dialog_project_by_path(reorder.moving_project_path.as_path());
        self.show_info_toast("project reorder cancelled");
    }
}
