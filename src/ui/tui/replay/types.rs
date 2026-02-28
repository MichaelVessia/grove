#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayBootstrapSnapshot {
    repo_name: String,
    discovery_state: ReplayDiscoveryState,
    projects: Vec<ProjectConfig>,
    workspaces: Vec<ReplayWorkspace>,
    selected_index: usize,
    focus: ReplayFocus,
    mode: ReplayMode,
    preview_tab: ReplayPreviewTab,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_skip_permissions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayStateSnapshot {
    selected_index: usize,
    workspace_count: usize,
    selected_workspace: Option<String>,
    focus: ReplayFocus,
    mode: ReplayMode,
    preview_tab: ReplayPreviewTab,
    interactive_session: Option<String>,
    poll_generation: u64,
    preview_offset: usize,
    preview_auto_scroll: bool,
    preview_line_count: usize,
    preview_line_hash: u64,
    output_changing: bool,
    pending_input_depth: u64,
    active_modal: Option<String>,
    keybind_help_open: bool,
    command_palette_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySnapshotFile {
    schema_version: u64,
    trace_path: String,
    steps: Vec<ReplaySnapshotStep>,
    final_state: ReplayStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySnapshotStep {
    seq: u64,
    msg_kind: String,
    state: ReplayStateSnapshot,
    frame_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ReplayDiscoveryState {
    Ready,
    Empty,
    Error { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayFocus {
    WorkspaceList,
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayMode {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayPreviewTab {
    Agent,
    Shell,
    Git,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspace {
    name: String,
    path: PathBuf,
    project_name: Option<String>,
    project_path: Option<PathBuf>,
    branch: String,
    base_branch: Option<String>,
    last_activity_unix_secs: Option<i64>,
    agent: ReplayAgentType,
    status: ReplayWorkspaceStatus,
    is_main: bool,
    is_orphaned: bool,
    supported_agent: bool,
    pull_requests: Vec<ReplayPullRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPullRequest {
    number: u64,
    url: String,
    status: ReplayPullRequestStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayPullRequestStatus {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayAgentType {
    Claude,
    Codex,
    Opencode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayWorkspaceStatus {
    Main,
    Idle,
    Active,
    Thinking,
    Waiting,
    Done,
    Error,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayMsg {
    Key {
        event: ReplayKeyEvent,
    },
    Mouse {
        event: ReplayMouseEvent,
    },
    Paste {
        event: ReplayPasteEvent,
    },
    Tick,
    Resize {
        width: u16,
        height: u16,
    },
    PreviewPollCompleted {
        completion: ReplayPreviewPollCompletion,
    },
    LazygitLaunchCompleted {
        completion: ReplayLazygitLaunchCompletion,
    },
    WorkspaceShellLaunchCompleted {
        completion: ReplayWorkspaceShellLaunchCompletion,
    },
    RefreshWorkspacesCompleted {
        completion: ReplayRefreshWorkspacesCompletion,
    },
    DeleteProjectCompleted {
        completion: ReplayDeleteProjectCompletion,
    },
    DeleteWorkspaceCompleted {
        completion: ReplayDeleteWorkspaceCompletion,
    },
    MergeWorkspaceCompleted {
        completion: ReplayMergeWorkspaceCompletion,
    },
    UpdateWorkspaceFromBaseCompleted {
        completion: ReplayUpdateWorkspaceFromBaseCompletion,
    },
    CreateWorkspaceCompleted {
        completion: ReplayCreateWorkspaceCompletion,
    },
    StartAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    StopAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    RestartAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    InteractiveSendCompleted {
        completion: ReplayInteractiveSendCompletion,
    },
    Noop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayKeyEvent {
    code: ReplayKeyCode,
    modifiers: u8,
    kind: ReplayKeyEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayKeyCode {
    Char { value: char },
    Enter,
    Escape,
    Backspace,
    Tab,
    BackTab,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Function { value: u8 },
    Null,
    MediaPlayPause,
    MediaStop,
    MediaNextTrack,
    MediaPrevTrack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayKeyEventKind {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayMouseEvent {
    kind: ReplayMouseEventKind,
    x: u16,
    y: u16,
    modifiers: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayMouseEventKind {
    Down { button: ReplayMouseButton },
    Up { button: ReplayMouseButton },
    Drag { button: ReplayMouseButton },
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayMouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPasteEvent {
    text: String,
    bracketed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPreviewPollCompletion {
    generation: u64,
    live_capture: Option<ReplayLivePreviewCapture>,
    cursor_capture: Option<ReplayCursorCapture>,
    workspace_status_captures: Vec<ReplayWorkspaceStatusCapture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayLivePreviewCapture {
    session: String,
    #[serde(default = "default_live_preview_scrollback_lines")]
    scrollback_lines: usize,
    include_escape_sequences: bool,
    capture_ms: u64,
    total_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCursorCapture {
    session: String,
    capture_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspaceStatusCapture {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    supported_agent: bool,
    capture_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayLazygitLaunchCompletion {
    session_name: String,
    duration_ms: u64,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspaceShellLaunchCompletion {
    session_name: String,
    duration_ms: u64,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayRefreshWorkspacesCompletion {
    preferred_workspace_path: Option<PathBuf>,
    bootstrap: ReplayBootstrapData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayBootstrapData {
    repo_name: String,
    workspaces: Vec<ReplayWorkspace>,
    discovery_state: ReplayDiscoveryState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayDeleteProjectCompletion {
    project_name: String,
    project_path: PathBuf,
    projects: Vec<ProjectConfig>,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayDeleteWorkspaceCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayMergeWorkspaceCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    workspace_branch: String,
    base_branch: String,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayUpdateWorkspaceFromBaseCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    workspace_branch: String,
    base_branch: String,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceCompletion {
    request: ReplayCreateWorkspaceRequest,
    result: ReplayCreateWorkspaceResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceRequest {
    workspace_name: String,
    branch_mode: ReplayBranchMode,
    agent: ReplayAgentType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayBranchMode {
    NewBranch { base_branch: String },
    ExistingBranch { existing_branch: String },
    PullRequest { number: u64, base_branch: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayCreateWorkspaceResult {
    Ok {
        workspace_path: PathBuf,
        branch: String,
        warnings: Vec<String>,
    },
    Err {
        error: ReplayWorkspaceLifecycleError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
enum ReplayWorkspaceLifecycleError {
    EmptyWorkspaceName,
    InvalidWorkspaceName,
    EmptyBaseBranch,
    EmptyExistingBranch,
    InvalidPullRequestNumber,
    RepoNameUnavailable,
    HomeDirectoryUnavailable,
    GitCommandFailed(String),
    Io(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySessionCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayInteractiveSendCompletion {
    send: ReplayQueuedInteractiveSend,
    tmux_send_ms: u64,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayQueuedInteractiveSend {
    command: Vec<String>,
    target_session: String,
    attention_ack_workspace_path: Option<PathBuf>,
    action_kind: String,
    trace_context: Option<ReplayInputTraceContext>,
    literal_chars: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayInputTraceContext {
    seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayStringResult {
    Ok { output: String },
    Err { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayUnitResult {
    Ok,
    Err { error: String },
}

#[derive(Debug, Clone)]
struct ReplayTrace {
    bootstrap: ReplayBootstrapSnapshot,
    messages: Vec<ReplayTraceMessage>,
    states: HashMap<u64, ReplayStateSnapshot>,
    frame_hashes: HashMap<u64, VecDeque<u64>>,
}

#[derive(Debug, Clone)]
struct ReplayTraceMessage {
    seq: u64,
    msg: ReplayMsg,
}

#[derive(Debug, Deserialize)]
struct LoggedLine {
    event: String,
    kind: String,
    data: Value,
}

struct ReplayTmuxInput;

impl TmuxInput for ReplayTmuxInput {
    fn execute(&self, _command: &[String]) -> std::io::Result<()> {
        Ok(())
    }

    fn capture_output(
        &self,
        _target_session: &str,
        _scrollback_lines: usize,
        _include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        Ok(String::new())
    }

    fn capture_cursor_metadata(&self, _target_session: &str) -> std::io::Result<String> {
        Ok("0 0 0 0 0".to_string())
    }

    fn resize_session(
        &self,
        _target_session: &str,
        _target_width: u16,
        _target_height: u16,
    ) -> std::io::Result<()> {
        Ok(())
    }

    fn paste_buffer(&self, _target_session: &str, _text: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn supports_background_send(&self) -> bool {
        true
    }

    fn supports_background_poll(&self) -> bool {
        true
    }

    fn supports_background_launch(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct ReplayClipboard {
    text: String,
}

impl ClipboardAccess for ReplayClipboard {
    fn read_text(&mut self) -> Result<String, String> {
        if self.text.is_empty() {
            return Err("clipboard empty".to_string());
        }

        Ok(self.text.clone())
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.text = text.to_string();
        Ok(())
    }
}

impl GroveApp {
    pub(super) fn replay_enabled(&self) -> bool {
        self.debug_record_start_ts.is_some()
    }

    pub(super) fn record_replay_bootstrap(&self) {
        if !self.replay_enabled() {
            return;
        }

        let data = serde_json::to_value(ReplayBootstrapSnapshot::from_app(self));
        let Ok(snapshot) = data else {
            return;
        };

        self.event_log.log(
            LogEvent::new("replay", "bootstrap")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("bootstrap", snapshot),
        );
    }

    pub(super) fn record_replay_msg_received(&mut self, msg: &Msg) -> u64 {
        if !self.replay_enabled() {
            return 0;
        }

        self.replay_msg_seq_counter = self.replay_msg_seq_counter.saturating_add(1);
        let seq = self.replay_msg_seq_counter;

        let replay_msg = ReplayMsg::from_msg(msg);
        let Ok(encoded) = serde_json::to_value(replay_msg) else {
            return seq;
        };

        self.event_log.log(
            LogEvent::new("replay", "msg_received")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("seq", Value::from(seq))
                .with_data("msg", encoded),
        );

        seq
    }

    pub(super) fn record_replay_state_after_update(&self, seq: u64) {
        if !self.replay_enabled() || seq == 0 {
            return;
        }

        let Ok(snapshot) = serde_json::to_value(ReplayStateSnapshot::from_app(self)) else {
            return;
        };

        self.event_log.log(
            LogEvent::new("replay", "state_after_update")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("seq", Value::from(seq))
                .with_data("state", snapshot),
        );
    }
}

impl ReplayBootstrapSnapshot {
    fn from_app(app: &GroveApp) -> Self {
        Self {
            repo_name: app.repo_name.clone(),
            discovery_state: ReplayDiscoveryState::from_discovery_state(&app.discovery_state),
            projects: app.projects.clone(),
            workspaces: app
                .state
                .workspaces
                .iter()
                .map(ReplayWorkspace::from_workspace)
                .collect(),
            selected_index: app.state.selected_index,
            focus: ReplayFocus::from_focus(app.state.focus),
            mode: ReplayMode::from_mode(app.state.mode),
            preview_tab: ReplayPreviewTab::from_preview_tab(app.preview_tab),
            viewport_width: app.viewport_width,
            viewport_height: app.viewport_height,
            sidebar_width_pct: app.sidebar_width_pct,
            sidebar_hidden: app.sidebar_hidden,
            mouse_capture_enabled: app.mouse_capture_enabled,
            launch_skip_permissions: app.launch_skip_permissions,
        }
    }

    fn to_bootstrap_data(&self) -> BootstrapData {
        BootstrapData {
            repo_name: self.repo_name.clone(),
            workspaces: self
                .workspaces
                .iter()
                .map(ReplayWorkspace::to_workspace)
                .collect(),
            discovery_state: self.discovery_state.to_discovery_state(),
        }
    }
}

impl ReplayStateSnapshot {
    fn from_app(app: &GroveApp) -> Self {
        let preview_line_hash = {
            let mut hasher = DefaultHasher::new();
            app.preview.render_lines.hash(&mut hasher);
            hasher.finish()
        };

        Self {
            selected_index: app.state.selected_index,
            workspace_count: app.state.workspaces.len(),
            selected_workspace: app.selected_workspace_name(),
            focus: ReplayFocus::from_focus(app.state.focus),
            mode: ReplayMode::from_mode(app.state.mode),
            preview_tab: ReplayPreviewTab::from_preview_tab(app.preview_tab),
            interactive_session: app.interactive_target_session(),
            poll_generation: app.poll_generation,
            preview_offset: app.preview.offset,
            preview_auto_scroll: app.preview.auto_scroll,
            preview_line_count: app.preview.render_lines.len(),
            preview_line_hash,
            output_changing: app.output_changing,
            pending_input_depth: app.pending_input_depth(),
            active_modal: app.active_dialog_kind().map(str::to_string),
            keybind_help_open: app.keybind_help_open,
            command_palette_open: app.command_palette.is_visible(),
        }
    }
}

impl ReplayMsg {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Key { .. } => "key",
            Self::Mouse { .. } => "mouse",
            Self::Paste { .. } => "paste",
            Self::Tick => "tick",
            Self::Resize { .. } => "resize",
            Self::PreviewPollCompleted { .. } => "preview_poll_completed",
            Self::LazygitLaunchCompleted { .. } => "lazygit_launch_completed",
            Self::WorkspaceShellLaunchCompleted { .. } => "workspace_shell_launch_completed",
            Self::RefreshWorkspacesCompleted { .. } => "refresh_workspaces_completed",
            Self::DeleteProjectCompleted { .. } => "delete_project_completed",
            Self::DeleteWorkspaceCompleted { .. } => "delete_workspace_completed",
            Self::MergeWorkspaceCompleted { .. } => "merge_workspace_completed",
            Self::UpdateWorkspaceFromBaseCompleted { .. } => "update_workspace_from_base_completed",
            Self::CreateWorkspaceCompleted { .. } => "create_workspace_completed",
            Self::StartAgentCompleted { .. } => "start_agent_completed",
            Self::StopAgentCompleted { .. } => "stop_agent_completed",
            Self::RestartAgentCompleted { .. } => "restart_agent_completed",
            Self::InteractiveSendCompleted { .. } => "interactive_send_completed",
            Self::Noop => "noop",
        }
    }

    fn from_msg(msg: &Msg) -> Self {
        match msg {
            Msg::Key(event) => Self::Key {
                event: ReplayKeyEvent::from_key_event(event),
            },
            Msg::Mouse(event) => Self::Mouse {
                event: ReplayMouseEvent::from_mouse_event(event),
            },
            Msg::Paste(event) => Self::Paste {
                event: ReplayPasteEvent::from_paste_event(event),
            },
            Msg::Tick => Self::Tick,
            Msg::Resize { width, height } => Self::Resize {
                width: *width,
                height: *height,
            },
            Msg::PreviewPollCompleted(completion) => Self::PreviewPollCompleted {
                completion: ReplayPreviewPollCompletion::from_completion(completion),
            },
            Msg::LazygitLaunchCompleted(completion) => Self::LazygitLaunchCompleted {
                completion: ReplayLazygitLaunchCompletion::from_completion(completion),
            },
            Msg::WorkspaceShellLaunchCompleted(completion) => Self::WorkspaceShellLaunchCompleted {
                completion: ReplayWorkspaceShellLaunchCompletion::from_completion(completion),
            },
            Msg::RefreshWorkspacesCompleted(completion) => Self::RefreshWorkspacesCompleted {
                completion: ReplayRefreshWorkspacesCompletion::from_completion(completion),
            },
            Msg::DeleteProjectCompleted(completion) => Self::DeleteProjectCompleted {
                completion: ReplayDeleteProjectCompletion::from_completion(completion),
            },
            Msg::DeleteWorkspaceCompleted(completion) => Self::DeleteWorkspaceCompleted {
                completion: ReplayDeleteWorkspaceCompletion::from_completion(completion),
            },
            Msg::MergeWorkspaceCompleted(completion) => Self::MergeWorkspaceCompleted {
                completion: ReplayMergeWorkspaceCompletion::from_completion(completion),
            },
            Msg::UpdateWorkspaceFromBaseCompleted(completion) => {
                Self::UpdateWorkspaceFromBaseCompleted {
                    completion: ReplayUpdateWorkspaceFromBaseCompletion::from_completion(
                        completion,
                    ),
                }
            }
            Msg::CreateWorkspaceCompleted(completion) => Self::CreateWorkspaceCompleted {
                completion: ReplayCreateWorkspaceCompletion::from_completion(completion),
            },
            Msg::StartAgentCompleted(completion) => Self::StartAgentCompleted {
                completion: ReplaySessionCompletion::from_start_completion(completion),
            },
            Msg::StopAgentCompleted(completion) => Self::StopAgentCompleted {
                completion: ReplaySessionCompletion::from_stop_completion(completion),
            },
            Msg::RestartAgentCompleted(completion) => Self::RestartAgentCompleted {
                completion: ReplaySessionCompletion::from_restart_completion(completion),
            },
            Msg::InteractiveSendCompleted(completion) => Self::InteractiveSendCompleted {
                completion: ReplayInteractiveSendCompletion::from_completion(completion),
            },
            Msg::Noop => Self::Noop,
        }
    }

    fn to_msg(&self) -> Msg {
        match self {
            Self::Key { event } => Msg::Key(event.to_key_event()),
            Self::Mouse { event } => Msg::Mouse(event.to_mouse_event()),
            Self::Paste { event } => Msg::Paste(event.to_paste_event()),
            Self::Tick => Msg::Tick,
            Self::Resize { width, height } => Msg::Resize {
                width: *width,
                height: *height,
            },
            Self::PreviewPollCompleted { completion } => {
                Msg::PreviewPollCompleted(completion.to_completion())
            }
            Self::LazygitLaunchCompleted { completion } => {
                Msg::LazygitLaunchCompleted(completion.to_completion())
            }
            Self::WorkspaceShellLaunchCompleted { completion } => {
                Msg::WorkspaceShellLaunchCompleted(completion.to_completion())
            }
            Self::RefreshWorkspacesCompleted { completion } => {
                Msg::RefreshWorkspacesCompleted(completion.to_completion())
            }
            Self::DeleteProjectCompleted { completion } => {
                Msg::DeleteProjectCompleted(completion.to_completion())
            }
            Self::DeleteWorkspaceCompleted { completion } => {
                Msg::DeleteWorkspaceCompleted(completion.to_completion())
            }
            Self::MergeWorkspaceCompleted { completion } => {
                Msg::MergeWorkspaceCompleted(completion.to_completion())
            }
            Self::UpdateWorkspaceFromBaseCompleted { completion } => {
                Msg::UpdateWorkspaceFromBaseCompleted(completion.to_completion())
            }
            Self::CreateWorkspaceCompleted { completion } => {
                Msg::CreateWorkspaceCompleted(completion.to_completion())
            }
            Self::StartAgentCompleted { completion } => {
                Msg::StartAgentCompleted(completion.to_start_completion())
            }
            Self::StopAgentCompleted { completion } => {
                Msg::StopAgentCompleted(completion.to_stop_completion())
            }
            Self::RestartAgentCompleted { completion } => {
                Msg::RestartAgentCompleted(completion.to_restart_completion())
            }
            Self::InteractiveSendCompleted { completion } => {
                Msg::InteractiveSendCompleted(completion.to_completion())
            }
            Self::Noop => Msg::Noop,
        }
    }
}

impl ReplayKeyEvent {
    fn from_key_event(event: &KeyEvent) -> Self {
        Self {
            code: ReplayKeyCode::from_key_code(event.code),
            modifiers: event.modifiers.bits(),
            kind: ReplayKeyEventKind::from_key_event_kind(event.kind),
        }
    }

    fn to_key_event(&self) -> KeyEvent {
        KeyEvent::new(self.code.to_key_code())
            .with_modifiers(Modifiers::from_bits_retain(self.modifiers))
            .with_kind(self.kind.to_key_event_kind())
    }
}

impl ReplayKeyCode {
    fn from_key_code(code: KeyCode) -> Self {
        match code {
            KeyCode::Char(value) => Self::Char { value },
            KeyCode::Enter => Self::Enter,
            KeyCode::Escape => Self::Escape,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Tab => Self::Tab,
            KeyCode::BackTab => Self::BackTab,
            KeyCode::Delete => Self::Delete,
            KeyCode::Insert => Self::Insert,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::F(value) => Self::Function { value },
            KeyCode::Null => Self::Null,
            KeyCode::MediaPlayPause => Self::MediaPlayPause,
            KeyCode::MediaStop => Self::MediaStop,
            KeyCode::MediaNextTrack => Self::MediaNextTrack,
            KeyCode::MediaPrevTrack => Self::MediaPrevTrack,
        }
    }

    fn to_key_code(&self) -> KeyCode {
        match self {
            Self::Char { value } => KeyCode::Char(*value),
            Self::Enter => KeyCode::Enter,
            Self::Escape => KeyCode::Escape,
            Self::Backspace => KeyCode::Backspace,
            Self::Tab => KeyCode::Tab,
            Self::BackTab => KeyCode::BackTab,
            Self::Delete => KeyCode::Delete,
            Self::Insert => KeyCode::Insert,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PageUp => KeyCode::PageUp,
            Self::PageDown => KeyCode::PageDown,
            Self::Up => KeyCode::Up,
            Self::Down => KeyCode::Down,
            Self::Left => KeyCode::Left,
            Self::Right => KeyCode::Right,
            Self::Function { value } => KeyCode::F(*value),
            Self::Null => KeyCode::Null,
            Self::MediaPlayPause => KeyCode::MediaPlayPause,
            Self::MediaStop => KeyCode::MediaStop,
            Self::MediaNextTrack => KeyCode::MediaNextTrack,
            Self::MediaPrevTrack => KeyCode::MediaPrevTrack,
        }
    }
}

impl ReplayKeyEventKind {
    fn from_key_event_kind(kind: KeyEventKind) -> Self {
        match kind {
            KeyEventKind::Press => Self::Press,
            KeyEventKind::Repeat => Self::Repeat,
            KeyEventKind::Release => Self::Release,
        }
    }

    fn to_key_event_kind(self) -> KeyEventKind {
        match self {
            Self::Press => KeyEventKind::Press,
            Self::Repeat => KeyEventKind::Repeat,
            Self::Release => KeyEventKind::Release,
        }
    }
}

impl ReplayMouseEvent {
    fn from_mouse_event(event: &MouseEvent) -> Self {
        Self {
            kind: ReplayMouseEventKind::from_mouse_event_kind(event.kind),
            x: event.x,
            y: event.y,
            modifiers: event.modifiers.bits(),
        }
    }

    fn to_mouse_event(&self) -> MouseEvent {
        MouseEvent::new(self.kind.to_mouse_event_kind(), self.x, self.y)
            .with_modifiers(Modifiers::from_bits_retain(self.modifiers))
    }
}

impl ReplayMouseEventKind {
    fn from_mouse_event_kind(kind: MouseEventKind) -> Self {
        match kind {
            MouseEventKind::Down(button) => Self::Down {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Up(button) => Self::Up {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Drag(button) => Self::Drag {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Moved => Self::Moved,
            MouseEventKind::ScrollUp => Self::ScrollUp,
            MouseEventKind::ScrollDown => Self::ScrollDown,
            MouseEventKind::ScrollLeft => Self::ScrollLeft,
            MouseEventKind::ScrollRight => Self::ScrollRight,
        }
    }

    fn to_mouse_event_kind(&self) -> MouseEventKind {
        match self {
            Self::Down { button } => MouseEventKind::Down(button.to_mouse_button()),
            Self::Up { button } => MouseEventKind::Up(button.to_mouse_button()),
            Self::Drag { button } => MouseEventKind::Drag(button.to_mouse_button()),
            Self::Moved => MouseEventKind::Moved,
            Self::ScrollUp => MouseEventKind::ScrollUp,
            Self::ScrollDown => MouseEventKind::ScrollDown,
            Self::ScrollLeft => MouseEventKind::ScrollLeft,
            Self::ScrollRight => MouseEventKind::ScrollRight,
        }
    }
}

impl ReplayMouseButton {
    fn from_mouse_button(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
        }
    }

    fn to_mouse_button(self) -> MouseButton {
        match self {
            Self::Left => MouseButton::Left,
            Self::Right => MouseButton::Right,
            Self::Middle => MouseButton::Middle,
        }
    }
}

impl ReplayPasteEvent {
    fn from_paste_event(event: &PasteEvent) -> Self {
        Self {
            text: event.text.clone(),
            bracketed: event.bracketed,
        }
    }

    fn to_paste_event(&self) -> PasteEvent {
        PasteEvent::new(self.text.clone(), self.bracketed)
    }
}

impl ReplayPreviewPollCompletion {
    fn from_completion(completion: &PreviewPollCompletion) -> Self {
        Self {
            generation: completion.generation,
            live_capture: completion
                .live_capture
                .as_ref()
                .map(ReplayLivePreviewCapture::from_capture),
            cursor_capture: completion
                .cursor_capture
                .as_ref()
                .map(ReplayCursorCapture::from_capture),
            workspace_status_captures: completion
                .workspace_status_captures
                .iter()
                .map(ReplayWorkspaceStatusCapture::from_capture)
                .collect(),
        }
    }

    fn to_completion(&self) -> PreviewPollCompletion {
        PreviewPollCompletion {
            generation: self.generation,
            live_capture: self
                .live_capture
                .as_ref()
                .map(ReplayLivePreviewCapture::to_capture),
            cursor_capture: self
                .cursor_capture
                .as_ref()
                .map(ReplayCursorCapture::to_capture),
            workspace_status_captures: self
                .workspace_status_captures
                .iter()
                .map(ReplayWorkspaceStatusCapture::to_capture)
                .collect(),
        }
    }
}

fn default_live_preview_scrollback_lines() -> usize {
    LIVE_PREVIEW_SCROLLBACK_LINES
}

impl ReplayLivePreviewCapture {
    fn from_capture(capture: &LivePreviewCapture) -> Self {
        Self {
            session: capture.session.clone(),
            scrollback_lines: capture.scrollback_lines,
            include_escape_sequences: capture.include_escape_sequences,
            capture_ms: capture.capture_ms,
            total_ms: capture.total_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> LivePreviewCapture {
        LivePreviewCapture {
            session: self.session.clone(),
            scrollback_lines: self.scrollback_lines,
            include_escape_sequences: self.include_escape_sequences,
            capture_ms: self.capture_ms,
            total_ms: self.total_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayCursorCapture {
    fn from_capture(capture: &CursorCapture) -> Self {
        Self {
            session: capture.session.clone(),
            capture_ms: capture.capture_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> CursorCapture {
        CursorCapture {
            session: self.session.clone(),
            capture_ms: self.capture_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayWorkspaceStatusCapture {
    fn from_capture(capture: &WorkspaceStatusCapture) -> Self {
        Self {
            workspace_name: capture.workspace_name.clone(),
            workspace_path: capture.workspace_path.clone(),
            session_name: capture.session_name.clone(),
            supported_agent: capture.supported_agent,
            capture_ms: capture.capture_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> WorkspaceStatusCapture {
        WorkspaceStatusCapture {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            supported_agent: self.supported_agent,
            capture_ms: self.capture_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayLazygitLaunchCompletion {
    fn from_completion(completion: &LazygitLaunchCompletion) -> Self {
        Self {
            session_name: completion.session_name.clone(),
            duration_ms: completion.duration_ms,
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> LazygitLaunchCompletion {
        LazygitLaunchCompletion {
            session_name: self.session_name.clone(),
            duration_ms: self.duration_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayWorkspaceShellLaunchCompletion {
    fn from_completion(completion: &WorkspaceShellLaunchCompletion) -> Self {
        Self {
            session_name: completion.session_name.clone(),
            duration_ms: completion.duration_ms,
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> WorkspaceShellLaunchCompletion {
        WorkspaceShellLaunchCompletion {
            session_name: self.session_name.clone(),
            duration_ms: self.duration_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayRefreshWorkspacesCompletion {
    fn from_completion(completion: &RefreshWorkspacesCompletion) -> Self {
        Self {
            preferred_workspace_path: completion.preferred_workspace_path.clone(),
            bootstrap: ReplayBootstrapData::from_bootstrap_data(&completion.bootstrap),
        }
    }

    fn to_completion(&self) -> RefreshWorkspacesCompletion {
        RefreshWorkspacesCompletion {
            preferred_workspace_path: self.preferred_workspace_path.clone(),
            bootstrap: self.bootstrap.to_bootstrap_data(),
        }
    }
}

impl ReplayBootstrapData {
    fn from_bootstrap_data(data: &BootstrapData) -> Self {
        Self {
            repo_name: data.repo_name.clone(),
            workspaces: data
                .workspaces
                .iter()
                .map(ReplayWorkspace::from_workspace)
                .collect(),
            discovery_state: ReplayDiscoveryState::from_discovery_state(&data.discovery_state),
        }
    }

    fn to_bootstrap_data(&self) -> BootstrapData {
        BootstrapData {
            repo_name: self.repo_name.clone(),
            workspaces: self
                .workspaces
                .iter()
                .map(ReplayWorkspace::to_workspace)
                .collect(),
            discovery_state: self.discovery_state.to_discovery_state(),
        }
    }
}

impl ReplayDeleteProjectCompletion {
    fn from_completion(completion: &DeleteProjectCompletion) -> Self {
        Self {
            project_name: completion.project_name.clone(),
            project_path: completion.project_path.clone(),
            projects: completion.projects.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> DeleteProjectCompletion {
        DeleteProjectCompletion {
            project_name: self.project_name.clone(),
            project_path: self.project_path.clone(),
            projects: self.projects.clone(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayDeleteWorkspaceCompletion {
    fn from_completion(completion: &DeleteWorkspaceCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> DeleteWorkspaceCompletion {
        DeleteWorkspaceCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayMergeWorkspaceCompletion {
    fn from_completion(completion: &MergeWorkspaceCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            workspace_branch: completion.workspace_branch.clone(),
            base_branch: completion.base_branch.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> MergeWorkspaceCompletion {
        MergeWorkspaceCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            workspace_branch: self.workspace_branch.clone(),
            base_branch: self.base_branch.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayUpdateWorkspaceFromBaseCompletion {
    fn from_completion(completion: &UpdateWorkspaceFromBaseCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            workspace_branch: completion.workspace_branch.clone(),
            base_branch: completion.base_branch.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> UpdateWorkspaceFromBaseCompletion {
        UpdateWorkspaceFromBaseCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            workspace_branch: self.workspace_branch.clone(),
            base_branch: self.base_branch.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayCreateWorkspaceCompletion {
    fn from_completion(completion: &CreateWorkspaceCompletion) -> Self {
        Self {
            request: ReplayCreateWorkspaceRequest::from_request(&completion.request),
            result: ReplayCreateWorkspaceResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> CreateWorkspaceCompletion {
        CreateWorkspaceCompletion {
            request: self.request.to_request(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayCreateWorkspaceRequest {
    fn from_request(request: &CreateWorkspaceRequest) -> Self {
        Self {
            workspace_name: request.workspace_name.clone(),
            branch_mode: ReplayBranchMode::from_branch_mode(&request.branch_mode),
            agent: ReplayAgentType::from_agent_type(request.agent),
        }
    }

    fn to_request(&self) -> CreateWorkspaceRequest {
        CreateWorkspaceRequest {
            workspace_name: self.workspace_name.clone(),
            branch_mode: self.branch_mode.to_branch_mode(),
            agent: self.agent.to_agent_type(),
        }
    }
}

impl ReplayBranchMode {
    fn from_branch_mode(mode: &BranchMode) -> Self {
        match mode {
            BranchMode::NewBranch { base_branch } => Self::NewBranch {
                base_branch: base_branch.clone(),
            },
            BranchMode::ExistingBranch { existing_branch } => Self::ExistingBranch {
                existing_branch: existing_branch.clone(),
            },
            BranchMode::PullRequest {
                number,
                base_branch,
            } => Self::PullRequest {
                number: *number,
                base_branch: base_branch.clone(),
            },
        }
    }

    fn to_branch_mode(&self) -> BranchMode {
        match self {
            Self::NewBranch { base_branch } => BranchMode::NewBranch {
                base_branch: base_branch.clone(),
            },
            Self::ExistingBranch { existing_branch } => BranchMode::ExistingBranch {
                existing_branch: existing_branch.clone(),
            },
            Self::PullRequest {
                number,
                base_branch,
            } => BranchMode::PullRequest {
                number: *number,
                base_branch: base_branch.clone(),
            },
        }
    }
}

impl ReplayCreateWorkspaceResult {
    fn from_result(result: &Result<CreateWorkspaceResult, WorkspaceLifecycleError>) -> Self {
        match result {
            Ok(value) => Self::Ok {
                workspace_path: value.workspace_path.clone(),
                branch: value.branch.clone(),
                warnings: value.warnings.clone(),
            },
            Err(error) => Self::Err {
                error: ReplayWorkspaceLifecycleError::from_error(error),
            },
        }
    }

    fn to_result(&self) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
        match self {
            Self::Ok {
                workspace_path,
                branch,
                warnings,
            } => Ok(CreateWorkspaceResult {
                workspace_path: workspace_path.clone(),
                branch: branch.clone(),
                warnings: warnings.clone(),
            }),
            Self::Err { error } => Err(error.to_error()),
        }
    }
}

impl ReplayWorkspaceLifecycleError {
    fn from_error(error: &WorkspaceLifecycleError) -> Self {
        match error {
            WorkspaceLifecycleError::EmptyWorkspaceName => Self::EmptyWorkspaceName,
            WorkspaceLifecycleError::InvalidWorkspaceName => Self::InvalidWorkspaceName,
            WorkspaceLifecycleError::EmptyBaseBranch => Self::EmptyBaseBranch,
            WorkspaceLifecycleError::EmptyExistingBranch => Self::EmptyExistingBranch,
            WorkspaceLifecycleError::InvalidPullRequestNumber => Self::InvalidPullRequestNumber,
            WorkspaceLifecycleError::RepoNameUnavailable => Self::RepoNameUnavailable,
            WorkspaceLifecycleError::HomeDirectoryUnavailable => Self::HomeDirectoryUnavailable,
            WorkspaceLifecycleError::GitCommandFailed(message) => {
                Self::GitCommandFailed(message.clone())
            }
            WorkspaceLifecycleError::Io(message) => Self::Io(message.clone()),
        }
    }

    fn to_error(&self) -> WorkspaceLifecycleError {
        match self {
            Self::EmptyWorkspaceName => WorkspaceLifecycleError::EmptyWorkspaceName,
            Self::InvalidWorkspaceName => WorkspaceLifecycleError::InvalidWorkspaceName,
            Self::EmptyBaseBranch => WorkspaceLifecycleError::EmptyBaseBranch,
            Self::EmptyExistingBranch => WorkspaceLifecycleError::EmptyExistingBranch,
            Self::InvalidPullRequestNumber => WorkspaceLifecycleError::InvalidPullRequestNumber,
            Self::RepoNameUnavailable => WorkspaceLifecycleError::RepoNameUnavailable,
            Self::HomeDirectoryUnavailable => WorkspaceLifecycleError::HomeDirectoryUnavailable,
            Self::GitCommandFailed(message) => {
                WorkspaceLifecycleError::GitCommandFailed(message.clone())
            }
            Self::Io(message) => WorkspaceLifecycleError::Io(message.clone()),
        }
    }
}

impl ReplaySessionCompletion {
    fn from_start_completion(completion: &StartAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn from_stop_completion(completion: &StopAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn from_restart_completion(completion: &RestartAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_start_completion(&self) -> StartAgentCompletion {
        StartAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }

    fn to_stop_completion(&self) -> StopAgentCompletion {
        StopAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }

    fn to_restart_completion(&self) -> RestartAgentCompletion {
        RestartAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayInteractiveSendCompletion {
    fn from_completion(completion: &InteractiveSendCompletion) -> Self {
        Self {
            send: ReplayQueuedInteractiveSend::from_send(&completion.send),
            tmux_send_ms: completion.tmux_send_ms,
            error: completion.error.clone(),
        }
    }

    fn to_completion(&self) -> InteractiveSendCompletion {
        InteractiveSendCompletion {
            send: self.send.to_send(),
            tmux_send_ms: self.tmux_send_ms,
            error: self.error.clone(),
        }
    }
}

impl ReplayQueuedInteractiveSend {
    fn from_send(send: &QueuedInteractiveSend) -> Self {
        Self {
            command: send.command.clone(),
            target_session: send.target_session.clone(),
            attention_ack_workspace_path: send.attention_ack_workspace_path.clone(),
            action_kind: send.action_kind.clone(),
            trace_context: send
                .trace_context
                .as_ref()
                .map(ReplayInputTraceContext::from_trace_context),
            literal_chars: send.literal_chars,
        }
    }

    fn to_send(&self) -> QueuedInteractiveSend {
        QueuedInteractiveSend {
            command: self.command.clone(),
            target_session: self.target_session.clone(),
            attention_ack_workspace_path: self.attention_ack_workspace_path.clone(),
            action_kind: self.action_kind.clone(),
            trace_context: self
                .trace_context
                .as_ref()
                .map(ReplayInputTraceContext::to_trace_context),
            literal_chars: self.literal_chars,
        }
    }
}

impl ReplayInputTraceContext {
    fn from_trace_context(trace_context: &InputTraceContext) -> Self {
        Self {
            seq: trace_context.seq,
        }
    }

    fn to_trace_context(&self) -> InputTraceContext {
        InputTraceContext {
            seq: self.seq,
            received_at: std::time::Instant::now(),
        }
    }
}

impl ReplayStringResult {
    fn from_result(result: &Result<String, String>) -> Self {
        match result {
            Ok(output) => Self::Ok {
                output: output.clone(),
            },
            Err(error) => Self::Err {
                error: error.clone(),
            },
        }
    }

    fn to_result(&self) -> Result<String, String> {
        match self {
            Self::Ok { output } => Ok(output.clone()),
            Self::Err { error } => Err(error.clone()),
        }
    }
}

impl ReplayUnitResult {
    fn from_result(result: &Result<(), String>) -> Self {
        match result {
            Ok(()) => Self::Ok,
            Err(error) => Self::Err {
                error: error.clone(),
            },
        }
    }

    fn to_result(&self) -> Result<(), String> {
        match self {
            Self::Ok => Ok(()),
            Self::Err { error } => Err(error.clone()),
        }
    }
}

impl ReplayFocus {
    fn from_focus(focus: PaneFocus) -> Self {
        match focus {
            PaneFocus::WorkspaceList => Self::WorkspaceList,
            PaneFocus::Preview => Self::Preview,
        }
    }

    fn to_focus(self) -> PaneFocus {
        match self {
            Self::WorkspaceList => PaneFocus::WorkspaceList,
            Self::Preview => PaneFocus::Preview,
        }
    }
}

impl ReplayMode {
    fn from_mode(mode: UiMode) -> Self {
        match mode {
            UiMode::List => Self::List,
            UiMode::Preview => Self::Preview,
        }
    }

    fn to_mode(self) -> UiMode {
        match self {
            Self::List => UiMode::List,
            Self::Preview => UiMode::Preview,
        }
    }
}

impl ReplayPreviewTab {
    fn from_preview_tab(tab: PreviewTab) -> Self {
        match tab {
            PreviewTab::Agent => Self::Agent,
            PreviewTab::Shell => Self::Shell,
            PreviewTab::Git => Self::Git,
        }
    }

    fn to_preview_tab(self) -> PreviewTab {
        match self {
            Self::Agent => PreviewTab::Agent,
            Self::Shell => PreviewTab::Shell,
            Self::Git => PreviewTab::Git,
        }
    }
}

impl ReplayDiscoveryState {
    fn from_discovery_state(state: &DiscoveryState) -> Self {
        match state {
            DiscoveryState::Ready => Self::Ready,
            DiscoveryState::Empty => Self::Empty,
            DiscoveryState::Error(message) => Self::Error {
                message: message.clone(),
            },
        }
    }

    fn to_discovery_state(&self) -> DiscoveryState {
        match self {
            Self::Ready => DiscoveryState::Ready,
            Self::Empty => DiscoveryState::Empty,
            Self::Error { message } => DiscoveryState::Error(message.clone()),
        }
    }
}

impl ReplayAgentType {
    fn from_agent_type(agent: AgentType) -> Self {
        match agent {
            AgentType::Claude => Self::Claude,
            AgentType::Codex => Self::Codex,
            AgentType::OpenCode => Self::Opencode,
        }
    }

    fn to_agent_type(self) -> AgentType {
        match self {
            Self::Claude => AgentType::Claude,
            Self::Codex => AgentType::Codex,
            Self::Opencode => AgentType::OpenCode,
        }
    }
}

impl ReplayWorkspaceStatus {
    fn from_workspace_status(status: WorkspaceStatus) -> Self {
        match status {
            WorkspaceStatus::Main => Self::Main,
            WorkspaceStatus::Idle => Self::Idle,
            WorkspaceStatus::Active => Self::Active,
            WorkspaceStatus::Thinking => Self::Thinking,
            WorkspaceStatus::Waiting => Self::Waiting,
            WorkspaceStatus::Done => Self::Done,
            WorkspaceStatus::Error => Self::Error,
            WorkspaceStatus::Unknown => Self::Unknown,
            WorkspaceStatus::Unsupported => Self::Unsupported,
        }
    }

    fn to_workspace_status(self) -> WorkspaceStatus {
        match self {
            Self::Main => WorkspaceStatus::Main,
            Self::Idle => WorkspaceStatus::Idle,
            Self::Active => WorkspaceStatus::Active,
            Self::Thinking => WorkspaceStatus::Thinking,
            Self::Waiting => WorkspaceStatus::Waiting,
            Self::Done => WorkspaceStatus::Done,
            Self::Error => WorkspaceStatus::Error,
            Self::Unknown => WorkspaceStatus::Unknown,
            Self::Unsupported => WorkspaceStatus::Unsupported,
        }
    }
}

impl ReplayPullRequestStatus {
    fn from_pull_request_status(status: PullRequestStatus) -> Self {
        match status {
            PullRequestStatus::Open => Self::Open,
            PullRequestStatus::Merged => Self::Merged,
            PullRequestStatus::Closed => Self::Closed,
        }
    }

    fn to_pull_request_status(self) -> PullRequestStatus {
        match self {
            Self::Open => PullRequestStatus::Open,
            Self::Merged => PullRequestStatus::Merged,
            Self::Closed => PullRequestStatus::Closed,
        }
    }
}

impl ReplayPullRequest {
    fn from_pull_request(pull_request: &PullRequest) -> Self {
        Self {
            number: pull_request.number,
            url: pull_request.url.clone(),
            status: ReplayPullRequestStatus::from_pull_request_status(pull_request.status),
        }
    }

    fn to_pull_request(&self) -> PullRequest {
        PullRequest {
            number: self.number,
            url: self.url.clone(),
            status: self.status.to_pull_request_status(),
        }
    }
}

impl ReplayWorkspace {
    fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            name: workspace.name.clone(),
            path: workspace.path.clone(),
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            branch: workspace.branch.clone(),
            base_branch: workspace.base_branch.clone(),
            last_activity_unix_secs: workspace.last_activity_unix_secs,
            agent: ReplayAgentType::from_agent_type(workspace.agent),
            status: ReplayWorkspaceStatus::from_workspace_status(workspace.status),
            is_main: workspace.is_main,
            is_orphaned: workspace.is_orphaned,
            supported_agent: workspace.supported_agent,
            pull_requests: workspace
                .pull_requests
                .iter()
                .map(ReplayPullRequest::from_pull_request)
                .collect(),
        }
    }

    fn to_workspace(&self) -> Workspace {
        Workspace {
            name: self.name.clone(),
            path: self.path.clone(),
            project_name: self.project_name.clone(),
            project_path: self.project_path.clone(),
            branch: self.branch.clone(),
            base_branch: self.base_branch.clone(),
            last_activity_unix_secs: self.last_activity_unix_secs,
            agent: self.agent.to_agent_type(),
            status: self.status.to_workspace_status(),
            is_main: self.is_main,
            is_orphaned: self.is_orphaned,
            supported_agent: self.supported_agent,
            pull_requests: self
                .pull_requests
                .iter()
                .map(ReplayPullRequest::to_pull_request)
                .collect(),
        }
    }
}

