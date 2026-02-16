use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::infrastructure::adapters::{
    BootstrapData, CommandGitAdapter, CommandMultiplexerAdapter, CommandSystemAdapter,
    DiscoveryState, MultiplexerAdapter, bootstrap_data,
};
use crate::infrastructure::config::{GroveConfig, MultiplexerKind, ProjectConfig};
use crate::ui::mouse::parse_sidebar_ratio;

use super::*;

#[derive(Debug)]
pub(super) struct AppPaths {
    sidebar_ratio_path: PathBuf,
    config_path: PathBuf,
}

impl AppPaths {
    pub(super) fn new(sidebar_ratio_path: PathBuf, config_path: PathBuf) -> Self {
        Self {
            sidebar_ratio_path,
            config_path,
        }
    }
}

pub(super) struct AppDependencies {
    pub(super) tmux_input: Box<dyn TmuxInput>,
    pub(super) clipboard: Box<dyn ClipboardAccess>,
    pub(super) paths: AppPaths,
    pub(super) multiplexer: MultiplexerKind,
    pub(super) event_log: Box<dyn EventLogger>,
    pub(super) debug_record_start_ts: Option<u64>,
}

pub(super) fn default_sidebar_ratio_path() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(SIDEBAR_RATIO_FILENAME),
        Err(_) => PathBuf::from(SIDEBAR_RATIO_FILENAME),
    }
}

pub(super) fn load_sidebar_ratio(path: &Path) -> u16 {
    let Ok(raw) = fs::read_to_string(path) else {
        return DEFAULT_SIDEBAR_WIDTH_PCT;
    };

    parse_sidebar_ratio(&raw).unwrap_or(DEFAULT_SIDEBAR_WIDTH_PCT)
}

fn default_config_path() -> PathBuf {
    crate::infrastructure::config::config_path()
        .unwrap_or_else(|| PathBuf::from(".config/grove/config.toml"))
}

fn current_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?;
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.canonicalize().ok().or(Some(path))
}

pub(super) fn project_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn project_paths_equal(left: &Path, right: &Path) -> bool {
    let left_canonical = left.canonicalize().ok();
    let right_canonical = right.canonicalize().ok();
    match (left_canonical, right_canonical) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

fn ensure_current_repo_project(config: &mut GroveConfig, config_path: &Path) -> Option<String> {
    let repo_root = current_repo_root()?;

    let already_present = config
        .projects
        .iter()
        .any(|project| project_paths_equal(&project.path, &repo_root));
    if already_present {
        return None;
    }

    config.projects.push(ProjectConfig {
        name: project_display_name(&repo_root),
        path: repo_root,
    });
    crate::infrastructure::config::save_to_path(config_path, config).err()
}

pub(super) fn load_runtime_config() -> (GroveConfig, PathBuf, Option<String>) {
    let (mut config, config_path, load_error) = match crate::infrastructure::config::load() {
        Ok(loaded) => (loaded.config, loaded.path, None),
        Err(error) => (GroveConfig::default(), default_config_path(), Some(error)),
    };
    let startup_error = ensure_current_repo_project(&mut config, &config_path);
    let error = match (load_error, startup_error) {
        (Some(load_error), Some(startup_error)) => Some(format!(
            "{load_error}; startup project add failed: {startup_error}"
        )),
        (Some(load_error), None) => Some(load_error),
        (None, Some(startup_error)) => Some(format!("startup project add failed: {startup_error}")),
        (None, None) => None,
    };

    (config, config_path, error)
}

#[derive(Debug, Clone)]
struct StaticMultiplexerAdapter {
    running_sessions: HashSet<String>,
}

impl MultiplexerAdapter for StaticMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        self.running_sessions.clone()
    }
}

pub(super) fn bootstrap_data_for_projects(
    projects: &[ProjectConfig],
    multiplexer: MultiplexerKind,
) -> BootstrapData {
    if projects.is_empty() {
        return BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Empty,
            orphaned_sessions: Vec::new(),
        };
    }

    let live_multiplexer = CommandMultiplexerAdapter { multiplexer };
    let static_multiplexer = StaticMultiplexerAdapter {
        running_sessions: live_multiplexer.running_sessions(),
    };
    let mut workspaces = Vec::new();
    let mut orphaned_sessions = Vec::new();
    let mut errors = Vec::new();
    for project in projects {
        let git = CommandGitAdapter::for_repo(project.path.clone());
        let system = CommandSystemAdapter::for_repo(project.path.clone());
        let bootstrap = bootstrap_data(&git, &static_multiplexer, &system);
        if let DiscoveryState::Error(message) = &bootstrap.discovery_state {
            errors.push(format!("{}: {message}", project.name));
        }

        workspaces.extend(bootstrap.workspaces);
        orphaned_sessions.extend(bootstrap.orphaned_sessions);
    }

    let discovery_state = if !workspaces.is_empty() {
        DiscoveryState::Ready
    } else if !errors.is_empty() {
        DiscoveryState::Error(errors.join("; "))
    } else {
        DiscoveryState::Empty
    };
    let repo_name = if projects.len() == 1 {
        projects[0].name.clone()
    } else {
        format!("{} projects", projects.len())
    };

    BootstrapData {
        repo_name,
        workspaces,
        discovery_state,
        orphaned_sessions,
    }
}

pub(super) fn input_for_multiplexer(multiplexer: MultiplexerKind) -> Box<dyn TmuxInput> {
    let _ = multiplexer;
    Box::new(CommandTmuxInput)
}

pub(super) fn read_workspace_launch_prompt(workspace_path: &Path) -> Option<String> {
    let raw = fs::read_to_string(workspace_path.join(WORKSPACE_LAUNCH_PROMPT_FILENAME)).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

pub(super) fn load_local_branches(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|error| format!("git branch failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git branch failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("git branch output decode failed: {error}"))?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

pub(super) fn filter_branches(query: &str, all_branches: &[String]) -> Vec<String> {
    if query.is_empty() {
        return all_branches.to_vec();
    }

    let query_lower = query.to_lowercase();
    all_branches
        .iter()
        .filter(|branch| branch.to_lowercase().contains(&query_lower))
        .cloned()
        .collect()
}

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
            delete_in_flight: false,
            merge_in_flight: false,
            update_from_base_in_flight: false,
            create_in_flight: false,
            start_in_flight: false,
            stop_in_flight: false,
            deferred_cmds: Vec::new(),
        };
        app.refresh_preview_summary();
        app
    }
}
