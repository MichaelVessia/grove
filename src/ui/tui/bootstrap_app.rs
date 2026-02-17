use crate::infrastructure::adapters::BootstrapData;
use crate::infrastructure::config::{MultiplexerKind, ProjectConfig};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[cfg(test)]
use super::bootstrap_config::project_paths_equal;
use super::bootstrap_config::{
    AppDependencies, AppPaths, default_sidebar_ratio_path, input_for_multiplexer,
    load_runtime_config, load_sidebar_ratio,
};
use super::bootstrap_discovery::bootstrap_data_for_projects;
use super::*;

impl GroveApp {
    pub(super) fn new_with_event_logger(event_log: Box<dyn EventLogger>) -> Self {
        let (config, config_path, _config_error) = load_runtime_config();
        let multiplexer = config.multiplexer;
        let bootstrap = bootstrap_data_for_projects(&config.projects, multiplexer);
        Self::from_parts_with_projects(
            bootstrap,
            config.projects,
            input_for_multiplexer(multiplexer),
            AppPaths::new(default_sidebar_ratio_path(), config_path),
            multiplexer,
            event_log,
            None,
        )
    }

    pub(super) fn new_with_debug_recorder(
        event_log: Box<dyn EventLogger>,
        app_start_ts: u64,
    ) -> Self {
        let (config, config_path, _config_error) = load_runtime_config();
        let multiplexer = config.multiplexer;
        let bootstrap = bootstrap_data_for_projects(&config.projects, multiplexer);
        Self::from_parts_with_projects(
            bootstrap,
            config.projects,
            input_for_multiplexer(multiplexer),
            AppPaths::new(default_sidebar_ratio_path(), config_path),
            multiplexer,
            event_log,
            Some(app_start_ts),
        )
    }

    #[cfg(test)]
    fn projects_from_bootstrap(bootstrap: &BootstrapData) -> Vec<ProjectConfig> {
        let mut projects = Vec::new();
        for workspace in &bootstrap.workspaces {
            let Some(project_path) = workspace.project_path.as_ref() else {
                continue;
            };
            let project_name = workspace.project_name.clone().unwrap_or_else(|| {
                project_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map_or_else(|| project_path.display().to_string(), ToString::to_string)
            });

            if projects.iter().any(|project: &ProjectConfig| {
                project.name == project_name || project_paths_equal(&project.path, project_path)
            }) {
                continue;
            }
            projects.push(ProjectConfig {
                name: project_name,
                path: project_path.clone(),
                defaults: Default::default(),
            });
        }
        projects
    }

    #[cfg(test)]
    pub(super) fn from_parts(
        bootstrap: BootstrapData,
        tmux_input: Box<dyn TmuxInput>,
        paths: AppPaths,
        multiplexer: MultiplexerKind,
        event_log: Box<dyn EventLogger>,
        debug_record_start_ts: Option<u64>,
    ) -> Self {
        let projects = Self::projects_from_bootstrap(&bootstrap);
        Self::from_parts_with_projects(
            bootstrap,
            projects,
            tmux_input,
            paths,
            multiplexer,
            event_log,
            debug_record_start_ts,
        )
    }

    fn from_parts_with_projects(
        bootstrap: BootstrapData,
        projects: Vec<ProjectConfig>,
        tmux_input: Box<dyn TmuxInput>,
        paths: AppPaths,
        multiplexer: MultiplexerKind,
        event_log: Box<dyn EventLogger>,
        debug_record_start_ts: Option<u64>,
    ) -> Self {
        Self::from_parts_with_clipboard_and_projects(
            bootstrap,
            projects,
            AppDependencies {
                tmux_input,
                clipboard: Box::new(SystemClipboardAccess::default()),
                paths,
                multiplexer,
                event_log,
                debug_record_start_ts,
            },
        )
    }

    pub(super) fn from_parts_with_clipboard_and_projects(
        bootstrap: BootstrapData,
        projects: Vec<ProjectConfig>,
        dependencies: AppDependencies,
    ) -> Self {
        let AppDependencies {
            tmux_input,
            clipboard,
            paths,
            multiplexer,
            event_log,
            debug_record_start_ts,
        } = dependencies;
        let AppPaths {
            sidebar_ratio_path,
            config_path,
        } = paths;
        let sidebar_width_pct = load_sidebar_ratio(&sidebar_ratio_path);
        let mapper_config = KeybindingConfig::from_env().with_sequence_config(
            KeySequenceConfig::from_env()
                .disable_sequences()
                .validated(),
        );
        let mut app = Self {
            repo_name: bootstrap.repo_name,
            projects,
            state: AppState::new(bootstrap.workspaces),
            discovery_state: bootstrap.discovery_state,
            preview_tab: PreviewTab::Agent,
            preview: PreviewState::new(),
            notifications: NotificationQueue::new(
                QueueConfig::new()
                    .max_visible(3)
                    .max_queued(24)
                    .position(ToastPosition::TopRight)
                    .default_duration(Duration::from_secs(3))
                    .dedup_window_ms(0),
            ),
            interactive: None,
            action_mapper: ActionMapper::new(mapper_config),
            launch_dialog: None,
            delete_dialog: None,
            merge_dialog: None,
            update_from_base_dialog: None,
            create_dialog: None,
            edit_dialog: None,
            project_dialog: None,
            settings_dialog: None,
            keybind_help_open: false,
            command_palette: CommandPalette::new().with_max_visible(12),
            create_branch_all: Vec::new(),
            create_branch_filtered: Vec::new(),
            create_branch_index: 0,
            multiplexer,
            tmux_input,
            config_path,
            clipboard,
            last_tmux_error: None,
            output_changing: false,
            agent_output_changing: false,
            agent_activity_frames: VecDeque::with_capacity(AGENT_ACTIVITY_WINDOW_FRAMES),
            workspace_status_digests: HashMap::new(),
            workspace_output_changing: HashMap::new(),
            lazygit_ready_sessions: HashSet::new(),
            lazygit_failed_sessions: HashSet::new(),
            lazygit_launch_in_flight: HashSet::new(),
            shell_ready_sessions: HashSet::new(),
            shell_failed_sessions: HashSet::new(),
            shell_launch_in_flight: HashSet::new(),
            lazygit_command: resolve_lazygit_command(),
            viewport_width: 120,
            viewport_height: 40,
            sidebar_width_pct,
            sidebar_hidden: false,
            launch_skip_permissions: false,
            sidebar_ratio_path,
            divider_drag_active: false,
            preview_selection: TextSelectionState::default(),
            copied_text: None,
            event_log,
            last_hit_grid: RefCell::new(None),
            next_tick_due_at: None,
            next_tick_interval_ms: None,
            next_poll_due_at: None,
            preview_poll_in_flight: false,
            preview_poll_requested: false,
            next_visual_due_at: None,
            interactive_poll_due_at: None,
            fast_animation_frame: 0,
            poll_generation: 0,
            debug_record_start_ts,
            frame_render_seq: RefCell::new(0),
            input_seq_counter: 1,
            pending_interactive_inputs: VecDeque::new(),
            pending_interactive_sends: VecDeque::new(),
            interactive_send_in_flight: false,
            pending_resize_verification: None,
            refresh_in_flight: false,
            project_delete_in_flight: false,
            delete_in_flight: false,
            delete_in_flight_workspace: None,
            pending_delete_workspaces: VecDeque::new(),
            delete_requested_workspaces: HashSet::new(),
            merge_in_flight: false,
            update_from_base_in_flight: false,
            create_in_flight: false,
            pending_auto_start_workspace_path: None,
            start_in_flight: false,
            stop_in_flight: false,
            deferred_cmds: Vec::new(),
        };
        app.refresh_preview_summary();
        app
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
    if let Some(override_command) = normalized_command_override(override_value) {
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

fn normalized_command_override(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
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
