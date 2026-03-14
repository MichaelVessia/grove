use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use ftui::core::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    PasteEvent,
};
use ftui::core::geometry::Rect;
use ftui::core::keybinding::{
    Action as KeybindingAction, ActionConfig as KeybindingConfig, ActionMapper,
    AppState as KeybindingAppState, SequenceConfig as KeySequenceConfig,
};
use ftui::layout::pane::PaneDragResizeMachine;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::{Frame, HitGrid, HitId, HitRegion as FrameHitRegion};
use ftui::text::{
    Line as FtLine, Span as FtSpan, Text as FtText, display_width as text_display_width,
};
use ftui::widgets::block::{Alignment as BlockAlignment, Block};
use ftui::widgets::borders::Borders;
use ftui::widgets::command_palette::{
    ActionItem as PaletteActionItem, CommandPalette, PaletteAction, PaletteStyle,
};
use ftui::widgets::modal::{BackdropConfig, Modal, ModalSizeConstraints};
use ftui::widgets::notification_queue::{
    NotificationPriority, NotificationQueue, NotificationStack, QueueConfig,
};
use ftui::widgets::input::TextInput;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::{StatefulWidget, StatusItem, StatusLine, Widget};
use ftui::widgets::toast::{Toast, ToastIcon, ToastPosition, ToastStyle};
use ftui::widgets::virtualized::{Virtualized, VirtualizedListState};
use ftui::{Cmd, Model, PackedRgba, Style};
use ftui_extras::text_effects::{AnimationClock, ColorGradient, StyledText, TextEffect};
use serde_json::Value;

use crate::application::agent_runtime::capture::{
    evaluate_capture_change, tmux_capture_error_indicates_missing_session,
};
use crate::application::agent_runtime::{
    CommandExecutionMode, LivePreviewTarget, OutputDigest, SessionActivity, ShellLaunchRequest,
    TaskLaunchRequest, WorkspaceStatusTarget, execute_command_with,
    git_session_name_for_workspace, infer_workspace_skip_permissions, poll_interval,
    restart_workspace_in_pane_with_io, session_name_for_task, session_name_for_workspace_ref,
    shell_session_name_for_workspace, tmux_launch_error_indicates_duplicate_session,
    trimmed_nonempty, workspace_can_enter_interactive, workspace_can_start_agent,
    workspace_can_stop_agent,
};
use crate::application::interactive::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload,
    multiplexer_send_input_command,
};
use crate::application::preview::PreviewState;
use crate::application::session_cleanup::{
    SessionCleanupOptions, SessionCleanupPlan, SessionCleanupReason, apply_session_cleanup,
    plan_session_cleanup_for_tasks,
};
use crate::application::task_lifecycle::{
    AddWorktreeToTaskRequest, AddWorktreeToTaskResult, CreateTaskRequest, CreateTaskResult,
    DeleteTaskRequest, TaskLifecycleError, create_task, create_task_in_root, delete_task,
    task_lifecycle_error_message,
};
use crate::application::agent_runtime::{
    detect_status_with_session_override, execute_launch_request_with_result_for_mode,
    execute_restart_workspace_in_pane_with_result, execute_shell_launch_request_for_mode,
    execute_stop_task_with_result_for_mode, execute_stop_workspace_with_result_for_mode,
    execute_task_launch_request_with_result_for_mode, latest_assistant_attention_marker,
    launch_request_for_workspace, shell_launch_request_for_workspace,
};
use crate::application::workspace_lifecycle::{
    CommandGitRunner, CommandSetupCommandRunner, CommandSetupScriptRunner, DeleteWorkspaceRequest,
    MergeWorkspaceRequest, RuntimeSessionTerminator, UpdateWorkspaceFromBaseRequest,
    WorkspaceLifecycleError, delete_workspace, merge_workspace_with_terminator,
    update_workspace_from_base_with_terminator, workspace_lifecycle_error_message,
    write_workspace_base_marker,
};
use crate::domain::{AgentType, Task, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::DiscoveryState;
use crate::infrastructure::config::{
    AgentEnvDefaults, GroveConfig, ProjectConfig, ThemeName, WorkspaceAttentionAckConfig,
};
use crate::infrastructure::event_log::{Event as LogEvent, EventLogger};
use crate::infrastructure::paths::refer_to_same_location;
use crate::ui::mouse::{clamp_sidebar_ratio, ratio_from_drag};
use crate::ui::state::{Action, AppState, PaneFocus, UiMode, reduce};

#[cfg(test)]
use bootstrap_config::AppDependencies;
use bootstrap_config::{
    project_display_name, read_workspace_init_command, read_workspace_launch_prompt,
    read_workspace_skip_permissions, write_workspace_init_command, write_workspace_skip_permissions,
};
use terminal::{
    ClipboardAccess, CommandTmuxInput, SystemClipboardAccess, TmuxInput, parse_cursor_metadata,
};
use selection::{TextSelectionPoint, TextSelectionState};
use msg::*;
use shared::*;
use dialogs::*;
use dialogs_state::*;
use commands::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedDeleteWorkspace {
    request: QueuedDeleteRequest,
    workspace_name: String,
    workspace_path: PathBuf,
    requested_workspace_paths: Vec<PathBuf>,
    deleted_task: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueuedDeleteRequest {
    Task(DeleteTaskRequest),
    Worktree(DeleteWorkspaceRequest),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskReorderState {
    original_task_order: Vec<String>,
    moving_task_slug: String,
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

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
enum ActiveDialog {
    Launch(LaunchDialogState),
    Stop(StopDialogState),
    Confirm(ConfirmDialogState),
    SessionCleanup(SessionCleanupDialogState),
    Delete(DeleteDialogState),
    Merge(MergeDialogState),
    UpdateFromBase(UpdateFromBaseDialogState),
    PullUpstream(PullUpstreamDialogState),
    Create(CreateDialogState),
    Edit(EditDialogState),
    RenameTab(RenameTabDialogState),
    Project(Box<ProjectDialogState>),
    Settings(SettingsDialogState),
}

struct SessionState {
    interactive: Option<InteractiveState>,
    last_tmux_error: Option<String>,
    agent_sessions: SessionTracker,
    lazygit_sessions: SessionTracker,
    shell_sessions: SessionTracker,
    lazygit_command: String,
    pending_interactive_inputs: VecDeque<PendingInteractiveInput>,
    pending_interactive_sends: VecDeque<QueuedInteractiveSend>,
    interactive_send_in_flight: bool,
    pending_resize_verification: Option<PendingResizeVerification>,
    pending_restart_workspace_path: Option<PathBuf>,
}

struct PollingState {
    output_changing: bool,
    agent_output_changing: bool,
    agent_working_until: Option<Instant>,
    agent_idle_polls_since_output: u8,
    workspace_status_digests: HashMap<PathBuf, OutputDigest>,
    workspace_output_changing: HashMap<PathBuf, bool>,
    next_tick_due_at: Option<Instant>,
    next_tick_interval_ms: Option<u64>,
    next_poll_due_at: Option<Instant>,
    last_workspace_status_poll_at: Option<Instant>,
    preview_poll_in_flight: bool,
    preview_poll_requested: bool,
    next_visual_due_at: Option<Instant>,
    interactive_poll_due_at: Option<Instant>,
    activity_animation: AnimationClock,
    poll_generation: u64,
}

struct DialogState {
    active_dialog: Option<ActiveDialog>,
    keybind_help_open: bool,
    command_palette: CommandPalette,
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
    pull_upstream_in_flight: bool,
    create_in_flight: bool,
    start_in_flight: bool,
    stop_in_flight: bool,
    restart_in_flight: bool,
}

struct TelemetryState {
    event_log: Box<dyn EventLogger>,
    debug_record_start_ts: Option<u64>,
    replay_msg_seq_counter: u64,
    frame_render_seq: RefCell<u64>,
    last_frame_hash: RefCell<u64>,
    input_seq_counter: u64,
    deferred_cmds: Vec<Cmd<Msg>>,
}

struct GroveApp {
    repo_name: String,
    projects: Vec<ProjectConfig>,
    task_order: Vec<String>,
    task_reorder: Option<TaskReorderState>,
    state: AppState,
    discovery_state: DiscoveryState,
    preview_tab: PreviewTab,
    workspace_tabs: HashMap<PathBuf, WorkspaceTabsState>,
    last_agent_selection: HashMap<PathBuf, AgentType>,
    preview: PreviewState,
    notifications: NotificationQueue,
    action_mapper: ActionMapper,
    dialogs: DialogState,
    tmux_input: Box<dyn TmuxInput>,
    config_path: PathBuf,
    clipboard: Box<dyn ClipboardAccess>,
    session: SessionState,
    polling: PollingState,
    workspace_attention: HashMap<PathBuf, WorkspaceAttention>,
    workspace_attention_ack_markers: HashMap<PathBuf, String>,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    panes: panes::GrovePaneModel,
    theme_name: ThemeName,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_skip_permissions: bool,
    divider_resize: PaneDragResizeMachine,
    divider_resize_anchor_x: i32,
    divider_resize_event_seq: u64,
    preview_selection: TextSelectionState,
    copied_text: Option<String>,
    telemetry: TelemetryState,
    last_hit_grid: RefCell<Option<HitGrid>>,
    preview_scroll: RefCell<Virtualized<()>>,
    sidebar_list_state: RefCell<VirtualizedListState>,
    last_sidebar_mouse_scroll_at: Option<Instant>,
    last_sidebar_mouse_scroll_delta: i8,
    #[cfg(test)]
    task_root_override: Option<PathBuf>,
    #[cfg(test)]
    pull_request_branch_name_override: Option<String>,
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
