use super::update_prelude::*;

impl GroveApp {
    fn summarize_merge_failure(error: &str) -> String {
        let conflict_prefix = "CONFLICT (content): Merge conflict in ";
        let conflict_files = error
            .lines()
            .filter_map(|line| line.trim().strip_prefix(conflict_prefix))
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>();
        if !conflict_files.is_empty() {
            let files = conflict_files.join(", ");
            return format!("merge conflict, resolve in base worktree then retry (files: {files})");
        }

        if error.contains("Automatic merge failed; fix conflicts and then commit the result.") {
            return "merge conflict, resolve in base worktree then retry".to_string();
        }

        if error.contains("base worktree has uncommitted changes") {
            return "merge blocked, base worktree has uncommitted or untracked files".to_string();
        }
        if error.contains("workspace worktree has uncommitted changes") {
            return "merge blocked, workspace has uncommitted or untracked files".to_string();
        }

        format!("workspace merge failed: {error}")
    }

    pub(super) fn apply_delete_project_completion(&mut self, completion: DeleteProjectCompletion) {
        self.dialogs.project_delete_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.projects = completion.projects;
                self.refresh_project_dialog_filtered();
                self.telemetry.event_log.log(
                    LogEvent::new("project_lifecycle", "project_deleted")
                        .with_data("project", Value::from(completion.project_name.clone()))
                        .with_data(
                            "path",
                            Value::from(completion.project_path.display().to_string()),
                        ),
                );
                self.refresh_workspaces(None);
                self.show_success_toast(format!(
                    "project '{}' removed from workspace list",
                    completion.project_name
                ));
            }
            Err(error) => {
                self.telemetry.event_log.log(
                    LogEvent::new("project_lifecycle", "project_delete_failed")
                        .with_data("project", Value::from(completion.project_name))
                        .with_data(
                            "path",
                            Value::from(completion.project_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.show_error_toast(format!("project delete failed: {error}"));
            }
        }
    }

    pub(super) fn apply_delete_workspace_completion(
        &mut self,
        completion: DeleteWorkspaceCompletion,
    ) {
        self.dialogs
            .delete_requested_workspaces
            .remove(&completion.workspace_path);
        if self
            .dialogs
            .delete_in_flight_workspace
            .as_ref()
            .is_some_and(|workspace_path| workspace_path == &completion.workspace_path)
        {
            self.dialogs.delete_in_flight_workspace = None;
        }
        match completion.result {
            Ok(()) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_deleted")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data(
                            "warning_count",
                            Value::from(usize_to_u64(completion.warnings.len())),
                        ),
                );
                self.session.last_tmux_error = None;
                self.refresh_workspaces(None);
                if completion.warnings.is_empty() {
                    self.show_success_toast(format!(
                        "workspace '{}' deleted",
                        completion.workspace_name
                    ));
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' deleted, warning: {}",
                        completion.workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_delete_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("error", Value::from(error.clone())),
                );
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast(format!("workspace delete failed: {error}"));
            }
        }
        self.start_next_queued_delete_workspace();
    }

    pub(super) fn apply_merge_workspace_completion(
        &mut self,
        completion: MergeWorkspaceCompletion,
    ) {
        self.dialogs.merge_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.telemetry.event_log.log(
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
                            Value::from(usize_to_u64(completion.warnings.len())),
                        ),
                );
                self.session.last_tmux_error = None;
                self.refresh_workspaces(None);
                if completion.warnings.is_empty() {
                    self.show_success_toast(format!(
                        "workspace '{}' merged into '{}'",
                        completion.workspace_name, completion.base_branch
                    ));
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' merged, warning: {}",
                        completion.workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_merge_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast(Self::summarize_merge_failure(&error));
            }
        }
    }

    pub(super) fn apply_update_from_base_completion(
        &mut self,
        completion: UpdateWorkspaceFromBaseCompletion,
    ) {
        self.dialogs.update_from_base_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.telemetry.event_log.log(
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
                            Value::from(usize_to_u64(completion.warnings.len())),
                        ),
                );
                self.session.last_tmux_error = None;
                self.refresh_workspaces(Some(completion.workspace_path));
                if completion.warnings.is_empty() {
                    self.show_success_toast(format!(
                        "workspace '{}' updated from '{}'",
                        completion.workspace_name, completion.base_branch
                    ));
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' updated, warning: {}",
                        completion.workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_update_from_base_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast(format!("workspace update failed: {error}"));
            }
        }
    }
}
