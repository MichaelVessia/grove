use super::update_prelude::*;

impl GroveApp {
    pub(super) fn selected_task_supports_parent_agent(&self) -> bool {
        let Some(task) = self.state.selected_task() else {
            return false;
        };
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        !workspace.is_main && task.root_path != workspace.path
    }

    pub(super) fn selected_home_tab_targets_task_root(&self) -> bool {
        self.preview_tab == PreviewTab::Home && self.selected_task_supports_parent_agent()
    }

    pub(super) fn task_agent_for_selected_task(&self) -> AgentType {
        self.state
            .selected_worktree()
            .map(|worktree| worktree.agent)
            .unwrap_or(AgentType::Codex)
    }

    pub(super) fn task_init_command_for_task(&self, task: &Task) -> Option<String> {
        read_workspace_init_command(&task.root_path)
    }

    pub(super) fn task_skip_permissions_for_task(&self, task: &Task) -> bool {
        read_workspace_skip_permissions(&task.root_path).unwrap_or(self.launch_skip_permissions)
    }

    pub(super) fn project_agent_env_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Result<Vec<(String, String)>, String> {
        let Some(workspace_project_path) = workspace.project_path.as_ref() else {
            return Ok(Vec::new());
        };
        let Some(project) = self
            .projects
            .iter()
            .find(|project| refer_to_same_location(&project.path, workspace_project_path))
        else {
            return Ok(Vec::new());
        };
        let entries = match workspace.agent {
            AgentType::Claude => &project.defaults.agent_env.claude,
            AgentType::Codex => &project.defaults.agent_env.codex,
            AgentType::OpenCode => &project.defaults.agent_env.opencode,
        };
        parse_agent_env_vars_from_entries(entries).map(|vars| {
            vars.into_iter()
                .map(|entry| (entry.key, entry.value))
                .collect()
        })
    }

    pub(super) fn project_workspace_init_command_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Option<String> {
        let workspace_project_path = workspace.project_path.as_ref()?;
        let project = self
            .projects
            .iter()
            .find(|project| refer_to_same_location(&project.path, workspace_project_path))?;
        trimmed_nonempty(project.defaults.workspace_init_command.as_str())
    }

    pub(super) fn workspace_init_command_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Option<String> {
        read_workspace_init_command(&workspace.path)
            .or_else(|| self.project_workspace_init_command_for_workspace(workspace))
    }

    pub(super) fn workspace_skip_permissions_for_workspace(&self, workspace: &Workspace) -> bool {
        if let Some(skip_permissions) = read_workspace_skip_permissions(&workspace.path) {
            return skip_permissions;
        }
        if let Some(skip_permissions) =
            infer_workspace_skip_permissions(workspace.agent, &workspace.path)
        {
            return skip_permissions;
        }

        self.launch_skip_permissions
    }

    fn start_task_agent_with_options(
        &mut self,
        task: Task,
        agent: AgentType,
        prompt: Option<String>,
        init_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            return;
        }

        self.launch_skip_permissions = skip_permissions;
        if let Err(error) = write_workspace_skip_permissions(&task.root_path, skip_permissions) {
            self.session.last_tmux_error =
                Some(format!("skip permissions marker persist failed: {error}"));
        }
        if let Err(error) = write_workspace_init_command(&task.root_path, init_command.as_deref()) {
            self.session.last_tmux_error =
                Some(format!("init command marker persist failed: {error}"));
        }

        let (capture_cols, capture_rows) = self.capture_dimensions();
        let request = TaskLaunchRequest {
            task_slug: task.slug.clone(),
            task_root: task.root_path.clone(),
            agent,
            theme_name: self.theme_name,
            prompt,
            workspace_init_command: init_command.or_else(|| self.task_init_command_for_task(&task)),
            skip_permissions,
            agent_env: Vec::new(),
            capture_cols: Some(capture_cols),
            capture_rows: Some(capture_rows),
        };

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_task_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast("agent start failed");
                return;
            }

            self.apply_start_agent_completion(completion.into());
            return;
        }

        self.dialogs.start_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_task_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Process,
            );
            Msg::StartAgentCompleted(completion.into())
        }));
    }

    fn start_workspace_agent_with_options(
        &mut self,
        workspace: Workspace,
        prompt: Option<String>,
        init_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            return;
        }
        if !workspace_can_start_agent(Some(&workspace)) {
            self.show_info_toast("workspace cannot be started");
            return;
        }

        self.launch_skip_permissions = skip_permissions;
        if let Err(error) = write_workspace_skip_permissions(&workspace.path, skip_permissions) {
            self.session.last_tmux_error =
                Some(format!("skip permissions marker persist failed: {error}"));
        }
        if let Err(error) = write_workspace_init_command(&workspace.path, init_command.as_deref()) {
            self.session.last_tmux_error =
                Some(format!("init command marker persist failed: {error}"));
        }
        let agent_env = match self.project_agent_env_for_workspace(&workspace) {
            Ok(agent_env) => agent_env,
            Err(error) => {
                self.show_info_toast(format!("invalid project agent env: {error}"));
                return;
            }
        };
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let workspace_init_command =
            init_command.or_else(|| self.workspace_init_command_for_workspace(&workspace));
        let request = launch_request_for_workspace(
            &workspace,
            prompt,
            self.theme_name,
            workspace_init_command,
            skip_permissions,
            agent_env,
            Some((capture_cols, capture_rows)),
        );

        if !self.tmux_input.supports_background_launch() {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
            );
            if let Some(error) = completion.result.as_ref().err() {
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast("agent start failed");
                return;
            }

            self.apply_start_agent_completion(completion.into());
            return;
        }

        self.dialogs.start_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let completion = execute_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Process,
            );
            Msg::StartAgentCompleted(completion.into())
        }));
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
        let init_command = self.workspace_init_command_for_workspace(&workspace);
        let skip_permissions = self.workspace_skip_permissions_for_workspace(&workspace);
        self.start_workspace_agent_with_options(workspace, prompt, init_command, skip_permissions);
    }

    pub(super) fn apply_start_agent_completion(&mut self, completion: StartAgentCompletion) {
        self.dialogs.start_in_flight = false;
        let reused_existing_session = completion
            .result
            .as_ref()
            .err()
            .is_some_and(|error| tmux_launch_error_indicates_duplicate_session(error));
        let task_root_launch = self
            .state
            .tasks
            .iter()
            .any(|task| task.root_path == completion.workspace_path);

        if completion.result.is_ok() || reused_existing_session {
            if task_root_launch {
                self.session
                    .agent_sessions
                    .mark_ready(completion.session_name.clone());
            }
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
            let mut event = LogEvent::new("agent_lifecycle", "agent_started")
                .with_data("workspace", Value::from(completion.workspace_name))
                .with_data("session", Value::from(completion.session_name));
            if reused_existing_session && let Err(error) = &completion.result {
                event = event
                    .with_data("reused_existing_session", Value::from(true))
                    .with_data("error", Value::from(error.clone()));
            }
            self.telemetry.event_log.log(event);
            self.session.last_tmux_error = None;
            if reused_existing_session && task_root_launch {
                self.show_info_toast("parent agent already running");
            } else {
                self.show_success_toast("agent started");
            }
            self.poll_preview();
            return;
        }

        if let Err(error) = completion.result {
            self.session.last_tmux_error = Some(error.clone());
            self.log_tmux_error(error);
            self.show_error_toast("agent start failed");
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
                ("agent".to_string(), Value::from(dialog.agent.label())),
                (
                    "prompt_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.prompt.len())),
                ),
                (
                    "name_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.name.len())),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(dialog.start_config.skip_permissions),
                ),
                (
                    "init_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.init_command.len())),
                ),
            ],
        );

        let StartOptions {
            name,
            prompt,
            init_command,
            skip_permissions,
        } = dialog.start_config.parse_start_options();
        let options = StartOptions {
            name,
            prompt,
            init_command,
            skip_permissions,
        };
        match dialog.target {
            LaunchDialogTarget::WorkspaceTab => {
                if let Err(error) = self.launch_new_agent_tab(dialog.agent, options) {
                    self.session.last_tmux_error = Some(error.clone());
                    self.show_error_toast(format!("agent tab launch failed: {error}"));
                }
            }
            LaunchDialogTarget::ParentTask(task) => {
                self.start_task_agent_with_options(
                    task,
                    dialog.agent,
                    options.prompt,
                    options.init_command,
                    options.skip_permissions,
                );
            }
        }
    }
}
