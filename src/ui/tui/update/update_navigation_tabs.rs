use super::update_prelude::*;

const TMUX_TAB_METADATA_WORKSPACE_PATH_KEY: &str = "@grove_workspace_path";
const TMUX_TAB_METADATA_KIND_KEY: &str = "@grove_tab_kind";
const TMUX_TAB_METADATA_TITLE_KEY: &str = "@grove_tab_title";
const TMUX_TAB_METADATA_AGENT_KEY: &str = "@grove_tab_agent";
const TMUX_TAB_METADATA_ID_KEY: &str = "@grove_tab_id";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestoredTmuxTabMetadata {
    session_name: String,
    workspace_path: PathBuf,
    kind: WorkspaceTabKind,
    title: String,
    agent_type: Option<AgentType>,
    tab_id: u64,
}

impl GroveApp {
    fn home_tab_title_for_workspace(&self, workspace: &Workspace) -> &'static str {
        if workspace.is_main {
            return WorkspaceTabKind::Home.label();
        }
        workspace
            .task_slug
            .as_deref()
            .and_then(|task_slug| self.state.tasks.iter().find(|task| task.slug == task_slug))
            .map(|_| "Task Home")
            .unwrap_or_else(|| WorkspaceTabKind::Home.label())
    }

    fn sync_home_tab_titles(&mut self) {
        let titles = self
            .state
            .workspaces
            .iter()
            .map(|workspace| {
                (
                    workspace.path.clone(),
                    self.home_tab_title_for_workspace(workspace),
                )
            })
            .collect::<Vec<(PathBuf, &str)>>();

        for (workspace_path, home_title) in titles {
            if let Some(tabs) = self.workspace_tabs.get_mut(workspace_path.as_path()) {
                tabs.set_home_title(home_title);
            }
        }
    }

    pub(super) fn sync_workspace_tab_maps(&mut self) {
        let workspace_paths = self
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.path.clone())
            .collect::<std::collections::HashSet<PathBuf>>();

        self.workspace_tabs
            .retain(|path, _| workspace_paths.contains(path));
        self.last_agent_selection
            .retain(|path, _| workspace_paths.contains(path));

        for workspace in &self.state.workspaces {
            self.workspace_tabs
                .entry(workspace.path.clone())
                .or_default()
                .ensure_home_tab();
            self.last_agent_selection
                .entry(workspace.path.clone())
                .or_insert(workspace.agent);
        }

        self.sync_home_tab_titles();
        self.sync_preview_tab_from_active_workspace_tab();
    }

    pub(super) fn rebuild_workspace_tabs_from_tmux_metadata(&mut self) {
        let previous_last_agent = self.last_agent_selection.clone();
        self.workspace_tabs = self
            .state
            .workspaces
            .iter()
            .map(|workspace| (workspace.path.clone(), WorkspaceTabsState::new()))
            .collect::<std::collections::HashMap<PathBuf, WorkspaceTabsState>>();
        self.last_agent_selection = self
            .state
            .workspaces
            .iter()
            .map(|workspace| {
                (
                    workspace.path.clone(),
                    previous_last_agent
                        .get(workspace.path.as_path())
                        .copied()
                        .unwrap_or(workspace.agent),
                )
            })
            .collect::<std::collections::HashMap<PathBuf, AgentType>>();
        self.session.agent_sessions = SessionTracker::default();
        self.session.shell_sessions = SessionTracker::default();
        self.session.lazygit_sessions = SessionTracker::default();

        let metadata_rows = match self.tmux_input.list_sessions_with_tab_metadata() {
            Ok(rows) => rows,
            Err(error) => {
                self.log_event_with_fields(
                    "tab_restore",
                    "session_query_failed",
                    [("error".to_string(), Value::from(error.to_string()))],
                );
                self.sync_preview_tab_from_active_workspace_tab();
                return;
            }
        };
        let session_names = metadata_rows
            .lines()
            .filter_map(|row| row.split('\t').next())
            .map(str::trim)
            .filter(|session_name| !session_name.is_empty())
            .map(ToOwned::to_owned)
            .collect::<std::collections::HashSet<String>>();

        let mut skipped_invalid_metadata: u32 = 0;
        let mut skipped_workspace_not_found: u32 = 0;
        let mut skipped_insert_rejected: u32 = 0;
        let mut workspace_not_found_details = Vec::new();
        let mut invalid_metadata_details = Vec::new();
        let mut insert_rejected_details = Vec::new();

        for row in metadata_rows.lines() {
            if trimmed_nonempty(row).is_none() {
                continue;
            }
            let metadata = match Self::parse_tmux_tab_metadata_row(row) {
                Ok(parsed) => parsed,
                Err(reason) => {
                    if let Some(recovered) = self.recover_legacy_git_tmux_tab_metadata_row(row) {
                        recovered
                    } else if Self::has_no_grove_metadata(row) {
                        // Session has no @grove_* variables set at all — either a
                        // non-grove tmux session or a stale grove session that lost
                        // its metadata. Silently skip rather than warning the user.
                        continue;
                    } else {
                        self.log_event_with_fields(
                            "tab_restore",
                            "metadata_ignored",
                            [
                                ("reason".to_string(), Value::from(reason)),
                                ("row".to_string(), Value::from(row.to_string())),
                            ],
                        );
                        invalid_metadata_details
                            .push(Self::format_invalid_restore_detail(row, "invalid metadata"));
                        skipped_invalid_metadata += 1;
                        continue;
                    }
                }
            };

            let Some(workspace_path) = self
                .state
                .workspaces
                .iter()
                .find(|workspace| workspace.path == metadata.workspace_path)
                .map(|workspace| workspace.path.clone())
            else {
                self.log_event_with_fields(
                    "tab_restore",
                    "metadata_ignored",
                    [
                        (
                            "reason".to_string(),
                            Value::from("workspace_not_found".to_string()),
                        ),
                        (
                            "session".to_string(),
                            Value::from(metadata.session_name.clone()),
                        ),
                    ],
                );
                workspace_not_found_details.push(format!(
                    "workspace not found: {} -> {}",
                    metadata.session_name,
                    metadata.workspace_path.display()
                ));
                skipped_workspace_not_found += 1;
                continue;
            };

            let restored_tab = WorkspaceTab {
                id: metadata.tab_id,
                kind: metadata.kind,
                title: metadata.title,
                session_name: Some(metadata.session_name.clone()),
                agent_type: metadata.agent_type,
                state: WorkspaceTabRuntimeState::Running,
            };
            let inserted = self
                .workspace_tabs
                .get_mut(workspace_path.as_path())
                .is_some_and(|tabs| tabs.insert_restored_tab(restored_tab));
            if !inserted {
                self.log_event_with_fields(
                    "tab_restore",
                    "metadata_ignored",
                    [
                        (
                            "reason".to_string(),
                            Value::from("tab_insert_rejected".to_string()),
                        ),
                        (
                            "session".to_string(),
                            Value::from(metadata.session_name.clone()),
                        ),
                    ],
                );
                insert_rejected_details.push(format!(
                    "insert rejected: {} -> {}",
                    metadata.session_name,
                    metadata.workspace_path.display()
                ));
                skipped_insert_rejected += 1;
                continue;
            }

            if let Some(agent_type) = metadata.agent_type {
                self.last_agent_selection
                    .insert(workspace_path.clone(), agent_type);
            }
            match metadata.kind {
                WorkspaceTabKind::Agent => {
                    self.session
                        .agent_sessions
                        .mark_ready(metadata.session_name);
                }
                WorkspaceTabKind::Shell => {
                    self.session
                        .shell_sessions
                        .mark_ready(metadata.session_name);
                }
                WorkspaceTabKind::Git => {
                    self.session
                        .lazygit_sessions
                        .mark_ready(metadata.session_name);
                }
                WorkspaceTabKind::Home => {}
            }
        }

        for task in &self.state.tasks {
            let session_name = session_name_for_task(&task.slug);
            if session_names.contains(&session_name) {
                self.session.agent_sessions.mark_ready(session_name);
            }
        }

        let total_skipped =
            skipped_invalid_metadata + skipped_workspace_not_found + skipped_insert_rejected;
        if total_skipped > 0 {
            let message = Self::format_skipped_sessions_warning(
                total_skipped,
                skipped_workspace_not_found,
                skipped_invalid_metadata,
                skipped_insert_rejected,
            );
            self.show_warning_toast(message);
            for detail in workspace_not_found_details
                .into_iter()
                .chain(invalid_metadata_details)
                .chain(insert_rejected_details)
            {
                self.show_warning_toast(detail);
            }
        }

        self.sync_home_tab_titles();
        self.sync_preview_tab_from_active_workspace_tab();
    }

    pub(super) fn format_skipped_sessions_warning(
        total: u32,
        workspace_not_found: u32,
        invalid_metadata: u32,
        insert_rejected: u32,
    ) -> String {
        let mut parts = Vec::new();
        if workspace_not_found > 0 {
            parts.push(format!("{workspace_not_found} workspace not found"));
        }
        if invalid_metadata > 0 {
            parts.push(format!("{invalid_metadata} invalid metadata"));
        }
        if insert_rejected > 0 {
            parts.push(format!("{insert_rejected} insert rejected"));
        }
        format!(
            "{total} tmux session{} skipped during restore ({})",
            if total == 1 { "" } else { "s" },
            parts.join(", "),
        )
    }

    fn format_invalid_restore_detail(row: &str, reason: &str) -> String {
        let mut fields = row.split('\t');
        let session_name = fields
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let workspace_path = fields
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        match (session_name, workspace_path) {
            (Some(session_name), Some(workspace_path)) => {
                format!("{reason}: {session_name} -> {workspace_path}")
            }
            (Some(session_name), None) => format!("{reason}: {session_name}"),
            _ => format!("{reason}: {row}"),
        }
    }

    pub(super) fn selected_workspace_tabs_state(&self) -> Option<&WorkspaceTabsState> {
        let workspace = self.state.selected_workspace()?;
        self.workspace_tabs.get(workspace.path.as_path())
    }

    pub(super) fn selected_workspace_tabs_state_mut(&mut self) -> Option<&mut WorkspaceTabsState> {
        let workspace_path = self.state.selected_workspace()?.path.clone();
        self.workspace_tabs.get_mut(workspace_path.as_path())
    }

    pub(super) fn selected_active_tab(&self) -> Option<&WorkspaceTab> {
        self.selected_workspace_tabs_state()?.active_tab()
    }

    pub(super) fn workspace_path_for_session(&self, session_name: &str) -> Option<PathBuf> {
        self.workspace_tabs
            .iter()
            .find(|(_, tabs)| {
                tabs.tabs
                    .iter()
                    .any(|tab| tab.session_name.as_deref() == Some(session_name))
            })
            .map(|(workspace_path, _)| workspace_path.clone())
    }

    fn tab_is_running_agent(tab: &WorkspaceTab) -> bool {
        tab.kind == WorkspaceTabKind::Agent && tab.state == WorkspaceTabRuntimeState::Running
    }

    pub(super) fn workspace_running_agent_session_for_status_poll(
        &self,
        workspace_path: &Path,
        excluded_session: Option<&str>,
    ) -> Option<String> {
        let tabs = self.workspace_tabs.get(workspace_path)?;
        if let Some(active_session) = tabs
            .active_tab()
            .filter(|tab| Self::tab_is_running_agent(tab))
            .and_then(|tab| tab.session_name.as_deref())
            .filter(|session_name| Some(*session_name) != excluded_session)
        {
            return Some(active_session.to_string());
        }

        tabs.tabs
            .iter()
            .filter(|tab| Self::tab_is_running_agent(tab))
            .filter_map(|tab| tab.session_name.as_deref())
            .find(|session_name| Some(*session_name) != excluded_session)
            .map(str::to_string)
    }

    pub(super) fn workspace_has_running_agent_tab_excluding_session(
        &self,
        workspace_path: &Path,
        excluded_session: &str,
    ) -> bool {
        self.workspace_tabs.get(workspace_path).is_some_and(|tabs| {
            tabs.tabs.iter().any(|tab| {
                Self::tab_is_running_agent(tab)
                    && tab.session_name.as_deref() != Some(excluded_session)
            })
        })
    }

    pub(super) fn mark_tab_stopped_for_session(&mut self, session_name: &str) {
        for tabs in self.workspace_tabs.values_mut() {
            if let Some(tab) = tabs
                .tabs
                .iter_mut()
                .find(|tab| tab.session_name.as_deref() == Some(session_name))
            {
                tab.state = WorkspaceTabRuntimeState::Stopped;
            }
        }
    }

    pub(super) fn selected_active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.selected_workspace_tabs_state_mut()?.active_tab_mut()
    }

    pub(super) fn selected_active_tab_kind(&self) -> PreviewTab {
        self.selected_active_tab()
            .map(|tab| PreviewTab::from(tab.kind))
            .unwrap_or(PreviewTab::Home)
    }

    pub(super) fn sync_preview_tab_from_active_workspace_tab(&mut self) {
        self.preview_tab = self.selected_active_tab_kind();
    }

    pub(super) fn cycle_selected_workspace_tabs(&mut self, direction: i8) {
        let workspace_path = match self.state.selected_workspace() {
            Some(workspace) => workspace.path.clone(),
            None => return,
        };
        let Some(tabs) = self.workspace_tabs.get_mut(workspace_path.as_path()) else {
            return;
        };
        let Some(active_index) = tabs.active_index() else {
            return;
        };
        if tabs.tabs.is_empty() {
            return;
        }
        let next_index = if direction.is_negative() {
            if active_index == 0 {
                tabs.tabs.len().saturating_sub(1)
            } else {
                active_index.saturating_sub(1)
            }
        } else {
            (active_index + 1) % tabs.tabs.len()
        };
        if let Some(next_tab) = tabs.tabs.get(next_index) {
            tabs.active_tab_id = next_tab.id;
        }
        self.sync_preview_tab_from_active_workspace_tab();
    }

    pub(super) fn select_tab_by_id_for_selected_workspace(&mut self, tab_id: u64) -> bool {
        let Some(tabs) = self.selected_workspace_tabs_state_mut() else {
            return false;
        };
        if !tabs.set_active(tab_id) {
            return false;
        }
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
        true
    }

    fn new_session_name_for_tab(
        workspace: &Workspace,
        kind: WorkspaceTabKind,
        ordinal: u64,
    ) -> Option<String> {
        match kind {
            WorkspaceTabKind::Home => None,
            WorkspaceTabKind::Git => Some(git_session_name_for_workspace(workspace)),
            WorkspaceTabKind::Agent => Some(format!(
                "{}-agent-{ordinal}",
                session_name_for_workspace_ref(workspace)
            )),
            WorkspaceTabKind::Shell => Some(format!(
                "{}-shell-{ordinal}",
                session_name_for_workspace_ref(workspace)
            )),
        }
    }

    fn ensure_selected_workspace_tab_kind(
        &mut self,
        kind: WorkspaceTabKind,
    ) -> Option<(PathBuf, u64)> {
        self.sync_workspace_tab_maps();
        let workspace = self.state.selected_workspace()?.clone();
        let workspace_path = workspace.path.clone();
        let selected_tab_id = {
            let tabs = self.workspace_tabs.get_mut(workspace_path.as_path())?;
            if let Some(existing_id) = tabs.find_kind(kind).map(|tab| tab.id) {
                tabs.active_tab_id = existing_id;
                existing_id
            } else {
                let ordinal = tabs.next_tab_ordinal(kind);
                let session_name = Self::new_session_name_for_tab(&workspace, kind, ordinal);
                let title = match kind {
                    WorkspaceTabKind::Agent => {
                        let agent = self
                            .last_agent_selection
                            .get(workspace.path.as_path())
                            .copied()
                            .unwrap_or(workspace.agent);
                        format!("{} {ordinal}", agent.label())
                    }
                    WorkspaceTabKind::Shell => format!("Shell {ordinal}"),
                    WorkspaceTabKind::Git => "Git".to_string(),
                    WorkspaceTabKind::Home => "Home".to_string(),
                };
                tabs.insert_tab_adjacent(WorkspaceTab {
                    id: 0,
                    kind,
                    title,
                    session_name,
                    agent_type: None,
                    state: WorkspaceTabRuntimeState::Stopped,
                })
            }
        };
        self.sync_preview_tab_from_active_workspace_tab();
        Some((workspace_path, selected_tab_id))
    }

    fn tab_kind_marker(kind: WorkspaceTabKind) -> &'static str {
        match kind {
            WorkspaceTabKind::Home => "home",
            WorkspaceTabKind::Agent => "agent",
            WorkspaceTabKind::Shell => "shell",
            WorkspaceTabKind::Git => "git",
        }
    }

    fn parse_tab_kind_marker(raw: &str) -> Option<WorkspaceTabKind> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "agent" => Some(WorkspaceTabKind::Agent),
            "shell" => Some(WorkspaceTabKind::Shell),
            "git" => Some(WorkspaceTabKind::Git),
            "home" => Some(WorkspaceTabKind::Home),
            _ => None,
        }
    }

    fn parse_tmux_tab_metadata_row(row: &str) -> Result<RestoredTmuxTabMetadata, String> {
        let segments = row.split('\t').collect::<Vec<&str>>();
        if segments.len() != 6 {
            return Err("expected six tab metadata fields".to_string());
        }

        let session_name =
            trimmed_nonempty(segments[0]).ok_or_else(|| "missing session name".to_string())?;
        let workspace_path_raw =
            trimmed_nonempty(segments[1]).ok_or_else(|| "missing workspace path".to_string())?;
        let workspace_path = PathBuf::from(workspace_path_raw);
        let kind = Self::parse_tab_kind_marker(segments[2])
            .ok_or_else(|| "invalid tab kind".to_string())?;
        if kind == WorkspaceTabKind::Home {
            return Err("home tab metadata is invalid".to_string());
        }
        let title = trimmed_nonempty(segments[3]).ok_or_else(|| "missing tab title".to_string())?;
        let agent_type = if kind == WorkspaceTabKind::Agent {
            let agent_raw =
                trimmed_nonempty(segments[4]).ok_or_else(|| "missing agent type".to_string())?;
            Some(
                AgentType::from_marker(agent_raw.as_str())
                    .ok_or_else(|| "invalid agent type".to_string())?,
            )
        } else {
            None
        };
        let tab_id = segments[5]
            .trim()
            .parse::<u64>()
            .map_err(|_| "invalid tab id".to_string())?;
        if tab_id == 0 {
            return Err("invalid tab id".to_string());
        }

        Ok(RestoredTmuxTabMetadata {
            session_name,
            workspace_path,
            kind,
            title,
            agent_type,
            tab_id,
        })
    }

    /// Returns true when a tmux list-sessions row has a session name but all
    /// grove metadata columns (workspace path, kind, title, agent, id) are
    /// empty. This covers both non-grove tmux sessions and stale grove
    /// sessions that lost their metadata variables.
    fn has_no_grove_metadata(row: &str) -> bool {
        let segments: Vec<&str> = row.split('\t').collect();
        segments.len() == 6
            && trimmed_nonempty(segments[0]).is_some()
            && segments[1..].iter().all(|s| s.trim().is_empty())
    }

    fn recover_legacy_git_tmux_tab_metadata_row(
        &self,
        row: &str,
    ) -> Option<RestoredTmuxTabMetadata> {
        let segments = row.split('\t').collect::<Vec<&str>>();
        if segments.len() != 6 {
            return None;
        }

        let session_name = trimmed_nonempty(segments[0])?;
        if !segments[1..]
            .iter()
            .all(|segment| segment.trim().is_empty())
        {
            return None;
        }

        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| git_session_name_for_workspace(workspace) == session_name)?;
        let tab_id = self
            .workspace_tabs
            .get(workspace.path.as_path())
            .map(|tabs| {
                tabs.tabs
                    .iter()
                    .map(|tab| tab.id)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
            })
            .unwrap_or(1);

        Some(RestoredTmuxTabMetadata {
            session_name,
            workspace_path: workspace.path.clone(),
            kind: WorkspaceTabKind::Git,
            title: "Git".to_string(),
            agent_type: None,
            tab_id,
        })
    }

    fn write_tab_tmux_metadata(&mut self, workspace_path: &Path, tab: &WorkspaceTab) {
        let Some(session_name) = tab.session_name.as_deref() else {
            return;
        };
        let workspace_path_value = workspace_path.to_string_lossy().to_string();
        let kind_value = Self::tab_kind_marker(tab.kind).to_string();
        let title_value = tab.title.clone();
        let agent_value = tab
            .agent_type
            .map(|agent| agent.marker().to_string())
            .unwrap_or_default();
        let tab_id_value = tab.id.to_string();
        let commands = vec![
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                TMUX_TAB_METADATA_WORKSPACE_PATH_KEY.to_string(),
                workspace_path_value,
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                TMUX_TAB_METADATA_KIND_KEY.to_string(),
                kind_value,
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                TMUX_TAB_METADATA_TITLE_KEY.to_string(),
                title_value,
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                TMUX_TAB_METADATA_AGENT_KEY.to_string(),
                agent_value,
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                TMUX_TAB_METADATA_ID_KEY.to_string(),
                tab_id_value,
            ],
        ];

        for command in commands {
            if let Err(error) = self.execute_tmux_command(command.as_slice()) {
                self.log_event_with_fields(
                    "tab_restore",
                    "metadata_write_failed",
                    [
                        ("session".to_string(), Value::from(session_name.to_string())),
                        ("error".to_string(), Value::from(error.to_string())),
                    ],
                );
                break;
            }
        }
    }

    pub(super) fn rename_workspace_tab_title(
        &mut self,
        workspace_path: &Path,
        tab_id: u64,
        title: String,
    ) -> Result<(), String> {
        let tab = {
            let Some(tabs) = self.workspace_tabs.get_mut(workspace_path) else {
                return Err("workspace tabs unavailable".to_string());
            };
            let Some(tab) = tabs.tab_by_id_mut(tab_id) else {
                return Err("tab not found".to_string());
            };
            tab.title = title;
            tab.clone()
        };
        self.write_tab_tmux_metadata(workspace_path, &tab);
        self.poll_preview();
        Ok(())
    }

    pub(super) fn open_or_focus_git_tab(&mut self) {
        let Some((_, tab_id)) = self.ensure_selected_workspace_tab_kind(WorkspaceTabKind::Git)
        else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let _ = self.select_tab_by_id_for_selected_workspace(tab_id);
        let _ = self.ensure_lazygit_session_for_selected_workspace();
        if let Some(tab) = self.selected_active_tab_mut() {
            tab.state = WorkspaceTabRuntimeState::Running;
        }
        if let Some(workspace_path) = self.selected_workspace_path()
            && let Some(tab) = self.selected_active_tab().cloned()
        {
            self.write_tab_tmux_metadata(workspace_path.as_path(), &tab);
        }
        self.poll_preview();
    }

    fn set_tab_state_by_id(
        &mut self,
        workspace_path: &Path,
        tab_id: u64,
        state: WorkspaceTabRuntimeState,
    ) {
        if let Some(tabs) = self.workspace_tabs.get_mut(workspace_path)
            && let Some(tab) = tabs.tab_by_id_mut(tab_id)
        {
            tab.state = state;
        }
    }

    pub(super) fn open_new_shell_tab(&mut self) {
        self.sync_workspace_tab_maps();
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let Some(tabs) = self.workspace_tabs.get_mut(workspace.path.as_path()) else {
            return;
        };
        let ordinal = tabs.next_tab_ordinal(WorkspaceTabKind::Shell);
        let Some(session_name) =
            Self::new_session_name_for_tab(&workspace, WorkspaceTabKind::Shell, ordinal)
        else {
            return;
        };
        let tab_id = tabs.insert_tab_adjacent(WorkspaceTab {
            id: 0,
            kind: WorkspaceTabKind::Shell,
            title: format!("Shell {ordinal}"),
            session_name: Some(session_name.clone()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Starting,
        });
        self.sync_preview_tab_from_active_workspace_tab();
        self.session
            .shell_sessions
            .mark_in_flight(session_name.clone());
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let workspace_init_command = self.workspace_init_command_for_workspace(&workspace);
        let request = shell_launch_request_for_workspace(
            &workspace,
            session_name.clone(),
            String::new(),
            workspace_init_command,
            Some(capture_cols),
            Some(capture_rows),
        );
        let (_, result) = execute_shell_launch_request_for_mode(
            &request,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        if let Err(error) = &result
            && !tmux_launch_error_indicates_duplicate_session(error)
        {
            self.session
                .shell_sessions
                .mark_failed(session_name.clone());
            self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Failed);
            self.session.last_tmux_error = Some(error.clone());
            self.show_error_toast("shell tab launch failed");
            return;
        }
        self.session.shell_sessions.mark_ready(session_name);
        self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Running);
        if let Some(tab) = self
            .workspace_tabs
            .get(workspace.path.as_path())
            .and_then(|tabs| tabs.tab_by_id(tab_id))
            .cloned()
        {
            self.write_tab_tmux_metadata(workspace.path.as_path(), &tab);
        }
        self.session.last_tmux_error = None;
        self.poll_preview();
    }

    fn agent_env_for_workspace_agent(
        &self,
        workspace: &Workspace,
        agent: AgentType,
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
        let entries = match agent {
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

    pub(super) fn launch_new_agent_tab(
        &mut self,
        agent: AgentType,
        options: StartOptions,
    ) -> Result<(), String> {
        self.sync_workspace_tab_maps();
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            return Err("no workspace selected".to_string());
        };
        self.last_agent_selection
            .insert(workspace.path.clone(), agent);
        self.launch_skip_permissions = options.skip_permissions;
        let _ = write_workspace_skip_permissions(&workspace.path, options.skip_permissions);
        let _ = write_workspace_init_command(&workspace.path, options.init_command.as_deref());

        let Some(tabs) = self.workspace_tabs.get_mut(workspace.path.as_path()) else {
            return Err("workspace tabs unavailable".to_string());
        };
        let ordinal = tabs.next_tab_ordinal(WorkspaceTabKind::Agent);
        let Some(session_name) =
            Self::new_session_name_for_tab(&workspace, WorkspaceTabKind::Agent, ordinal)
        else {
            return Err("failed to build agent session name".to_string());
        };
        let tab_title = options
            .name
            .clone()
            .unwrap_or_else(|| format!("{} {ordinal}", agent.label()));
        let tab_id = tabs.insert_tab_adjacent(WorkspaceTab {
            id: 0,
            kind: WorkspaceTabKind::Agent,
            title: tab_title,
            session_name: Some(session_name.clone()),
            agent_type: Some(agent),
            state: WorkspaceTabRuntimeState::Starting,
        });
        self.sync_preview_tab_from_active_workspace_tab();

        let agent_env = self.agent_env_for_workspace_agent(&workspace, agent)?;
        let (capture_cols, capture_rows) = self.capture_dimensions();
        let mut launch_workspace = workspace.clone();
        launch_workspace.name = format!("{}-agent-{ordinal}", workspace.name);
        launch_workspace.agent = agent;
        let mut request = launch_request_for_workspace(
            &launch_workspace,
            options.prompt,
            options
                .init_command
                .or_else(|| self.workspace_init_command_for_workspace(&workspace)),
            options.skip_permissions,
            agent_env,
            Some(capture_cols),
            Some(capture_rows),
        );
        request.session_name = Some(session_name.clone());
        self.session
            .agent_sessions
            .mark_in_flight(session_name.clone());
        let completion = execute_launch_request_with_result_for_mode(
            &request,
            CommandExecutionMode::Delegating(&mut |command| self.execute_tmux_command(command)),
        );
        if let Err(error) = completion.result {
            self.session.agent_sessions.mark_failed(session_name);
            self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Failed);
            self.session.last_tmux_error = Some(error.clone());
            return Err(error);
        }
        self.session.agent_sessions.mark_ready(session_name);
        self.set_tab_state_by_id(&workspace.path, tab_id, WorkspaceTabRuntimeState::Running);
        if let Some(tab) = self
            .workspace_tabs
            .get(workspace.path.as_path())
            .and_then(|tabs| tabs.tab_by_id(tab_id))
            .cloned()
        {
            self.write_tab_tmux_metadata(workspace.path.as_path(), &tab);
        }
        self.session.last_tmux_error = None;
        self.poll_preview();
        Ok(())
    }

    fn session_exists(&self, session_name: &str) -> bool {
        let command = vec![
            "tmux".to_string(),
            "has-session".to_string(),
            "-t".to_string(),
            session_name.to_string(),
        ];
        self.tmux_input.execute(&command).is_ok()
    }

    pub(super) fn active_tab_session_name(&self) -> Option<String> {
        self.selected_active_tab()?.session_name.clone()
    }

    pub(super) fn selected_shell_tab_session_name(&self) -> Option<String> {
        let tab = self.selected_active_tab()?;
        if tab.kind != WorkspaceTabKind::Shell {
            return None;
        }
        tab.session_name.clone()
    }

    pub(super) fn kill_active_tab_session(&mut self) {
        let Some(session_name) = self.active_tab_session_name() else {
            self.show_info_toast("home tab has no live session");
            return;
        };
        let command = vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            session_name.clone(),
        ];
        if let Err(error) = self.execute_tmux_command(&command) {
            let message = error.to_string();
            self.session.last_tmux_error = Some(message.clone());
            self.show_error_toast(format!("kill failed: {message}"));
            return;
        }
        self.session.agent_sessions.remove_ready(&session_name);
        self.session.shell_sessions.remove_ready(&session_name);
        self.session.lazygit_sessions.remove_ready(&session_name);
        if let Some(tab) = self.selected_active_tab_mut() {
            tab.state = WorkspaceTabRuntimeState::Stopped;
        }
        self.session.last_tmux_error = None;
        self.poll_preview();
    }

    pub(super) fn close_active_tab_or_confirm(&mut self) {
        let Some(tab) = self.selected_active_tab().cloned() else {
            return;
        };
        if tab.kind == WorkspaceTabKind::Home {
            self.show_info_toast("home tab cannot be closed");
            return;
        }
        if let Some(session_name) = tab.session_name.as_deref()
            && self.session_exists(session_name)
        {
            let Some(workspace) = self.state.selected_workspace() else {
                return;
            };
            self.set_confirm_dialog(ConfirmDialogState {
                action: ConfirmDialogAction::CloseActiveTab {
                    workspace_path: workspace.path.clone(),
                    tab_id: tab.id,
                    session_name: session_name.to_string(),
                },
                focused_field: ConfirmDialogField::CancelButton,
            });
            return;
        }
        self.close_tab_for_selected_workspace(tab.id);
    }

    pub(super) fn close_tab_for_selected_workspace(&mut self, tab_id: u64) {
        let Some(tabs) = self.selected_workspace_tabs_state_mut() else {
            return;
        };
        let _ = tabs.close_tab(tab_id);
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
    }

    pub(super) fn force_close_active_tab_and_session(
        &mut self,
        workspace_path: &Path,
        tab_id: u64,
        session_name: &str,
    ) {
        let command = vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            session_name.to_string(),
        ];
        let _ = self.execute_tmux_command(&command);
        self.session.agent_sessions.remove_ready(session_name);
        self.session.shell_sessions.remove_ready(session_name);
        self.session.lazygit_sessions.remove_ready(session_name);
        if let Some(tabs) = self.workspace_tabs.get_mut(workspace_path) {
            let _ = tabs.close_tab(tab_id);
        }
        self.sync_preview_tab_from_active_workspace_tab();
        self.poll_preview();
    }

    pub(super) fn active_tab_is_scrollable(&self) -> bool {
        match self.selected_active_tab_kind() {
            PreviewTab::Home => self.selected_task_preview_session_if_ready().is_some(),
            PreviewTab::Agent | PreviewTab::Shell => true,
            PreviewTab::Git => false,
        }
    }
}
