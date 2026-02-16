use super::*;

impl GroveApp {
    pub(super) fn apply_delete_workspace_completion(
        &mut self,
        completion: DeleteWorkspaceCompletion,
    ) {
        self.delete_in_flight = false;
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

    pub(super) fn refresh_workspaces(&mut self, preferred_workspace_path: Option<PathBuf>) {
        if !self.tmux_input.supports_background_launch() {
            self.refresh_workspaces_sync(preferred_workspace_path);
            return;
        }

        if self.refresh_in_flight {
            return;
        }

        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let multiplexer = self.multiplexer;
        let projects = self.projects.clone();
        self.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let bootstrap = bootstrap_data_for_projects(&projects, multiplexer);
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_path: target_path,
                bootstrap,
            })
        }));
    }

    fn refresh_workspaces_sync(&mut self, preferred_workspace_path: Option<PathBuf>) {
        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let bootstrap = bootstrap_data_for_projects(&self.projects, self.multiplexer);

        self.repo_name = bootstrap.repo_name;
        self.discovery_state = bootstrap.discovery_state;
        self.state = AppState::new(bootstrap.workspaces);
        if let Some(path) = target_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
    }

    pub(super) fn apply_refresh_workspaces_completion(
        &mut self,
        completion: RefreshWorkspacesCompletion,
    ) {
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;

        self.repo_name = completion.bootstrap.repo_name;
        self.discovery_state = completion.bootstrap.discovery_state;
        self.state = AppState::new(completion.bootstrap.workspaces);
        if let Some(path) = completion.preferred_workspace_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.refresh_in_flight = false;
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
    }
}
