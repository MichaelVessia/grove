use super::*;

impl GroveApp {
    pub(super) fn start_selected_workspace_agent_with_options(
        &mut self,
        prompt: Option<String>,
        pre_launch_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.start_in_flight {
            return;
        }

        if !workspace_can_start_agent(self.state.selected_workspace()) {
            self.show_info_toast("workspace cannot be started");
            return;
        }
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if !self.ensure_workspace_backend_available(&workspace, "agent start") {
            return;
        }
        let Some(repo_root) = workspace.project_path.clone() else {
            self.show_error_toast("agent start failed");
            return;
        };
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let workspace_name = workspace.name.clone();
        let workspace_path = workspace.path.clone();
        let session_name = session_name_for_workspace_ref(&workspace);
        let daemon_socket_path = self.daemon_socket_path_for_workspace(&workspace);

        let request = AgentStartRequest {
            context: RepoContext { repo_root },
            selector: WorkspaceSelector::NameAndPath {
                name: workspace_name.clone(),
                path: workspace_path.clone(),
            },
            workspace_hint: Some(workspace.clone()),
            prompt,
            pre_launch_command,
            skip_permissions,
            capture_cols: Some(capture_cols),
            capture_rows: Some(capture_rows),
            dry_run: false,
        };

        if !self.tmux_input.supports_background_launch() {
            if let Some(daemon_socket_path) = daemon_socket_path.clone() {
                let completion = execute_start_agent(
                    request,
                    workspace_name,
                    workspace_path,
                    session_name,
                    Some(daemon_socket_path),
                );
                if let Some(error) = completion.result.as_ref().err() {
                    self.last_tmux_error = Some(error.clone());
                    self.show_error_toast("agent start failed");
                    return;
                }

                self.apply_start_agent_completion(completion);
                return;
            }

            let service = InProcessLifecycleCommandService::new();
            let completion = start_agent_completion_from_service_result(
                service.agent_start_for_mode(
                    request,
                    CommandExecutionMode::Delegating(&mut |command| {
                        self.execute_tmux_command(command)
                    }),
                ),
                workspace_name,
                workspace_path,
                session_name,
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.last_tmux_error = Some(error.clone());
                self.show_error_toast("agent start failed");
                return;
            }

            self.apply_start_agent_completion(completion);
            return;
        }

        self.start_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            Msg::StartAgentCompleted(execute_start_agent(
                request,
                workspace_name,
                workspace_path,
                session_name,
                daemon_socket_path,
            ))
        }));
    }

    pub(super) fn apply_start_agent_completion(&mut self, completion: StartAgentCompletion) {
        self.start_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                if let Some(workspace_index) = self
                    .state
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.path == completion.workspace_path)
                {
                    let previous_status = self.state.workspaces[workspace_index].status;
                    let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
                    let workspace = &mut self.state.workspaces[workspace_index];
                    workspace.status = WorkspaceStatus::Active;
                    workspace.is_orphaned = false;
                    self.track_workspace_status_transition(
                        &completion.workspace_path,
                        previous_status,
                        WorkspaceStatus::Active,
                        previous_orphaned,
                        false,
                    );
                }
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_started")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_success_toast("agent started");
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_error_toast("agent start failed");
            }
        }
    }

    pub(super) fn confirm_start_dialog(&mut self) {
        let Some(dialog) = self.take_launch_dialog() else {
            return;
        };
        let workspace_name = self.selected_workspace_name().unwrap_or_default();
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_confirmed",
            [
                ("workspace".to_string(), Value::from(workspace_name)),
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

        let StartOptions {
            prompt,
            pre_launch_command,
            skip_permissions,
        } = dialog.start_config.parse_start_options();
        self.launch_skip_permissions = skip_permissions;
        self.start_selected_workspace_agent_with_options(
            prompt,
            pre_launch_command,
            skip_permissions,
        );
    }
}

fn execute_start_agent(
    request: AgentStartRequest,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
    daemon_socket_path: Option<PathBuf>,
) -> StartAgentCompletion {
    if let Some(socket_path) = daemon_socket_path.as_deref() {
        let (workspace, workspace_path) = daemon_selector_parts(&request.selector);
        let payload = DaemonAgentStartPayload {
            repo_root: request.context.repo_root.display().to_string(),
            workspace,
            workspace_path,
            prompt: request.prompt,
            pre_launch_command: request.pre_launch_command,
            skip_permissions: request.skip_permissions,
            dry_run: request.dry_run,
            capture_cols: request.capture_cols,
            capture_rows: request.capture_rows,
        };
        return start_agent_completion_from_daemon_result(
            agent_start_via_socket(socket_path, payload),
            fallback_workspace_name,
            fallback_workspace_path,
            fallback_session_name,
        );
    }

    let service = InProcessLifecycleCommandService::new();
    start_agent_completion_from_service_result(
        service.agent_start(request),
        fallback_workspace_name,
        fallback_workspace_path,
        fallback_session_name,
    )
}

fn start_agent_completion_from_service_result(
    result: CommandResult<AgentMutationResponse>,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
) -> StartAgentCompletion {
    match result {
        Ok(response) => StartAgentCompletion {
            workspace_name: response.workspace.name.clone(),
            workspace_path: response.workspace.path.clone(),
            session_name: session_name_for_workspace_ref(&response.workspace),
            result: Ok(()),
        },
        Err(error) => StartAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(error.message),
        },
    }
}

fn start_agent_completion_from_daemon_result(
    result: std::io::Result<
        Result<
            crate::interface::daemon::DaemonWorkspaceMutationResult,
            crate::interface::daemon::DaemonCommandError,
        >,
    >,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
) -> StartAgentCompletion {
    match result {
        Ok(Ok(response)) => StartAgentCompletion {
            workspace_name: response.workspace.name,
            workspace_path: PathBuf::from(response.workspace.path),
            session_name: fallback_session_name,
            result: Ok(()),
        },
        Ok(Err(error)) => StartAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(error.message),
        },
        Err(error) => StartAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(format!("daemon request failed: {error}")),
        },
    }
}
