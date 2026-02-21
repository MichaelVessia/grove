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
        self.pending_create_start_config = Some(dialog.start_config.clone());
        let request = WorkspaceCreateRequest {
            context: RepoContext {
                repo_root: project.path,
            },
            name: workspace_name.clone(),
            base_branch: Some(dialog.base_branch.trim().to_string()),
            existing_branch: None,
            agent: Some(dialog.agent),
            start: false,
            dry_run: false,
            setup_template: Some(WorkspaceCreateSetupTemplate {
                auto_run_setup_commands: dialog.auto_run_setup_commands,
                commands: parse_setup_commands(&dialog.setup_commands),
            }),
        };
        if !self.tmux_input.supports_background_launch() {
            self.apply_create_workspace_completion(execute_workspace_create(
                request,
                self.daemon_socket_path.clone(),
            ));
            return;
        }

        let daemon_socket_path = self.daemon_socket_path.clone();
        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            Msg::CreateWorkspaceCompleted(execute_workspace_create(request, daemon_socket_path))
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
        let workspace_name = completion.workspace_name;
        match completion.result {
            Ok(()) => {
                self.close_active_dialog();
                self.clear_create_branch_picker();
                let workspace_path = completion.workspace_path;
                self.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
                    workspace_path: workspace_path.clone(),
                    start_config: start_config.clone(),
                });
                self.launch_skip_permissions = start_config.skip_permissions;
                self.pending_auto_launch_shell_workspace_path = Some(workspace_path.clone());
                self.refresh_workspaces(Some(workspace_path));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if completion.warnings.is_empty() {
                    self.show_success_toast(format!("workspace '{}' created", workspace_name));
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' created, warning: {}",
                        workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.show_error_toast(format!("workspace create failed: {error}"));
            }
        }
    }
}

fn execute_workspace_create(
    request: WorkspaceCreateRequest,
    daemon_socket_path: Option<PathBuf>,
) -> CreateWorkspaceCompletion {
    let workspace_name = request.name.clone();
    let workspace_path = request.context.repo_root.join(workspace_name.as_str());
    if let Some(socket_path) = daemon_socket_path.as_deref() {
        let payload = DaemonWorkspaceCreatePayload {
            repo_root: request.context.repo_root.display().to_string(),
            name: request.name,
            base_branch: request.base_branch,
            existing_branch: request.existing_branch,
            agent: request
                .agent
                .map(|agent| agent.label().to_ascii_lowercase()),
            start: request.start,
            dry_run: request.dry_run,
        };
        return match workspace_create_via_socket(socket_path, payload) {
            Ok(Ok(response)) => CreateWorkspaceCompletion {
                workspace_name,
                workspace_path: PathBuf::from(response.workspace.path),
                result: Ok(()),
                warnings: response.warnings,
            },
            Ok(Err(error)) => CreateWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result: Err(error.message),
                warnings: Vec::new(),
            },
            Err(error) => CreateWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result: Err(format!("daemon request failed: {error}")),
                warnings: Vec::new(),
            },
        };
    }

    let service = InProcessLifecycleCommandService::new();
    match service.workspace_create(request) {
        Ok(response) => CreateWorkspaceCompletion {
            workspace_name,
            workspace_path: response.workspace.path,
            result: Ok(()),
            warnings: response.warnings,
        },
        Err(error) => CreateWorkspaceCompletion {
            workspace_name,
            workspace_path,
            result: Err(error.message),
            warnings: Vec::new(),
        },
    }
}
