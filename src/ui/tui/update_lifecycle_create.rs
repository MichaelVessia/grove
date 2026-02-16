use super::*;

impl GroveApp {
    pub(super) fn confirm_create_dialog(&mut self) {
        if self.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog.as_ref().cloned() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "create",
            "dialog_confirmed",
            [
                (
                    "workspace_name".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("branch_mode".to_string(), Value::from("new")),
                (
                    "branch_value".to_string(),
                    Value::from(dialog.base_branch.clone()),
                ),
                (
                    "project_index".to_string(),
                    Value::from(u64::try_from(dialog.project_index).unwrap_or(u64::MAX)),
                ),
            ],
        );
        let Some(project) = self.projects.get(dialog.project_index).cloned() else {
            self.show_toast("project is required", true);
            return;
        };

        let workspace_name = dialog.workspace_name.trim().to_string();
        let branch_mode = BranchMode::NewBranch {
            base_branch: dialog.base_branch.trim().to_string(),
        };
        let request = CreateWorkspaceRequest {
            workspace_name: workspace_name.clone(),
            branch_mode,
            agent: dialog.agent,
        };

        if let Err(error) = request.validate() {
            self.show_toast(workspace_lifecycle_error_message(&error), true);
            return;
        }

        let repo_root = project.path;
        if !self.tmux_input.supports_background_launch() {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let result = create_workspace(&repo_root, &request, &git, &setup);
            self.apply_create_workspace_completion(CreateWorkspaceCompletion { request, result });
            return;
        }

        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let result = create_workspace(&repo_root, &request, &git, &setup);
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion { request, result })
        }));
    }

    pub(super) fn apply_create_workspace_completion(
        &mut self,
        completion: CreateWorkspaceCompletion,
    ) {
        self.create_in_flight = false;
        let workspace_name = completion.request.workspace_name;
        match completion.result {
            Ok(result) => {
                self.create_dialog = None;
                self.clear_create_branch_picker();
                self.refresh_workspaces(Some(result.workspace_path));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if result.warnings.is_empty() {
                    self.show_toast(format!("workspace '{}' created", workspace_name), false);
                } else if let Some(first_warning) = result.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' created, warning: {}",
                            workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.show_toast(
                    format!(
                        "workspace create failed: {}",
                        workspace_lifecycle_error_message(&error)
                    ),
                    true,
                );
            }
        }
    }
}
