use super::*;

impl GroveApp {
    pub(super) fn stop_selected_workspace_agent(&mut self) {
        if self.stop_in_flight {
            return;
        }

        if !workspace_can_stop_agent(self.state.selected_workspace()) {
            self.show_toast("no agent running", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        let workspace_for_task = workspace.clone();

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_stop_workspace_with_result_for_mode(
                &workspace,
                self.multiplexer,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.last_tmux_error = Some(error.clone());
                self.show_toast("agent stop failed", true);
                return;
            }

            self.apply_stop_agent_completion(completion.into());
            return;
        }

        self.stop_in_flight = true;
        let multiplexer = self.multiplexer;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_stop_workspace_with_result_for_mode(
                &workspace_for_task,
                multiplexer,
                CommandExecutionMode::Process,
            );
            Msg::StopAgentCompleted(completion.into())
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

                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.path == completion.workspace_path)
                {
                    workspace.status = if workspace.is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    workspace.is_orphaned = false;
                }
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                self.clear_agent_activity_tracking();
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_stopped")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_toast("agent stopped", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_toast("agent stop failed", true);
            }
        }
    }
}
