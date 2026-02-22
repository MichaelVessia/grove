use super::*;

impl GroveApp {
    pub(super) fn project_index_for_workspace(&self, workspace: &Workspace) -> Option<usize> {
        let workspace_project_path = workspace.project_path.as_ref()?;
        let mut matching_indexes = self
            .projects
            .iter()
            .enumerate()
            .filter_map(|(index, project)| {
                if refer_to_same_location(project.path.as_path(), workspace_project_path.as_path())
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<Vec<usize>>();
        if matching_indexes.is_empty() {
            return None;
        }
        if matching_indexes.len() == 1 {
            return matching_indexes.pop();
        }

        if let Some(workspace_project_name) = workspace.project_name.as_deref()
            && let Some(index) = matching_indexes.iter().copied().find(|index| {
                self.projects
                    .get(*index)
                    .is_some_and(|project| project.name == workspace_project_name)
            })
        {
            return Some(index);
        }

        matching_indexes.first().copied()
    }

    fn project_for_workspace(&self, workspace: &Workspace) -> Option<&ProjectConfig> {
        let project_index = self.project_index_for_workspace(workspace)?;
        self.projects.get(project_index)
    }

    fn remote_socket_path_for_profile(&self, profile_name: &str) -> Option<PathBuf> {
        let profile = self
            .remote_profiles
            .iter()
            .find(|candidate| candidate.name == profile_name)?;
        Some(normalized_socket_path(profile.remote_socket_path.as_str()))
    }

    pub(super) fn daemon_socket_path_for_project(
        &self,
        project: &ProjectConfig,
    ) -> Option<PathBuf> {
        match &project.target {
            ProjectTarget::Local => self.daemon_socket_path.clone(),
            ProjectTarget::Remote { profile } => {
                self.remote_socket_path_for_profile(profile.as_str())
            }
        }
    }

    pub(super) fn daemon_socket_path_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Option<PathBuf> {
        if let Some(project) = self.project_for_workspace(workspace) {
            return self.daemon_socket_path_for_project(project);
        }

        self.daemon_socket_path.clone()
    }

    pub(super) fn daemon_socket_path_for_workspace_path(
        &self,
        workspace_path: &std::path::Path,
    ) -> Option<PathBuf> {
        if let Some(workspace) = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)
        {
            return self.daemon_socket_path_for_workspace(workspace);
        }

        self.daemon_socket_path.clone()
    }

    /// Returns a daemon socket path only for remote workspaces.
    /// Session operations (capture, send-keys, resize, paste) must execute
    /// on the machine where the tmux session lives. Local workspaces run
    /// tmux locally, so they bypass the daemon for session ops.
    pub(super) fn remote_session_socket_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Option<PathBuf> {
        let project = self.project_for_workspace(workspace)?;
        match &project.target {
            ProjectTarget::Local => None,
            ProjectTarget::Remote { profile } => {
                self.remote_socket_path_for_profile(profile.as_str())
            }
        }
    }

    pub(super) fn remote_session_socket_for_workspace_path(
        &self,
        workspace_path: &std::path::Path,
    ) -> Option<PathBuf> {
        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)?;
        self.remote_session_socket_for_workspace(workspace)
    }

    fn ensure_remote_profile_available(&mut self, profile: &str, operation: &str) -> bool {
        let status = self.remote_status_for(profile);
        let active_matches = self.active_remote_profile.as_deref() == Some(profile);
        if active_matches && status == RemoteConnectionState::Connected {
            return true;
        }

        self.last_tmux_error = Some(format!(
            "REMOTE_UNAVAILABLE: profile '{profile}' is {}",
            status.label()
        ));
        self.show_error_toast(format!(
            "{operation} failed: REMOTE_UNAVAILABLE ({profile})"
        ));
        false
    }

    pub(super) fn ensure_project_backend_available(
        &mut self,
        project: &ProjectConfig,
        operation: &str,
    ) -> bool {
        let profile = match &project.target {
            ProjectTarget::Remote { profile } => profile.as_str(),
            ProjectTarget::Local => return true,
        };

        self.ensure_remote_profile_available(profile, operation)
    }

    pub(super) fn ensure_workspace_backend_available(
        &mut self,
        workspace: &Workspace,
        operation: &str,
    ) -> bool {
        let profile = match self.project_for_workspace(workspace) {
            Some(ProjectConfig {
                target: ProjectTarget::Remote { profile },
                ..
            }) => profile.clone(),
            _ => return true,
        };

        self.ensure_remote_profile_available(profile.as_str(), operation)
    }

    pub(super) fn selected_workspace_name(&self) -> Option<String> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
    }

    pub(super) fn selected_workspace_path(&self) -> Option<PathBuf> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.path.clone())
    }

    pub(super) fn queue_cmd(&mut self, cmd: Cmd<Msg>) {
        if matches!(cmd, Cmd::None) {
            return;
        }

        self.deferred_cmds.push(cmd);
    }

    pub(super) fn merge_deferred_cmds(&mut self, cmd: Cmd<Msg>) -> Cmd<Msg> {
        let deferred_cmds = std::mem::take(&mut self.deferred_cmds);
        if deferred_cmds.is_empty() {
            return cmd;
        }

        if matches!(cmd, Cmd::Quit) {
            return Cmd::Quit;
        }

        if matches!(cmd, Cmd::None) {
            return Cmd::batch(deferred_cmds);
        }

        let mut merged = Vec::with_capacity(deferred_cmds.len().saturating_add(1));
        merged.push(cmd);
        merged.extend(deferred_cmds);
        Cmd::batch(merged)
    }

    pub(super) fn next_input_seq(&mut self) -> u64 {
        let seq = self.input_seq_counter;
        self.input_seq_counter = self.input_seq_counter.saturating_add(1);
        seq
    }

    pub(super) fn init_model(&mut self) -> Cmd<Msg> {
        let daemon_socket = self
            .daemon_socket_path
            .as_ref()
            .map(|path| path.display().to_string());
        let active_remote_profile = self.active_remote_profile.clone();
        let term = std::env::var("TERM").unwrap_or_default();
        self.log_event_with_fields(
            "app",
            "session_started",
            [
                (
                    "pid".to_string(),
                    Value::from(u64::from(std::process::id())),
                ),
                ("term".to_string(), Value::from(term)),
                (
                    "tmux".to_string(),
                    Value::from(std::env::var_os("TMUX").is_some()),
                ),
                (
                    "debug_record".to_string(),
                    Value::from(self.debug_record_start_ts.is_some()),
                ),
                (
                    "daemon_socket".to_string(),
                    daemon_socket.map(Value::from).unwrap_or(Value::Null),
                ),
                (
                    "active_remote_profile".to_string(),
                    active_remote_profile
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                ),
            ],
        );
        self.reconnect_active_profile_on_startup();
        self.poll_preview();
        let next_tick_cmd = self.schedule_next_tick();
        let init_cmd = Cmd::batch(vec![
            next_tick_cmd,
            Cmd::set_mouse_capture(self.mouse_capture_enabled),
        ]);
        self.merge_deferred_cmds(init_cmd)
    }
}

fn normalized_socket_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(stripped);
    }

    PathBuf::from(raw)
}
