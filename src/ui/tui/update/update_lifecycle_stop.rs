use super::*;

impl GroveApp {
    fn take_pending_restart_for_workspace(&mut self, workspace_path: &Path) -> bool {
        if self
            .pending_restart_workspace_path
            .as_ref()
            .is_some_and(|pending_path| pending_path == workspace_path)
        {
            self.pending_restart_workspace_path = None;
            return true;
        }
        false
    }

    pub(super) fn restart_workspace_agent_for_path(&mut self, workspace_path: &Path) {
        if self.start_in_flight || self.stop_in_flight || self.restart_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        let Some(workspace) = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)
            .cloned()
        else {
            self.show_info_toast("no agent running");
            return;
        };
        if !workspace_can_stop_agent(Some(&workspace)) {
            self.show_info_toast("no agent running");
            return;
        }

        if agent_supports_in_pane_restart(workspace.agent) {
            self.restart_workspace_agent_in_pane(workspace);
            return;
        }

        self.pending_restart_workspace_path = Some(workspace.path.clone());
        self.stop_workspace_agent(workspace);
    }

    fn restart_workspace_agent_in_pane(&mut self, workspace: Workspace) {
        if self.restart_in_flight {
            return;
        }

        let skip_permissions = self.workspace_skip_permissions_for_workspace(&workspace);
        let agent_env = match self.project_agent_env_for_workspace(&workspace) {
            Ok(agent_env) => agent_env,
            Err(error) => {
                self.show_info_toast(format!("invalid project agent env: {error}"));
                return;
            }
        };
        if !self.tmux_input.supports_background_launch() {
            let session_name = session_name_for_workspace_ref(&workspace);
            let completion = RestartAgentCompletion {
                workspace_name: workspace.name.clone(),
                workspace_path: workspace.path.clone(),
                session_name,
                result: restart_workspace_in_pane_with_io(
                    &workspace,
                    skip_permissions,
                    &agent_env,
                    |command| self.tmux_input.execute(command),
                    |target_session, scrollback_lines, include_escape_sequences| {
                        self.tmux_input.capture_output(
                            target_session,
                            scrollback_lines,
                            include_escape_sequences,
                        )
                    },
                ),
            };
            self.apply_restart_agent_completion(completion);
            return;
        }

        self.restart_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_restart_workspace_in_pane_with_result(
                &workspace,
                skip_permissions,
                agent_env,
            );
            Msg::RestartAgentCompleted(completion.into())
        }));
    }

    pub(super) fn stop_workspace_agent(&mut self, workspace: Workspace) {
        if self.stop_in_flight || self.restart_in_flight {
            return;
        }

        if !workspace_can_stop_agent(Some(&workspace)) {
            self.show_info_toast("no agent running");
            return;
        }

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_stop_workspace_with_result_for_mode(
                &workspace,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.take_pending_restart_for_workspace(&workspace.path);
                self.last_tmux_error = Some(error.clone());
                self.show_error_toast("agent stop failed");
                return;
            }

            self.apply_stop_agent_completion(completion.into());
            return;
        }

        self.stop_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_stop_workspace_with_result_for_mode(
                &workspace,
                CommandExecutionMode::Process,
            );
            Msg::StopAgentCompleted(completion.into())
        }));
    }

    pub(super) fn apply_stop_agent_completion(&mut self, completion: StopAgentCompletion) {
        self.stop_in_flight = false;
        let should_restart = self.take_pending_restart_for_workspace(&completion.workspace_path);
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
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_stopped")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                if should_restart {
                    self.restart_workspace_agent_by_path(&completion.workspace_path);
                } else {
                    self.show_success_toast("agent stopped");
                }
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_error_toast("agent stop failed");
            }
        }
    }

    pub(super) fn apply_restart_agent_completion(&mut self, completion: RestartAgentCompletion) {
        self.restart_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                self.clear_agent_activity_tracking();
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
                    LogEvent::new("agent_lifecycle", "agent_restarted")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_success_toast("agent restarted");
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_error_toast("agent restart failed");
            }
        }
    }
}
