use super::*;

impl GroveApp {
    fn start_workspace_agent_with_options(
        &mut self,
        workspace: Workspace,
        prompt: Option<String>,
        pre_launch_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.start_in_flight {
            return;
        }
        if !workspace_can_start_agent(Some(&workspace)) {
            self.show_info_toast("workspace cannot be started");
            return;
        }

        let (capture_cols, capture_rows) = self.capture_dimensions();
        let request = launch_request_for_workspace(
            &workspace,
            prompt,
            pre_launch_command,
            skip_permissions,
            Some(capture_cols),
            Some(capture_rows),
        );

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.last_tmux_error = Some(error.clone());
                self.show_error_toast("agent start failed");
                return;
            }

            self.apply_start_agent_completion(completion.into());
            return;
        }

        self.start_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Process,
            );
            Msg::StartAgentCompleted(completion.into())
        }));
    }

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
        self.start_workspace_agent_with_options(
            workspace,
            prompt,
            pre_launch_command,
            skip_permissions,
        );
    }

    pub(super) fn restart_workspace_agent_by_path(&mut self, workspace_path: &Path) {
        let Some(workspace) = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)
            .cloned()
        else {
            self.show_error_toast("agent restart failed");
            return;
        };

        let prompt = read_workspace_launch_prompt(&workspace.path);
        self.start_workspace_agent_with_options(
            workspace,
            prompt,
            None,
            self.launch_skip_permissions,
        );
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
