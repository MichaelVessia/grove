use super::*;

fn compute_attention_markers(workspaces: &[Workspace]) -> HashMap<PathBuf, String> {
    let mut markers = HashMap::new();
    for workspace in workspaces {
        if !workspace.supported_agent || !workspace.status.has_session() {
            continue;
        }
        if let Some(marker) =
            latest_assistant_attention_marker(workspace.agent, workspace.path.as_path())
        {
            markers.insert(workspace.path.clone(), marker);
        }
    }
    markers
}

impl GroveApp {
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
            pre_launch_command,
            skip_permissions,
        } = pending.start_config.parse_start_options();
        self.launch_skip_permissions = skip_permissions;
        self.start_selected_workspace_agent_with_options(
            prompt,
            pre_launch_command,
            skip_permissions,
        );
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

        let projects = self.projects.clone();
        let remote_profiles = self.remote_profiles.clone();
        let daemon_socket_path = self.daemon_socket_path.clone();
        self.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let bootstrap = bootstrap_data_for_projects_with_transport(
                &projects,
                daemon_socket_path.as_deref(),
                &remote_profiles,
            );
            let attention_markers = compute_attention_markers(&bootstrap.workspaces);
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_path,
                bootstrap,
                attention_markers,
            })
        }));
    }

    fn refresh_workspaces_sync(&mut self, preferred_workspace_path: Option<PathBuf>) {
        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let bootstrap = bootstrap_data_for_projects_with_transport(
            &self.projects,
            self.daemon_socket_path.as_deref(),
            &self.remote_profiles,
        );

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
    }

    pub(super) fn apply_refresh_workspaces_completion(
        &mut self,
        completion: RefreshWorkspacesCompletion,
    ) {
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let current_selection = completion
            .preferred_workspace_path
            .or_else(|| self.selected_workspace_path());

        self.repo_name = completion.bootstrap.repo_name;
        self.discovery_state = completion.bootstrap.discovery_state;
        self.state = AppState::new(completion.bootstrap.workspaces);
        if let Some(path) = current_selection
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
        self.reconcile_workspace_attention_with_markers(completion.attention_markers);
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.auto_launch_pending_workspace_shell();
        let started_in_background = self.auto_start_pending_workspace_agent();
        if !started_in_background {
            self.poll_preview();
        }
    }
}
