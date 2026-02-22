use super::*;

impl GroveApp {
    pub(super) fn stop_workspace_agent(&mut self, workspace: Workspace) {
        if self.stop_in_flight {
            return;
        }

        if !workspace_can_stop_agent(Some(&workspace)) {
            self.show_info_toast("no agent running");
            return;
        }
        let Some(repo_root) = workspace.project_path.clone() else {
            self.show_error_toast("agent stop failed");
            return;
        };
        let workspace_name = workspace.name.clone();
        let workspace_path = workspace.path.clone();
        let session_name = session_name_for_workspace_ref(&workspace);
        let daemon_socket_path = self.daemon_socket_path_for_workspace(&workspace);
        let request = AgentStopRequest {
            context: RepoContext { repo_root },
            selector: WorkspaceSelector::NameAndPath {
                name: workspace_name.clone(),
                path: workspace_path.clone(),
            },
            workspace_hint: Some(workspace.clone()),
            dry_run: false,
        };

        if !self.tmux_input.supports_background_launch() {
            if let Some(daemon_socket_path) = daemon_socket_path.clone() {
                let completion = execute_stop_agent(
                    request,
                    workspace_name,
                    workspace_path,
                    session_name,
                    Some(daemon_socket_path),
                );
                if let Some(error) = completion.result.as_ref().err() {
                    self.last_tmux_error = Some(error.clone());
                    self.show_error_toast("agent stop failed");
                    return;
                }

                self.apply_stop_agent_completion(completion);
                return;
            }

            let service = InProcessLifecycleCommandService::new();
            let completion = stop_agent_completion_from_service_result(
                service.agent_stop_for_mode(
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
                self.show_error_toast("agent stop failed");
                return;
            }

            self.apply_stop_agent_completion(completion);
            return;
        }

        self.stop_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            Msg::StopAgentCompleted(execute_stop_agent(
                request,
                workspace_name,
                workspace_path,
                session_name,
                daemon_socket_path,
            ))
        }));
    }

    pub(super) fn apply_stop_agent_completion(&mut self, completion: StopAgentCompletion) {
        self.stop_in_flight = false;
        match completion.result {
            Ok(()) => {
                if self
                    .interactive
                    .as_ref()
                    .is_some_and(|state| state.target_session == completion.session_name)
                {
                    self.interactive = None;
                }

                if let Some(workspace_index) = self
                    .state
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.path == completion.workspace_path)
                {
                    let previous_status = self.state.workspaces[workspace_index].status;
                    let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
                    let next_status = if self.state.workspaces[workspace_index].is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    let workspace = &mut self.state.workspaces[workspace_index];
                    workspace.status = next_status;
                    workspace.is_orphaned = false;
                    self.track_workspace_status_transition(
                        &completion.workspace_path,
                        previous_status,
                        next_status,
                        previous_orphaned,
                        false,
                    );
                }
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                self.clear_agent_activity_tracking();
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                self.refresh_preview_summary();
                self.emit_event(
                    LogEvent::new("agent_lifecycle", "agent_stopped")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_success_toast("agent stopped");
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_error_toast("agent stop failed");
            }
        }
    }
}

fn execute_stop_agent(
    request: AgentStopRequest,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
    daemon_socket_path: Option<PathBuf>,
) -> StopAgentCompletion {
    if let Some(socket_path) = daemon_socket_path.as_deref() {
        let (workspace, workspace_path) = daemon_selector_parts(&request.selector);
        let payload = DaemonAgentStopPayload {
            repo_root: request.context.repo_root.display().to_string(),
            workspace,
            workspace_path,
            dry_run: request.dry_run,
        };
        return stop_agent_completion_from_daemon_result(
            agent_stop_via_socket(socket_path, payload),
            fallback_workspace_name,
            fallback_workspace_path,
            fallback_session_name,
        );
    }

    let service = InProcessLifecycleCommandService::new();
    stop_agent_completion_from_service_result(
        service.agent_stop(request),
        fallback_workspace_name,
        fallback_workspace_path,
        fallback_session_name,
    )
}

fn stop_agent_completion_from_service_result(
    result: CommandResult<AgentMutationResponse>,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
) -> StopAgentCompletion {
    match result {
        Ok(response) => StopAgentCompletion {
            workspace_name: response.workspace.name.clone(),
            workspace_path: response.workspace.path.clone(),
            session_name: session_name_for_workspace_ref(&response.workspace),
            result: Ok(()),
        },
        Err(error) => StopAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(error.message),
        },
    }
}

fn stop_agent_completion_from_daemon_result(
    result: std::io::Result<
        Result<
            crate::interface::daemon::DaemonWorkspaceMutationResult,
            crate::interface::daemon::DaemonCommandError,
        >,
    >,
    fallback_workspace_name: String,
    fallback_workspace_path: PathBuf,
    fallback_session_name: String,
) -> StopAgentCompletion {
    match result {
        Ok(Ok(response)) => StopAgentCompletion {
            workspace_name: response.workspace.name,
            workspace_path: PathBuf::from(response.workspace.path),
            session_name: fallback_session_name,
            result: Ok(()),
        },
        Ok(Err(error)) => StopAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(error.message),
        },
        Err(error) => StopAgentCompletion {
            workspace_name: fallback_workspace_name,
            workspace_path: fallback_workspace_path,
            session_name: fallback_session_name,
            result: Err(format!("daemon request failed: {error}")),
        },
    }
}
