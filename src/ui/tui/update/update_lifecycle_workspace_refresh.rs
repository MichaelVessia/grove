use super::*;

impl GroveApp {
    const MANUAL_WORKSPACE_REFRESH_COOLDOWN: Duration = Duration::from_secs(10);

    fn finalize_manual_workspace_refresh_feedback(&mut self) {
        if !self.manual_refresh_feedback_pending {
            return;
        }
        self.manual_refresh_feedback_pending = false;

        match &self.discovery_state {
            DiscoveryState::Ready => self.show_success_toast("workspace refresh complete"),
            DiscoveryState::Empty => {
                self.show_info_toast("workspace refresh complete, no workspaces found")
            }
            DiscoveryState::Error(message) => {
                self.show_error_toast(format!("workspace refresh failed: {message}"))
            }
        }
    }

    pub(super) fn request_manual_workspace_refresh(&mut self) {
        let now = Instant::now();
        if self.refresh_in_flight {
            self.show_info_toast("workspace refresh already in progress");
            return;
        }

        if let Some(last_requested_at) = self.last_manual_refresh_requested_at {
            let elapsed = now.saturating_duration_since(last_requested_at);
            if elapsed < Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN {
                let remaining = Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN.saturating_sub(elapsed);
                let remaining_seconds = remaining.as_secs().max(1);
                self.show_info_toast(format!("refresh throttled, retry in {remaining_seconds}s"));
                return;
            }
        }

        self.last_manual_refresh_requested_at = Some(now);
        self.manual_refresh_feedback_pending = true;
        self.show_info_toast("refreshing workspaces...");
        self.refresh_workspaces(None);
    }

    fn auto_launch_pending_workspace_shell(&mut self) -> bool {
        let Some(pending_path) = self.pending_auto_launch_shell_workspace_path.clone() else {
            return false;
        };
        self.pending_auto_launch_shell_workspace_path = None;

        let Some(workspace) = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == pending_path)
            .cloned()
        else {
            return false;
        };

        let session_name = shell_session_name_for_workspace(&workspace);
        let launched = self
            .ensure_workspace_shell_session_for_workspace(workspace, false, true, true)
            .is_some();
        launched || self.shell_sessions.is_in_flight(&session_name)
    }

    pub(super) fn auto_start_pending_workspace_agent(&mut self) -> bool {
        let Some(pending) = self.pending_auto_start_workspace.clone() else {
            return false;
        };
        self.pending_auto_start_workspace = None;

        let selected_matches = self
            .state
            .selected_workspace()
            .is_some_and(|workspace| workspace.path == pending.workspace_path);
        if !selected_matches {
            return false;
        }

        let StartOptions {
            prompt,
            init_command,
            skip_permissions,
        } = pending.start_config.parse_start_options();
        self.launch_skip_permissions = skip_permissions;
        self.start_selected_workspace_agent_with_options(prompt, init_command, skip_permissions);
        self.start_in_flight
    }

    pub(super) fn refresh_workspaces(&mut self, preferred_workspace_path: Option<PathBuf>) {
        if !self.tmux_input.supports_background_launch() {
            self.refresh_workspaces_sync(preferred_workspace_path);
            return;
        }

        if self.refresh_in_flight {
            return;
        }

        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let projects = self.projects.clone();
        self.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let bootstrap = bootstrap_data_for_projects(&projects);
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_path: target_path,
                bootstrap,
            })
        }));
    }

    fn refresh_workspaces_sync(&mut self, preferred_workspace_path: Option<PathBuf>) {
        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let bootstrap = bootstrap_data_for_projects(&self.projects);

        self.repo_name = bootstrap.repo_name;
        self.discovery_state = bootstrap.discovery_state;
        self.state = AppState::new(bootstrap.workspaces);
        if let Some(path) = target_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.auto_launch_pending_workspace_shell();
        let started_in_background = self.auto_start_pending_workspace_agent();
        if !started_in_background {
            self.poll_preview();
        }
        self.finalize_manual_workspace_refresh_feedback();
    }

    pub(super) fn apply_refresh_workspaces_completion(
        &mut self,
        completion: RefreshWorkspacesCompletion,
    ) {
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;

        self.repo_name = completion.bootstrap.repo_name;
        self.discovery_state = completion.bootstrap.discovery_state;
        self.state = AppState::new(completion.bootstrap.workspaces);
        if let Some(path) = completion.preferred_workspace_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.refresh_in_flight = false;
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.auto_launch_pending_workspace_shell();
        let started_in_background = self.auto_start_pending_workspace_agent();
        if !started_in_background {
            self.poll_preview();
        }
        self.finalize_manual_workspace_refresh_feedback();
    }
}
