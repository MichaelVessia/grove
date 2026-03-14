use crate::application::task_discovery::{TaskBootstrapData, TaskDiscoveryState};
use crate::infrastructure::config::ProjectConfig;
use crate::infrastructure::paths::tasks_root;
use std::collections::HashMap;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::bootstrap_config::{AppDependencies, load_runtime_config};
use super::bootstrap_discovery::bootstrap_task_data_for_root;
use super::*;
use crate::ui::mouse::clamp_sidebar_ratio;

impl GroveApp {
    fn default_workspace_tabs_and_last_agent(
        workspaces: &[crate::domain::Workspace],
    ) -> (
        HashMap<PathBuf, WorkspaceTabsState>,
        HashMap<PathBuf, AgentType>,
    ) {
        let workspace_tabs = workspaces
            .iter()
            .map(|workspace| (workspace.path.clone(), WorkspaceTabsState::new()))
            .collect::<HashMap<PathBuf, WorkspaceTabsState>>();
        let last_agent_selection = workspaces
            .iter()
            .map(|workspace| (workspace.path.clone(), workspace.agent))
            .collect::<HashMap<PathBuf, AgentType>>();
        (workspace_tabs, last_agent_selection)
    }

    pub(super) fn new(event_log: Box<dyn EventLogger>, debug_record_start_ts: Option<u64>) -> Self {
        let (config, config_path, _config_error) = load_runtime_config();
        let dependencies = AppDependencies {
            tmux_input: Box::new(CommandTmuxInput),
            clipboard: Box::new(SystemClipboardAccess::default()),
            config_path,
            event_log,
            debug_record_start_ts,
        };

        let bootstrap = tasks_root()
            .map(|tasks_root| bootstrap_task_data_for_root(tasks_root.as_path(), &config.projects))
            .unwrap_or(TaskBootstrapData {
                tasks: Vec::new(),
                discovery_state: TaskDiscoveryState::Empty,
            });
        Self::from_task_parts_with_clipboard_and_projects(bootstrap, config.projects, dependencies)
    }

    pub(super) fn from_task_parts_with_clipboard_and_projects(
        bootstrap: TaskBootstrapData,
        projects: Vec<ProjectConfig>,
        dependencies: AppDependencies,
    ) -> Self {
        let repo_name = if bootstrap.tasks.is_empty() {
            "tasks".to_string()
        } else {
            format!("{} tasks", bootstrap.tasks.len())
        };
        let discovery_state = match bootstrap.discovery_state {
            TaskDiscoveryState::Ready => DiscoveryState::Ready,
            TaskDiscoveryState::Empty => DiscoveryState::Empty,
            TaskDiscoveryState::Error(message) => DiscoveryState::Error(message),
        };
        Self::from_state_with_clipboard_and_projects(
            repo_name,
            AppState::new(bootstrap.tasks),
            discovery_state,
            projects,
            dependencies,
        )
    }

    fn from_state_with_clipboard_and_projects(
        repo_name: String,
        state: AppState,
        discovery_state: DiscoveryState,
        projects: Vec<ProjectConfig>,
        dependencies: AppDependencies,
    ) -> Self {
        let AppDependencies {
            tmux_input,
            clipboard,
            config_path,
            event_log,
            debug_record_start_ts,
        } = dependencies;
        let persisted_config = crate::infrastructure::config::load_from_path(&config_path)
            .unwrap_or_else(|_| GroveConfig::default());
        let sidebar_width_pct = clamp_sidebar_ratio(persisted_config.sidebar_width_pct);
        let theme_name = persisted_config.theme;
        let launch_skip_permissions = persisted_config.launch_skip_permissions;
        let task_order = persisted_config.task_order;
        let workspace_attention_ack_markers = persisted_config
            .attention_acks
            .into_iter()
            .filter_map(|entry| {
                let marker = trimmed_nonempty(&entry.marker)?;
                Some((entry.workspace_path, marker))
            })
            .collect::<HashMap<PathBuf, String>>();
        let mapper_config = KeybindingConfig::from_env().with_sequence_config(
            KeySequenceConfig::from_env()
                .disable_sequences()
                .validated(),
        );
        let (workspace_tabs, last_agent_selection) =
            Self::default_workspace_tabs_and_last_agent(&state.workspaces);
        let mut app = Self {
            repo_name,
            projects,
            task_order,
            task_reorder: None,
            state,
            discovery_state,
            preview_tab: PreviewTab::Agent,
            workspace_tabs,
            last_agent_selection,
            preview: PreviewState::new(),
            notifications: NotificationQueue::new(
                QueueConfig::new()
                    .max_visible(3)
                    .max_queued(24)
                    .position(ToastPosition::TopRight)
                    .default_duration(Duration::from_secs(3))
                    .dedup_window_ms(0),
            ),
            action_mapper: ActionMapper::new(mapper_config),
            dialogs: DialogState {
                active_dialog: None,
                keybind_help_open: false,
                command_palette: CommandPalette::new().with_max_visible(12),
                refresh_in_flight: false,
                last_manual_refresh_requested_at: None,
                manual_refresh_feedback_pending: false,
                project_delete_in_flight: false,
                delete_in_flight: false,
                delete_in_flight_workspace: None,
                pending_delete_workspaces: VecDeque::new(),
                delete_requested_workspaces: HashSet::new(),
                merge_in_flight: false,
                update_from_base_in_flight: false,
                pull_upstream_in_flight: false,
                create_in_flight: false,
                start_in_flight: false,
                stop_in_flight: false,
                restart_in_flight: false,
            },
            tmux_input,
            config_path,
            clipboard,
            session: SessionState {
                interactive: None,
                last_tmux_error: None,
                agent_sessions: SessionTracker::default(),
                lazygit_sessions: SessionTracker::default(),
                shell_sessions: SessionTracker::default(),
                lazygit_command: resolve_lazygit_command(),
                pending_interactive_inputs: VecDeque::new(),
                pending_interactive_sends: VecDeque::new(),
                interactive_send_in_flight: false,
                pending_resize_verification: None,
                pending_restart_workspace_path: None,
            },
            polling: PollingState {
                output_changing: false,
                agent_output_changing: false,
                agent_working_until: None,
                agent_idle_polls_since_output: 0,
                workspace_status_digests: HashMap::new(),
                workspace_output_changing: HashMap::new(),
                next_tick_due_at: None,
                next_tick_interval_ms: None,
                next_poll_due_at: None,
                last_workspace_status_poll_at: None,
                preview_poll_in_flight: false,
                preview_poll_requested: false,
                next_visual_due_at: None,
                interactive_poll_due_at: None,
                activity_animation: AnimationClock::new(),
                poll_generation: 0,
            },
            workspace_attention: HashMap::new(),
            workspace_attention_ack_markers,
            viewport_width: 120,
            viewport_height: 40,
            sidebar_width_pct,
            panes: panes::GrovePaneModel::canonical(sidebar_width_pct),
            theme_name,
            sidebar_hidden: false,
            mouse_capture_enabled: true,
            launch_skip_permissions,
            divider_resize: ftui::layout::pane::PaneDragResizeMachine::new_with_hysteresis(1, 1)
                .expect("valid divider resize machine"),
            divider_resize_anchor_x: 0,
            divider_resize_event_seq: 1,
            preview_selection: TextSelectionState::default(),
            copied_text: None,
            telemetry: TelemetryState {
                event_log,
                debug_record_start_ts,
                replay_msg_seq_counter: 0,
                frame_render_seq: RefCell::new(0),
                last_frame_hash: RefCell::new(0),
                input_seq_counter: 1,
                deferred_cmds: Vec::new(),
            },
            last_hit_grid: RefCell::new(None),
            preview_scroll: RefCell::new(Virtualized::external(0, 0).with_follow(true)),
            sidebar_list_state: RefCell::new(VirtualizedListState::new().with_overscan(0)),
            last_sidebar_mouse_scroll_at: None,
            last_sidebar_mouse_scroll_delta: 0,
            #[cfg(test)]
            task_root_override: None,
            #[cfg(test)]
            pull_request_branch_name_override: None,
        };
        app.reconcile_task_order();
        app.reorder_tasks_for_task_order();
        app.sync_workspace_tab_maps();
        app.rebuild_workspace_tabs_from_tmux_metadata();
        app.reconcile_workspace_attention_tracking();
        app.refresh_preview_summary();
        app
    }

    pub(super) fn from_task_state(
        repo_name: String,
        state: AppState,
        discovery_state: DiscoveryState,
        projects: Vec<ProjectConfig>,
        dependencies: AppDependencies,
    ) -> Self {
        Self::from_state_with_clipboard_and_projects(
            repo_name,
            state,
            discovery_state,
            projects,
            dependencies,
        )
    }
}

fn resolve_lazygit_command() -> String {
    resolve_lazygit_command_with(
        env::var("GROVE_LAZYGIT_CMD").ok(),
        resolve_executable_from_path(LAZYGIT_COMMAND),
        resolve_executable_from_login_shell(LAZYGIT_COMMAND),
        resolve_executable_from_standard_locations(LAZYGIT_COMMAND),
    )
}

fn resolve_lazygit_command_with(
    override_value: Option<String>,
    lazygit_path: Option<PathBuf>,
    login_shell_path: Option<PathBuf>,
    standard_path: Option<PathBuf>,
) -> String {
    if let Some(override_command) = override_value.as_deref().and_then(trimmed_nonempty) {
        return override_command;
    }

    if let Some(path) = lazygit_path {
        return path.to_string_lossy().to_string();
    }

    if let Some(path) = login_shell_path {
        return path.to_string_lossy().to_string();
    }

    if let Some(path) = standard_path {
        return path.to_string_lossy().to_string();
    }

    LAZYGIT_COMMAND.to_string()
}

fn resolve_executable_from_path(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(command);
        if candidate_is_executable(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn resolve_executable_from_login_shell(command: &str) -> Option<PathBuf> {
    let shell = env::var("SHELL").ok()?;
    let lookup = format!("command -v {command}");
    for flag in ["-lic", "-lc"] {
        let output = match std::process::Command::new(&shell)
            .args([flag, lookup.as_str()])
            .output()
        {
            Ok(output) => output,
            Err(_) => continue,
        };
        if !output.status.success() {
            continue;
        }
        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(_) => continue,
        };
        if let Some(path) = resolve_executable_from_shell_lookup_output(command, &stdout) {
            return Some(path);
        }
    }

    None
}

fn resolve_executable_from_shell_lookup_output(command: &str, output: &str) -> Option<PathBuf> {
    for line in output.lines() {
        let Some(path) = parse_shell_lookup_line_for_command_path(command, line) else {
            continue;
        };
        if candidate_is_executable(&path) {
            return Some(path);
        }
    }

    None
}

fn parse_shell_lookup_line_for_command_path(command: &str, line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed == command {
        return None;
    }

    if let Some(path) = parse_shell_lookup_token_to_command_path(command, trimmed) {
        return Some(path);
    }

    if let Some(rest) = trimmed
        .strip_prefix(command)
        .and_then(|suffix| suffix.strip_prefix(" is "))
        && let Some(path) = parse_shell_lookup_token_to_command_path(command, rest)
    {
        return Some(path);
    }

    for token in trimmed.split_whitespace() {
        if let Some(path) = parse_shell_lookup_token_to_command_path(command, token) {
            return Some(path);
        }
    }

    None
}

fn parse_shell_lookup_token_to_command_path(command: &str, token: &str) -> Option<PathBuf> {
    let token = token.trim_matches(|character| {
        matches!(
            character,
            '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'
        )
    });
    if token.is_empty() || !token.contains('/') || token.chars().any(char::is_whitespace) {
        return None;
    }

    let candidate = expand_home_path(token)?;
    if candidate.file_name().and_then(|name| name.to_str()) != Some(command) {
        return None;
    }

    Some(candidate)
}

fn expand_home_path(path: &str) -> Option<PathBuf> {
    if path == "~" {
        return env::var_os("HOME").map(PathBuf::from);
    }
    if let Some(suffix) = path.strip_prefix("~/") {
        return env::var_os("HOME").map(|home| PathBuf::from(home).join(suffix));
    }

    Some(PathBuf::from(path))
}

fn resolve_executable_from_standard_locations(command: &str) -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin").join(command),
        PathBuf::from("/usr/local/bin").join(command),
        PathBuf::from("/usr/bin").join(command),
    ];
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("bin").join(command));
        candidates.push(home.join(".local/bin").join(command));
        candidates.push(home.join(".cargo/bin").join(command));
    }

    candidates
        .into_iter()
        .find(|path| candidate_is_executable(path))
}

fn candidate_is_executable(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        let metadata = match fs::metadata(candidate) {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_lazygit_command_prefers_override() {
        let resolved = resolve_lazygit_command_with(
            Some("custom-lazygit".to_string()),
            Some(PathBuf::from("/bin/lazygit")),
            Some(PathBuf::from("/usr/local/bin/lazygit")),
            Some(PathBuf::from("/opt/homebrew/bin/lazygit")),
        );
        assert_eq!(resolved, "custom-lazygit");
    }

    #[test]
    fn resolve_lazygit_command_prefers_path_when_available() {
        let resolved = resolve_lazygit_command_with(
            None,
            Some(PathBuf::from("/bin/lazygit")),
            Some(PathBuf::from("/usr/local/bin/lazygit")),
            Some(PathBuf::from("/opt/homebrew/bin/lazygit")),
        );
        assert_eq!(resolved, "/bin/lazygit");
    }

    #[test]
    fn resolve_lazygit_command_uses_login_shell_path() {
        let resolved = resolve_lazygit_command_with(
            None,
            None,
            Some(PathBuf::from("/usr/local/bin/lazygit")),
            Some(PathBuf::from("/opt/homebrew/bin/lazygit")),
        );
        assert_eq!(resolved, "/usr/local/bin/lazygit");
    }

    #[test]
    fn resolve_lazygit_command_uses_standard_location() {
        let resolved = resolve_lazygit_command_with(
            None,
            None,
            None,
            Some(PathBuf::from("/opt/homebrew/bin/lazygit")),
        );
        assert_eq!(resolved, "/opt/homebrew/bin/lazygit");
    }

    #[test]
    fn resolve_lazygit_command_falls_back_to_plain_command() {
        let resolved = resolve_lazygit_command_with(None, None, None, None);
        assert_eq!(resolved, LAZYGIT_COMMAND);
    }

    #[test]
    fn parse_shell_lookup_line_extracts_direct_path() {
        let resolved = parse_shell_lookup_line_for_command_path("lazygit", "/tmp/lazygit");
        assert_eq!(resolved, Some(PathBuf::from("/tmp/lazygit")));
    }

    #[test]
    fn parse_shell_lookup_line_extracts_built_in_message_path() {
        let resolved = parse_shell_lookup_line_for_command_path(
            "lazygit",
            "lazygit is /opt/homebrew/bin/lazygit",
        );
        assert_eq!(resolved, Some(PathBuf::from("/opt/homebrew/bin/lazygit")));
    }

    #[test]
    fn parse_shell_lookup_line_ignores_noise_path_with_different_binary_name() {
        let resolved =
            parse_shell_lookup_line_for_command_path("lazygit", "direnv: loading /tmp/.envrc");
        assert_eq!(resolved, None);
    }
}
