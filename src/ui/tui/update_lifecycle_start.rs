use super::*;

impl GroveApp {
    fn start_selected_workspace_agent_with_options(
        &mut self,
        prompt: Option<String>,
        pre_launch_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.start_in_flight {
            return;
        }

        if !workspace_can_start_agent(self.state.selected_workspace()) {
            self.show_toast("workspace cannot be started", true);
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        let capture_cols = self
            .preview_output_dimensions()
            .map_or(self.viewport_width.saturating_sub(4), |(width, _)| width)
            .max(80);
        let capture_rows = self.viewport_height.saturating_sub(4).max(1);

        let request = launch_request_for_workspace(
            workspace,
            prompt,
            pre_launch_command,
            skip_permissions,
            Some(capture_cols),
            Some(capture_rows),
        );

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                self.multiplexer,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.last_tmux_error = Some(error.clone());
                self.show_toast("agent start failed", true);
                return;
            }

            self.apply_start_agent_completion(completion.into());
            return;
        }

        self.start_in_flight = true;
        let multiplexer = self.multiplexer;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                multiplexer,
                CommandExecutionMode::Process,
            );
            Msg::StartAgentCompleted(completion.into())
        }));
    }

    pub(super) fn apply_start_agent_completion(&mut self, completion: StartAgentCompletion) {
        self.start_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.path == completion.workspace_path)
                {
                    workspace.status = WorkspaceStatus::Active;
                    workspace.is_orphaned = false;
                }
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_started")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_toast("agent started", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_toast("agent start failed", true);
            }
        }
    }

    pub(super) fn confirm_start_dialog(&mut self) {
        let Some(dialog) = self.launch_dialog.take() else {
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
                    Value::from(u64::try_from(dialog.prompt.len()).unwrap_or(u64::MAX)),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(dialog.skip_permissions),
                ),
                (
                    "pre_launch_len".to_string(),
                    Value::from(u64::try_from(dialog.pre_launch_command.len()).unwrap_or(u64::MAX)),
                ),
            ],
        );

        self.launch_skip_permissions = dialog.skip_permissions;
        let prompt = if dialog.prompt.trim().is_empty() {
            None
        } else {
            Some(dialog.prompt.trim().to_string())
        };
        let pre_launch_command = if dialog.pre_launch_command.trim().is_empty() {
            None
        } else {
            Some(dialog.pre_launch_command.trim().to_string())
        };
        self.start_selected_workspace_agent_with_options(
            prompt,
            pre_launch_command,
            dialog.skip_permissions,
        );
    }
}
