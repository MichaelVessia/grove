use super::*;

impl GroveApp {
    pub(super) fn confirm_create_dialog(&mut self) {
        if self.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog().cloned() else {
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
                    Value::from(usize_to_u64(dialog.project_index)),
                ),
                (
                    "setup_auto_run".to_string(),
                    Value::from(dialog.auto_run_setup_commands),
                ),
                (
                    "setup_commands".to_string(),
                    Value::from(dialog.setup_commands.clone()),
                ),
                (
                    "prompt_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.prompt.len())),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(dialog.start_config.skip_permissions),
                ),
                (
                    "pre_launch_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.pre_launch_command.len())),
                ),
            ],
        );
        let Some(project) = self.projects.get(dialog.project_index).cloned() else {
            self.show_info_toast("project is required");
            return;
        };

        let workspace_name = dialog.workspace_name.trim().to_string();
        let branch_mode = BranchMode::NewBranch {
            base_branch: dialog.base_branch.trim().to_string(),
        };
        let setup_template = WorkspaceSetupTemplate {
            auto_run_setup_commands: dialog.auto_run_setup_commands,
            commands: parse_setup_commands(&dialog.setup_commands),
        };
        let request = CreateWorkspaceRequest {
            workspace_name: workspace_name.clone(),
            branch_mode,
            agent: dialog.agent,
        };

        if let Err(error) = request.validate() {
            self.show_info_toast(workspace_lifecycle_error_message(&error));
            return;
        }

        self.pending_create_start_config = Some(dialog.start_config.clone());
        let repo_root = project.path;
        if !self.tmux_input.supports_background_launch() {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let setup_command = CommandSetupCommandRunner;
            let result = create_workspace_with_template(
                &repo_root,
                &request,
                Some(&setup_template),
                &git,
                &setup,
                &setup_command,
            );
            self.apply_create_workspace_completion(CreateWorkspaceCompletion { request, result });
            return;
        }

        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let setup_command = CommandSetupCommandRunner;
            let result = create_workspace_with_template(
                &repo_root,
                &request,
                Some(&setup_template),
                &git,
                &setup,
                &setup_command,
            );
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion { request, result })
        }));
    }

    pub(super) fn apply_create_workspace_completion(
        &mut self,
        completion: CreateWorkspaceCompletion,
    ) {
        self.create_in_flight = false;
        let fallback_skip_permissions = self.launch_skip_permissions;
        let start_config = self.pending_create_start_config.take().unwrap_or_else(|| {
            StartAgentConfigState::new(String::new(), String::new(), fallback_skip_permissions)
        });
        let workspace_name = completion.request.workspace_name;
        match completion.result {
            Ok(result) => {
                self.close_active_dialog();
                self.clear_create_branch_picker();
                self.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
                    workspace_path: result.workspace_path.clone(),
                    start_config: start_config.clone(),
                });
                self.launch_skip_permissions = start_config.skip_permissions;
                self.pending_auto_launch_shell_workspace_path = Some(result.workspace_path.clone());
                self.refresh_workspaces(Some(result.workspace_path));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if result.warnings.is_empty() {
                    self.show_success_toast(format!("workspace '{}' created", workspace_name));
                } else if let Some(first_warning) = result.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' created, warning: {}",
                        workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.show_error_toast(format!(
                    "workspace create failed: {}",
                    workspace_lifecycle_error_message(&error)
                ));
            }
        }
    }
}
