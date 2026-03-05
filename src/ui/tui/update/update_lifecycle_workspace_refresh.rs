use super::update_prelude::*;

impl GroveApp {
    const MANUAL_WORKSPACE_REFRESH_COOLDOWN: Duration = Duration::from_secs(10);

    fn finalize_manual_workspace_refresh_feedback(&mut self) {
        if !self.dialogs.manual_refresh_feedback_pending {
            return;
        }
        self.dialogs.manual_refresh_feedback_pending = false;

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
        if self.dialogs.refresh_in_flight {
            self.show_info_toast("workspace refresh already in progress");
            return;
        }

        if let Some(last_requested_at) = self.dialogs.last_manual_refresh_requested_at {
            let elapsed = now.saturating_duration_since(last_requested_at);
            if elapsed < Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN {
                let remaining = Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN.saturating_sub(elapsed);
                let remaining_seconds = remaining.as_secs().max(1);
                self.show_info_toast(format!("refresh throttled, retry in {remaining_seconds}s"));
                return;
            }
        }

        self.dialogs.last_manual_refresh_requested_at = Some(now);
        self.dialogs.manual_refresh_feedback_pending = true;
        self.show_info_toast("refreshing workspaces...");
        self.refresh_workspaces(None);
    }

    pub(super) fn refresh_workspaces(&mut self, preferred_workspace_path: Option<PathBuf>) {
        if !self.tmux_input.supports_background_launch() {
            self.refresh_workspaces_sync(preferred_workspace_path);
            return;
        }

        if self.dialogs.refresh_in_flight {
            return;
        }

        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let projects = self.projects.clone();
        self.dialogs.refresh_in_flight = true;
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
        self.rebuild_workspace_tabs_from_tmux_metadata();
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
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
        self.rebuild_workspace_tabs_from_tmux_metadata();
        self.dialogs.refresh_in_flight = false;
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
        self.finalize_manual_workspace_refresh_feedback();
    }
}
