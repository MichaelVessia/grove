use super::update_prelude::*;

struct SessionLaunchCompletionContext {
    async_launch: bool,
    workspace_name: Option<String>,
    log_tmux_error_on_failure: bool,
    poll_preview_on_ready: bool,
}

impl GroveApp {
    pub(super) fn preview_output_dimensions(&self) -> Option<(u16, u16)> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }

        let inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if inner.is_empty() || inner.width == 0 {
            return None;
        }

        let output_height = inner.height.saturating_sub(PREVIEW_METADATA_ROWS).max(1);
        Some((inner.width, output_height))
    }

    pub(super) fn capture_dimensions(&self) -> (u16, u16) {
        let capture_cols = self
            .preview_output_dimensions()
            .map_or(self.viewport_width.saturating_sub(4), |(width, _)| width)
            .max(80);
        let capture_rows = self.viewport_height.saturating_sub(4).max(1);
        (capture_cols, capture_rows)
    }

    fn session_tracker(&self, kind: SessionKind) -> &SessionTracker {
        match kind {
            SessionKind::Lazygit => &self.session.lazygit_sessions,
            SessionKind::WorkspaceShell => &self.session.shell_sessions,
        }
    }

    fn session_tracker_mut(&mut self, kind: SessionKind) -> &mut SessionTracker {
        match kind {
            SessionKind::Lazygit => &mut self.session.lazygit_sessions,
            SessionKind::WorkspaceShell => &mut self.session.shell_sessions,
        }
    }

    fn session_launch_event(kind: SessionKind) -> &'static str {
        match kind {
            SessionKind::Lazygit => "lazygit_launch",
            SessionKind::WorkspaceShell => "workspace_shell_launch",
        }
    }

    fn session_launch_failure_toast(kind: SessionKind) -> &'static str {
        match kind {
            SessionKind::Lazygit => "lazygit launch failed",
            SessionKind::WorkspaceShell => "workspace shell launch failed",
        }
    }

    fn selected_workspace_has_session(&self, kind: SessionKind, session_name: &str) -> bool {
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        match kind {
            SessionKind::Lazygit => git_session_name_for_workspace(workspace) == session_name,
            SessionKind::WorkspaceShell => self
                .workspace_tabs
                .get(workspace.path.as_path())
                .is_some_and(|tabs| {
                    tabs.tabs.iter().any(|tab| {
                        tab.kind == WorkspaceTabKind::Shell
                            && tab.session_name.as_deref() == Some(session_name)
                    })
                }),
        }
    }

    fn should_poll_preview_after_launch(&self, kind: SessionKind) -> bool {
        match kind {
            SessionKind::Lazygit => self.preview_tab == PreviewTab::Git,
            SessionKind::WorkspaceShell => {
                matches!(self.preview_tab, PreviewTab::Agent | PreviewTab::Shell)
            }
        }
    }

    fn queue_session_launch_task(
        &mut self,
        kind: SessionKind,
        session_name: String,
        launch_request: ShellLaunchRequest,
    ) {
        let completion_session = session_name.clone();
        self.queue_cmd(Cmd::task(move || {
            let started_at = Instant::now();
            let (_, result) = execute_shell_launch_request_for_mode(
                &launch_request,
                CommandExecutionMode::Process,
            );
            let duration_ms =
                GroveApp::duration_millis(Instant::now().saturating_duration_since(started_at));
            match kind {
                SessionKind::Lazygit => Msg::LazygitLaunchCompleted(LazygitLaunchCompletion {
                    session_name: completion_session,
                    duration_ms,
                    result,
                }),
                SessionKind::WorkspaceShell => {
                    Msg::WorkspaceShellLaunchCompleted(WorkspaceShellLaunchCompletion {
                        session_name: completion_session,
                        duration_ms,
                        result,
                    })
                }
            }
        }));
    }

    fn complete_session_launch(
        &mut self,
        kind: SessionKind,
        session_name: String,
        duration_ms: u64,
        result: Result<(), String>,
        context: SessionLaunchCompletionContext,
    ) -> bool {
        let mut completion_fields = vec![
            ("session".to_string(), Value::from(session_name.clone())),
            ("multiplexer".to_string(), Value::from("tmux")),
            ("async".to_string(), Value::from(context.async_launch)),
            ("duration_ms".to_string(), Value::from(duration_ms)),
        ];
        if let Some(workspace_name) = context.workspace_name {
            completion_fields.push(("workspace".to_string(), Value::from(workspace_name)));
        }

        let is_success = match &result {
            Ok(()) => true,
            Err(error) => tmux_launch_error_indicates_duplicate_session(error),
        };
        completion_fields.push(("ok".to_string(), Value::from(is_success)));

        if is_success {
            if let Err(error) = &result {
                completion_fields.push(("reused_existing_session".to_string(), Value::from(true)));
                completion_fields.push(("error".to_string(), Value::from(error.clone())));
            }
            self.session.last_tmux_error = None;
            self.session_tracker_mut(kind)
                .mark_ready(session_name.clone());
            self.log_event_with_fields(
                Self::session_launch_event(kind),
                "completed",
                completion_fields,
            );
            if context.poll_preview_on_ready
                && self.selected_workspace_has_session(kind, &session_name)
                && self.should_poll_preview_after_launch(kind)
            {
                self.poll_preview();
            }
            return true;
        }

        let error = result.unwrap_err();
        completion_fields.push(("error".to_string(), Value::from(error.clone())));
        self.log_event_with_fields(
            Self::session_launch_event(kind),
            "completed",
            completion_fields,
        );
        self.session.last_tmux_error = Some(error.clone());
        if context.log_tmux_error_on_failure {
            self.log_tmux_error(error);
        }
        self.session_tracker_mut(kind).mark_failed(session_name);
        self.show_error_toast(Self::session_launch_failure_toast(kind));
        false
    }

    fn ensure_session_for_workspace(
        &mut self,
        kind: SessionKind,
        workspace: &Workspace,
        command: String,
        retry_failed: bool,
        log_tmux_error_on_sync_failure: bool,
    ) -> Option<String> {
        let session_name = match kind {
            SessionKind::Lazygit => git_session_name_for_workspace(workspace),
            SessionKind::WorkspaceShell => shell_session_name_for_workspace(workspace),
        };

        if self.session_tracker(kind).is_ready(&session_name) {
            return Some(session_name);
        }
        if self.session_tracker(kind).is_failed(&session_name) {
            if !retry_failed {
                return None;
            }
            self.session_tracker_mut(kind).retry_failed(&session_name);
        }
        if self.session_tracker(kind).is_in_flight(&session_name) {
            return None;
        }

        let (capture_cols, capture_rows) = self.capture_dimensions();
        let workspace_init_command = self.workspace_init_command_for_workspace(workspace);
        let launch_request = shell_launch_request_for_workspace(
            workspace,
            session_name.clone(),
            command,
            workspace_init_command,
            Some(capture_cols),
            Some(capture_rows),
        );
        let async_launch = self.tmux_input.supports_background_launch();
        let mut started_fields = vec![
            ("session".to_string(), Value::from(session_name.clone())),
            ("multiplexer".to_string(), Value::from("tmux")),
            ("async".to_string(), Value::from(async_launch)),
            ("capture_cols".to_string(), Value::from(capture_cols)),
            ("capture_rows".to_string(), Value::from(capture_rows)),
        ];
        if kind == SessionKind::WorkspaceShell {
            started_fields.push(("workspace".to_string(), Value::from(workspace.name.clone())));
        }
        self.log_event_with_fields(Self::session_launch_event(kind), "started", started_fields);

        if async_launch {
            self.session_tracker_mut(kind)
                .mark_in_flight(session_name.clone());
            self.queue_session_launch_task(kind, session_name, launch_request);
            return None;
        }

        let launch_started_at = Instant::now();
        let (_, launch_result) = execute_shell_launch_request_for_mode(
            &launch_request,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        let duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(launch_started_at));
        let workspace_name = if kind == SessionKind::WorkspaceShell {
            Some(workspace.name.clone())
        } else {
            None
        };
        if self.complete_session_launch(
            kind,
            session_name.clone(),
            duration_ms,
            launch_result,
            SessionLaunchCompletionContext {
                async_launch: false,
                workspace_name,
                log_tmux_error_on_failure: log_tmux_error_on_sync_failure,
                poll_preview_on_ready: false,
            },
        ) {
            return Some(session_name);
        }

        None
    }

    pub(super) fn ensure_lazygit_session_for_selected_workspace(&mut self) -> Option<String> {
        let workspace = self.state.selected_workspace()?.clone();
        self.ensure_session_for_workspace(
            SessionKind::Lazygit,
            &workspace,
            self.session.lazygit_command.clone(),
            false,
            false,
        )
    }

    pub(super) fn selected_agent_preview_session_if_ready(&self) -> Option<String> {
        let tab = self.selected_active_tab()?;
        if tab.kind != WorkspaceTabKind::Agent {
            return None;
        }
        let session_name = tab.session_name.clone()?;
        if self.session.agent_sessions.is_ready(&session_name) {
            return Some(session_name);
        }
        None
    }

    pub(super) fn selected_shell_preview_session_if_ready(&self) -> Option<String> {
        let session_name = self.selected_shell_tab_session_name()?;
        self.session
            .shell_sessions
            .is_ready(&session_name)
            .then_some(session_name)
    }

    pub(super) fn can_enter_interactive_session(&self) -> bool {
        match self.preview_tab {
            PreviewTab::Home => false,
            PreviewTab::Git => {
                workspace_can_enter_interactive(self.state.selected_workspace(), true)
            }
            PreviewTab::Shell => self.selected_shell_preview_session_if_ready().is_some(),
            PreviewTab::Agent => self.selected_agent_preview_session_if_ready().is_some(),
        }
    }

    pub(super) fn ensure_agent_preview_session_for_interactive(&mut self) -> Option<String> {
        if let Some(session_name) = self.selected_agent_preview_session_if_ready() {
            return Some(session_name);
        }
        None
    }

    pub(super) fn ensure_shell_preview_session_for_interactive(&mut self) -> Option<String> {
        self.selected_shell_preview_session_if_ready()
    }

    pub(super) fn prepare_live_preview_session(&mut self) -> Option<LivePreviewTarget> {
        let session_name = match self.preview_tab {
            PreviewTab::Home => return None,
            PreviewTab::Git => self.ensure_lazygit_session_for_selected_workspace()?,
            PreviewTab::Shell => self.selected_shell_preview_session_if_ready()?,
            PreviewTab::Agent => self.selected_agent_preview_session_if_ready()?,
        };
        Some(LivePreviewTarget {
            session_name,
            include_escape_sequences: true,
        })
    }

    fn handle_async_session_launch_completed(
        &mut self,
        kind: SessionKind,
        session_name: String,
        duration_ms: u64,
        result: Result<(), String>,
    ) {
        self.complete_session_launch(
            kind,
            session_name,
            duration_ms,
            result,
            SessionLaunchCompletionContext {
                async_launch: true,
                workspace_name: None,
                log_tmux_error_on_failure: true,
                poll_preview_on_ready: true,
            },
        );
    }

    pub(super) fn handle_lazygit_launch_completed(&mut self, completion: LazygitLaunchCompletion) {
        let LazygitLaunchCompletion {
            session_name,
            duration_ms,
            result,
        } = completion;
        self.handle_async_session_launch_completed(
            SessionKind::Lazygit,
            session_name,
            duration_ms,
            result,
        );
    }

    pub(super) fn handle_workspace_shell_launch_completed(
        &mut self,
        completion: WorkspaceShellLaunchCompletion,
    ) {
        let WorkspaceShellLaunchCompletion {
            session_name,
            duration_ms,
            result,
        } = completion;
        self.handle_async_session_launch_completed(
            SessionKind::WorkspaceShell,
            session_name,
            duration_ms,
            result,
        );
    }

    pub(super) fn interactive_target_session(&self) -> Option<String> {
        self.session
            .interactive
            .as_ref()
            .map(|state| state.target_session.clone())
    }

    fn shell_session_status_summary(&self, workspace: &Workspace) -> Option<String> {
        let shell_session_name = self.selected_shell_tab_session_name()?;
        if self
            .session
            .shell_sessions
            .is_in_flight(&shell_session_name)
        {
            return Some(format!("Starting shell session for {}...", workspace.name));
        }
        if self.session.shell_sessions.is_failed(&shell_session_name) {
            return Some(format!(
                "Shell session failed for {}.\nPress Enter to retry session launch.",
                workspace.name
            ));
        }
        if workspace.is_orphaned {
            return Some(format!("Reconnecting session for {}...", workspace.name));
        }
        None
    }

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                let has_running_tabs = self
                    .workspace_tabs
                    .get(workspace.path.as_path())
                    .is_some_and(|tabs| {
                        tabs.tabs.iter().any(|tab| {
                            tab.kind != WorkspaceTabKind::Home
                                && tab.state == WorkspaceTabRuntimeState::Running
                        })
                    });
                if self.preview_tab == PreviewTab::Home {
                    if workspace.is_main {
                        return self.main_worktree_splash();
                    }
                    return self.workspace_home_splash(workspace, has_running_tabs);
                }
                if self.preview_tab == PreviewTab::Shell {
                    return self
                        .shell_session_status_summary(workspace)
                        .unwrap_or_else(|| {
                            format!("Preparing shell session for {}...", workspace.name)
                        });
                }

                if workspace.is_main && !has_running_tabs {
                    return self.main_worktree_splash();
                }
                if workspace.is_main {
                    return "Connecting to main workspace session...".to_string();
                }

                self.shell_session_status_summary(workspace)
                    .unwrap_or_else(|| format!("Preparing session for {}...", workspace.name))
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn workspace_home_splash(&self, workspace: &Workspace, has_running_tabs: bool) -> String {
        let running_note = if has_running_tabs {
            "Sessions are running in other tabs."
        } else {
            "No sessions running in this workspace."
        };
        [
            "Workspace Home".to_string(),
            format!("{} · {}", workspace.name, workspace.path.display()),
            String::new(),
            running_note.to_string(),
            "Press 'a' for agent tabs, 's' for shell tabs, 'g' for git tab.".to_string(),
            "Use 'x' to kill active tab session, 'X' to close active tab.".to_string(),
        ]
        .join("\n")
    }

    fn main_worktree_splash(&self) -> String {
        const G: &str = "\x1b[38;2;166;227;161m";
        const T: &str = "\x1b[38;2;250;179;135m";
        const R: &str = "\x1b[0m";

        [
            String::new(),
            format!("{G}                    .@@@.{R}"),
            format!("{G}                 .@@@@@@@@@.{R}"),
            format!("{G}               .@@@@@@@@@@@@@.{R}"),
            format!("{G}    .@@@.     @@@@@@@@@@@@@@@@@        .@@.{R}"),
            format!("{G}  .@@@@@@@.  @@@@@@@@@@@@@@@@@@@    .@@@@@@@@.{R}"),
            format!("{G} @@@@@@@@@@@ @@@@@@@@@@@@@@@@@@@@  @@@@@@@@@@@@@{R}"),
            format!("{G} @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@'{R}"),
            format!("{G}    '@@@@@@@@  '@@@@@@@@@@@@@@@' @@@@@@@@@@@@@@'{R}"),
            format!("{G}      '@@@@'     '@@@@@@@@@@@'    '@@@@@@@@@@'{R}"),
            format!("         {T}||{R}        {G}'@@@@@@@'{R}        {G}'@@@@'{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("        {T}/||\\{R}         {T}/|||\\{R}            {T}/||\\{R}"),
            String::new(),
            "Base Worktree".to_string(),
            String::new(),
            "This is your repo root.".to_string(),
            "Create focused workspaces from here when you start new work.".to_string(),
            String::new(),
            "--------------------------------------------------".to_string(),
            String::new(),
            "Press 'n' to create a workspace.".to_string(),
            "Then use 'a' for agent tabs, 's' for shell tabs, 'g' for git tab.".to_string(),
            String::new(),
            "Each workspace has its own directory and branch.".to_string(),
            "Run agents in parallel without branch hopping.".to_string(),
        ]
        .join("\n")
    }

    pub(super) fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }
}
