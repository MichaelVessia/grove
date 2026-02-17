use crate::infrastructure::adapters::BootstrapData;
use crate::infrastructure::config::{MultiplexerKind, ProjectConfig};

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
            project_delete_in_flight: false,
            delete_in_flight: false,
            delete_in_flight_workspace: None,
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
