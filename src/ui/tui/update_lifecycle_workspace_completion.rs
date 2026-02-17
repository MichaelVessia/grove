use super::*;

impl GroveApp {
    pub(super) fn apply_delete_project_completion(&mut self, completion: DeleteProjectCompletion) {
        self.project_delete_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.projects = completion.projects;
                self.refresh_project_dialog_filtered();
                self.event_log.log(
                    LogEvent::new("project_lifecycle", "project_deleted")
                        .with_data("project", Value::from(completion.project_name.clone()))
                        .with_data(
                            "path",
                            Value::from(completion.project_path.display().to_string()),
                        ),
                );
                self.refresh_workspaces(None);
                self.show_toast(
                    format!("project '{}' removed from workspace list", completion.project_name),
                    false,
                );
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("project_lifecycle", "project_delete_failed")
                        .with_data("project", Value::from(completion.project_name))
                        .with_data(
                            "path",
                            Value::from(completion.project_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.show_toast(format!("project delete failed: {error}"), true);
            }
        }
    }

    pub(super) fn apply_delete_workspace_completion(
        &mut self,
        completion: DeleteWorkspaceCompletion,
    ) {
        self.delete_in_flight = false;
        self.delete_in_flight_workspace = None;
        match completion.result {
            Ok(()) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_deleted")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data(
                            "warning_count",
                            Value::from(
                                u64::try_from(completion.warnings.len()).unwrap_or(u64::MAX),
                            ),
                        ),
                );
                self.last_tmux_error = None;
                self.refresh_workspaces(None);
                if completion.warnings.is_empty() {
                    self.show_toast(
                        format!("workspace '{}' deleted", completion.workspace_name),
                        false,
                    );
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' deleted, warning: {}",
                            completion.workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_delete_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("error", Value::from(error.clone())),
                );
                self.last_tmux_error = Some(error.clone());
                self.show_toast(format!("workspace delete failed: {error}"), true);
            }
        }
    }

    pub(super) fn apply_merge_workspace_completion(
        &mut self,
        completion: MergeWorkspaceCompletion,
    ) {
        self.merge_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_merged")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data(
                            "workspace_branch",
                            Value::from(completion.workspace_branch.clone()),
                        )
                        .with_data("base_branch", Value::from(completion.base_branch.clone()))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data(
                            "warning_count",
                            Value::from(
                                u64::try_from(completion.warnings.len()).unwrap_or(u64::MAX),
                            ),
                        ),
                );
                self.last_tmux_error = None;
                self.refresh_workspaces(None);
                if completion.warnings.is_empty() {
                    self.show_toast(
                        format!(
                            "workspace '{}' merged into '{}'",
                            completion.workspace_name, completion.base_branch
                        ),
                        false,
                    );
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' merged, warning: {}",
                            completion.workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_merge_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.last_tmux_error = Some(error.clone());
                self.show_toast(format!("workspace merge failed: {error}"), true);
            }
        }
    }

    pub(super) fn apply_update_from_base_completion(
        &mut self,
        completion: UpdateWorkspaceFromBaseCompletion,
    ) {
        self.update_from_base_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_updated_from_base")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data(
                            "workspace_branch",
                            Value::from(completion.workspace_branch.clone()),
                        )
                        .with_data("base_branch", Value::from(completion.base_branch.clone()))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data(
                            "warning_count",
                            Value::from(
                                u64::try_from(completion.warnings.len()).unwrap_or(u64::MAX),
                            ),
                        ),
                );
                self.last_tmux_error = None;
                self.refresh_workspaces(Some(completion.workspace_path));
                if completion.warnings.is_empty() {
                    self.show_toast(
                        format!(
                            "workspace '{}' updated from '{}'",
                            completion.workspace_name, completion.base_branch
                        ),
                        false,
                    );
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' updated, warning: {}",
                            completion.workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_update_from_base_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.last_tmux_error = Some(error.clone());
                self.show_toast(format!("workspace update failed: {error}"), true);
            }
        }
    }
}
