use super::*;

impl GroveApp {
    fn project_for_workspace(&self, workspace: &Workspace) -> Option<&ProjectConfig> {
        let workspace_project_path = workspace.project_path.as_ref()?;
        self.projects.iter().find(|project| {
            refer_to_same_location(project.path.as_path(), workspace_project_path.as_path())
        })
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

        let status = self.remote_status_for(profile.as_str());
        let active_matches = self.active_remote_profile.as_deref() == Some(profile.as_str());
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
        self.poll_preview();
        let next_tick_cmd = self.schedule_next_tick();
        let init_cmd = Cmd::batch(vec![
            next_tick_cmd,
            Cmd::set_mouse_capture(self.mouse_capture_enabled),
        ]);
        self.merge_deferred_cmds(init_cmd)
    }
}
