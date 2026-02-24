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

        // Try graceful restart for agents that support it
        if workspace.agent.exit_command().is_some()
            && workspace.agent.resume_command_pattern().is_some()
        {
            self.graceful_restart_workspace_agent(workspace);
            return;
        }

        // Fall back to hard restart for unsupported agents
        self.pending_restart_workspace_path = Some(workspace.path.clone());
        self.stop_workspace_agent(workspace);
    }

    pub(super) fn stop_workspace_agent(&mut self, workspace: Workspace) {
        if self.stop_in_flight {
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

    fn graceful_restart_workspace_agent(&mut self, workspace: Workspace) {
        let exit_command = workspace.agent.exit_command().unwrap().to_string();
        let resume_pattern = workspace
            .agent
            .resume_command_pattern()
            .unwrap()
            .to_string();
        let session_name = session_name_for_workspace_ref(&workspace);
        let workspace_name = workspace.name.clone();
        let workspace_path = workspace.path.clone();

        self.restart_in_flight = true;
        self.show_info_toast("restarting agent...");

        self.queue_cmd(Cmd::task(move || {
            let result = execute_graceful_restart(&session_name, &exit_command, &resume_pattern);
            Msg::GracefulRestartCompleted(GracefulRestartCompletion {
                workspace_name,
                workspace_path,
                session_name,
                result,
            })
        }));
    }

    pub(super) fn apply_graceful_restart_completion(
        &mut self,
        completion: GracefulRestartCompletion,
    ) {
        self.restart_in_flight = false;
        match completion.result {
            Ok(()) => {
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
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_restarted_graceful")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_success_toast("agent restarted (resumed)");
                self.poll_preview();
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "graceful_restart_failed")
                        .with_data("workspace", Value::from(completion.workspace_name.as_str()))
                        .with_data("error", Value::from(error.as_str())),
                );
                self.show_info_toast("graceful restart failed, hard restarting...");
                self.pending_restart_workspace_path = Some(completion.workspace_path.clone());

                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter()
                    .find(|w| w.path == completion.workspace_path)
                    .cloned()
                {
                    self.stop_workspace_agent(workspace);
                } else {
                    self.show_error_toast("agent restart failed");
                }
            }
        }
    }
}

fn execute_graceful_restart(
    session_name: &str,
    exit_command: &str,
    resume_pattern: &str,
) -> Result<(), String> {
    use std::thread;
    use std::time::{Duration, Instant};

    let regex =
        regex::Regex::new(resume_pattern).map_err(|e| format!("invalid resume pattern: {e}"))?;

    // Send exit command via tmux send-keys -l (literal mode)
    let send_status = std::process::Command::new("tmux")
        .args(["send-keys", "-l", "-t", session_name, exit_command])
        .output()
        .map_err(|e| format!("failed to send exit command: {e}"))?;

    if !send_status.status.success() {
        return Err(format!(
            "tmux send-keys failed: {}",
            String::from_utf8_lossy(&send_status.stderr).trim()
        ));
    }

    // Poll for resume command in pane output
    let timeout = Duration::from_secs(15);
    let poll_interval = Duration::from_millis(500);
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err("timeout waiting for resume command".to_string());
        }

        thread::sleep(poll_interval);

        let capture = std::process::Command::new("tmux")
            .args(["capture-pane", "-p", "-t", session_name, "-S", "-50"])
            .output()
            .map_err(|e| format!("capture-pane failed: {e}"))?;

        if !capture.status.success() {
            return Err(format!(
                "capture-pane failed: {}",
                String::from_utf8_lossy(&capture.stderr).trim()
            ));
        }

        let output = String::from_utf8_lossy(&capture.stdout);

        if let Some(captures) = regex.captures(&output)
            && let Some(resume_cmd) = captures.get(1)
        {
            let resume_command = format!("{}\n", resume_cmd.as_str());

            let resume_status = std::process::Command::new("tmux")
                .args(["send-keys", "-l", "-t", session_name, &resume_command])
                .output()
                .map_err(|e| format!("failed to send resume command: {e}"))?;

            if !resume_status.status.success() {
                return Err(format!(
                    "tmux send-keys for resume failed: {}",
                    String::from_utf8_lossy(&resume_status.stderr).trim()
                ));
            }

            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::AgentType;

    #[test]
    fn claude_resume_pattern_captures_full_command() {
        let pattern = AgentType::Claude.resume_command_pattern().unwrap();
        let regex = regex::Regex::new(pattern).unwrap();

        let output = "Goodbye! To resume this conversation:\nclaude --resume 01j8k9m2n3\n$ ";
        let captures = regex.captures(output).unwrap();
        assert_eq!(
            captures.get(1).unwrap().as_str(),
            "claude --resume 01j8k9m2n3"
        );
    }

    #[test]
    fn claude_resume_pattern_handles_long_session_ids() {
        let pattern = AgentType::Claude.resume_command_pattern().unwrap();
        let regex = regex::Regex::new(pattern).unwrap();

        let output = "claude --resume abc123-def456-ghi789\n";
        let captures = regex.captures(output).unwrap();
        assert_eq!(
            captures.get(1).unwrap().as_str(),
            "claude --resume abc123-def456-ghi789"
        );
    }
}
