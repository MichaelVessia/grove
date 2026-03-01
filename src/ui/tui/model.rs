#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedDeleteWorkspace {
    request: DeleteWorkspaceRequest,
    workspace_name: String,
    workspace_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingAutoStartWorkspace {
    workspace_path: PathBuf,
    start_config: StartAgentConfigState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionKind {
    Lazygit,
    WorkspaceShell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceAttention {
    NeedsAttention,
}

#[derive(Debug, Default)]
struct SessionTracker {
    ready: HashSet<String>,
    failed: HashSet<String>,
    in_flight: HashSet<String>,
}

impl SessionTracker {
    fn is_ready(&self, session_name: &str) -> bool {
        self.ready.contains(session_name)
    }

    fn is_failed(&self, session_name: &str) -> bool {
        self.failed.contains(session_name)
    }

    fn is_in_flight(&self, session_name: &str) -> bool {
        self.in_flight.contains(session_name)
    }

    fn retry_failed(&mut self, session_name: &str) {
        self.failed.remove(session_name);
    }

    fn mark_in_flight(&mut self, session_name: String) {
        self.in_flight.insert(session_name);
    }

    fn mark_ready(&mut self, session_name: String) {
        self.in_flight.remove(&session_name);
        self.failed.remove(&session_name);
        self.ready.insert(session_name);
    }

    fn mark_failed(&mut self, session_name: String) {
        self.in_flight.remove(&session_name);
        self.ready.remove(&session_name);
        self.failed.insert(session_name);
    }

    fn remove_ready(&mut self, session_name: &str) {
        self.ready.remove(session_name);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActiveDialog {
    Launch(LaunchDialogState),
    Stop(StopDialogState),
    Confirm(ConfirmDialogState),
    Delete(DeleteDialogState),
    Merge(MergeDialogState),
    UpdateFromBase(UpdateFromBaseDialogState),
    Create(CreateDialogState),
    Edit(EditDialogState),
    Project(ProjectDialogState),
    Settings(SettingsDialogState),
}

struct GroveApp {
    repo_name: String,
    projects: Vec<ProjectConfig>,
    state: AppState,
    discovery_state: DiscoveryState,
    preview_tab: PreviewTab,
    preview: PreviewState,
    notifications: NotificationQueue,
    interactive: Option<InteractiveState>,
    action_mapper: ActionMapper,
    active_dialog: Option<ActiveDialog>,
    keybind_help_open: bool,
    command_palette: CommandPalette,
    create_branch_all: Vec<String>,
    create_branch_filtered: Vec<String>,
    create_branch_index: usize,
    tmux_input: Box<dyn TmuxInput>,
    config_path: PathBuf,
    clipboard: Box<dyn ClipboardAccess>,
    last_tmux_error: Option<String>,
    output_changing: bool,
    agent_output_changing: bool,
    agent_activity_frames: VecDeque<bool>,
    workspace_attention: HashMap<PathBuf, WorkspaceAttention>,
    workspace_attention_ack_markers: HashMap<PathBuf, String>,
    workspace_status_digests: HashMap<String, OutputDigest>,
    workspace_output_changing: HashMap<String, bool>,
    lazygit_sessions: SessionTracker,
    shell_sessions: SessionTracker,
    lazygit_command: String,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_skip_permissions: bool,
    divider_drag_active: bool,
    divider_drag_pointer_offset: i32,
    preview_selection: TextSelectionState,
    copied_text: Option<String>,
    event_log: Box<dyn EventLogger>,
    last_hit_grid: RefCell<Option<HitGrid>>,
    sidebar_list_state: RefCell<VirtualizedListState>,
    last_sidebar_mouse_scroll_at: Option<Instant>,
    last_sidebar_mouse_scroll_delta: i8,
    next_tick_due_at: Option<Instant>,
    next_tick_interval_ms: Option<u64>,
    next_poll_due_at: Option<Instant>,
    last_workspace_status_poll_at: Option<Instant>,
    preview_poll_in_flight: bool,
    preview_poll_requested: bool,
    next_visual_due_at: Option<Instant>,
    interactive_poll_due_at: Option<Instant>,
    fast_animation_frame: usize,
    poll_generation: u64,
    debug_record_start_ts: Option<u64>,
    replay_msg_seq_counter: u64,
    frame_render_seq: RefCell<u64>,
    last_frame_hash: RefCell<u64>,
    input_seq_counter: u64,
    pending_interactive_inputs: VecDeque<PendingInteractiveInput>,
    pending_interactive_sends: VecDeque<QueuedInteractiveSend>,
    interactive_send_in_flight: bool,
    pending_resize_verification: Option<PendingResizeVerification>,
    refresh_in_flight: bool,
    last_manual_refresh_requested_at: Option<Instant>,
    manual_refresh_feedback_pending: bool,
    project_delete_in_flight: bool,
    delete_in_flight: bool,
    delete_in_flight_workspace: Option<PathBuf>,
    pending_delete_workspaces: VecDeque<QueuedDeleteWorkspace>,
    delete_requested_workspaces: HashSet<PathBuf>,
    merge_in_flight: bool,
    update_from_base_in_flight: bool,
    create_in_flight: bool,
    pending_auto_start_workspace: Option<PendingAutoStartWorkspace>,
    pending_create_start_config: Option<StartAgentConfigState>,
    pending_auto_launch_shell_workspace_path: Option<PathBuf>,
    pending_restart_workspace_path: Option<PathBuf>,
    start_in_flight: bool,
    stop_in_flight: bool,
    restart_in_flight: bool,
    deferred_cmds: Vec<Cmd<Msg>>,
}

impl Model for GroveApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.init_model()
    }

    fn update(&mut self, msg: Msg) -> Cmd<Self::Message> {
        app::update(self, msg)
    }

    fn view(&self, frame: &mut Frame) {
        app::view(self, frame);
    }
}
