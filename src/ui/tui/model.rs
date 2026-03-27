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
use ftui::widgets::input::TextInput;
use ftui::widgets::list::{List, ListItem, ListState};
use ftui::widgets::modal::{BackdropConfig, Modal, ModalSizeConstraints};
use ftui::widgets::notification_queue::{
    NotificationPriority, NotificationQueue, NotificationStack, QueueConfig,
};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::toast::{Toast, ToastIcon, ToastPosition, ToastStyle};
use ftui::widgets::virtualized::{Virtualized, VirtualizedListState};
use ftui::widgets::{FocusManager, FocusNode, NavDirection};
use ftui::widgets::{StatefulWidget, StatusItem, StatusLine, Widget};
use ftui::{Cmd, Model, PackedRgba, Style};
use ftui_extras::text_effects::{AnimationClock, ColorGradient, StyledText, TextEffect};
use serde_json::Value;

use crate::application::agent_runtime::capture::{
    evaluate_capture_change, tmux_capture_error_indicates_missing_session,
};
use crate::application::agent_runtime::{
    CommandExecutionMode, LivePreviewTarget, OutputDigest, SessionActivity, ShellLaunchRequest,
    TaskLaunchRequest, WorkspaceStatusTarget, execute_command_with, git_session_name_for_workspace,
    infer_workspace_permission_mode, poll_interval, restart_workspace_in_pane_with_io,
    session_name_for_task, session_name_for_workspace_ref, shell_session_name_for_workspace,
    tmux_launch_error_indicates_duplicate_session, trimmed_nonempty,
    workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
};
use crate::application::agent_runtime::{
    detect_status_with_session_override, execute_launch_request_with_result_for_mode,
    execute_restart_workspace_in_pane_with_result, execute_shell_launch_request_for_mode,
    execute_stop_task_with_result_for_mode, execute_stop_workspace_with_result_for_mode,
    execute_task_launch_request_with_result_for_mode, latest_assistant_attention_marker,
    launch_request_for_workspace, shell_launch_request_for_workspace,
    status::detect_waiting_prompt,
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
use crate::application::workspace_lifecycle::{
    CommandGitRunner, CommandSetupCommandRunner, CommandSetupScriptRunner, DeleteWorkspaceRequest,
    MergeWorkspaceRequest, RuntimeSessionTerminator, UpdateWorkspaceFromBaseRequest,
    WorkspaceLifecycleError, delete_workspace, merge_workspace_with_terminator,
    update_workspace_from_base_with_terminator, workspace_lifecycle_error_message,
    write_workspace_base_marker,
};
use crate::domain::{AgentType, PermissionMode, Task, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::DiscoveryState;
use crate::infrastructure::config::{
    AgentEnvDefaults, GroveConfig, ProjectConfig, ThemeName, WorkspaceAttentionAckConfig,
};
use crate::infrastructure::event_log::{Event as LogEvent, EventLogger, now_millis};
use crate::infrastructure::paths::refer_to_same_location;
use crate::infrastructure::process_metrics::{ProcessMetricsSampler, ProcessMetricsSnapshot};
use crate::ui::mouse::{clamp_sidebar_ratio, ratio_from_drag};
use crate::ui::state::{Action, AppState, UiMode};
use performance::DurationWindow;

#[cfg(test)]
use bootstrap_config::AppDependencies;
use bootstrap_config::{
    project_display_name, read_workspace_init_command, read_workspace_launch_prompt,
    read_workspace_permission_mode, write_workspace_init_command, write_workspace_permission_mode,
};
use commands::*;
use dialogs::*;
use dialogs_state::*;
use msg::*;
use selection::{TextSelectionPoint, TextSelectionState};
use terminal::{
    ClipboardAccess, CommandTmuxInput, PreviewStreamSource, PreviewStreamState,
    SystemClipboardAccess, TmuxInput, parse_cursor_metadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedDeleteWorkspace {
    request: QueuedDeleteRequest,
    workspace_name: String,
    workspace_path: PathBuf,
    requested_workspace_paths: Vec<PathBuf>,
    deleted_task: bool,
    removed_base_task: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AttentionReason {
    BlockedOnQuestion,
    PermissionWall,
    SessionEnded,
    Finished,
    Stalled,
}

impl AttentionReason {
    const fn rank(self) -> u8 {
        match self {
            Self::BlockedOnQuestion => 0,
            Self::PermissionWall => 1,
            Self::SessionEnded => 2,
            Self::Finished => 3,
            Self::Stalled => 4,
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::BlockedOnQuestion => "blocked on question",
            Self::PermissionWall => "permission wall",
            Self::SessionEnded => "session ended unexpectedly",
            Self::Finished => "finished, awaiting review",
            Self::Stalled => "stalled, no output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttentionItem {
    fingerprint: String,
    reason: AttentionReason,
    summary: String,
    workspace_path: PathBuf,
    task_slug: String,
    first_seen_at_ms: u64,
    last_seen_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttentionObservation {
    item: AttentionItem,
    seen_polls: u8,
    missing_polls: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarSelectable {
    Attention(usize),
    Workspace(usize),
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
    Performance(PerformanceDialogState),
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
    last_live_preview_session: Option<String>,
    pending_selected_session_bootstrap: Option<String>,
    recent_local_echo_session: Option<String>,
    agent_working_until: Option<Instant>,
    agent_idle_polls_since_output: u8,
    workspace_status_digests: HashMap<PathBuf, OutputDigest>,
    workspace_output_changing: HashMap<PathBuf, bool>,
    workspace_waiting_prompts: HashMap<PathBuf, String>,
    workspace_idle_polls_since_output: HashMap<PathBuf, u8>,
    next_tick_due_at: Option<Instant>,
    next_tick_interval_ms: Option<u64>,
    next_tick_source: Option<String>,
    next_tick_trigger: Option<String>,
    next_poll_due_at: Option<Instant>,
    last_workspace_status_poll_at: Option<Instant>,
    preview_poll_in_flight: bool,
    preview_poll_requested: bool,
    next_visual_due_at: Option<Instant>,
    interactive_poll_due_at: Option<Instant>,
    activity_animation: AnimationClock,
    poll_generation: u64,
    preview_session_geometry: Option<PreviewSessionGeometry>,
    last_diff_poll_at: Option<Instant>,
    last_diff_stat_poll_at: Option<Instant>,
    diff_capture_in_flight: bool,
    diff_stat_in_flight: bool,
    preview_stream: PreviewStreamState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiffStatBadge {
    insertions: usize,
    deletions: usize,
}

struct PerformanceState {
    redraw_timing: RefCell<DurationWindow>,
    draw_timing: RefCell<DurationWindow>,
    view_timing: RefCell<DurationWindow>,
    last_redraw_started_at: RefCell<Option<Instant>>,
    process_sampler: RefCell<ProcessMetricsSampler>,
    process_metrics: RefCell<ProcessMetricsSnapshot>,
    last_process_refresh_at: RefCell<Option<Instant>>,
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
    hidden_base_project_paths: HashSet<PathBuf>,
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
    attention_observations: HashMap<PathBuf, AttentionObservation>,
    attention_items: Vec<AttentionItem>,
    selected_attention_item: Option<usize>,
    startup_attention_focus_pending: bool,
    #[cfg(test)]
    attention_marker_overrides: HashMap<PathBuf, Option<String>>,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    panes: panes::GrovePaneModel,
    theme_name: ThemeName,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_permission_mode: PermissionMode,
    divider_resize: PaneDragResizeMachine,
    divider_resize_anchor_x: i32,
    divider_resize_event_seq: u64,
    preview_selection: TextSelectionState,
    copied_text: Option<String>,
    telemetry: TelemetryState,
    performance: PerformanceState,
    last_hit_grid: RefCell<Option<HitGrid>>,
    preview_scroll: RefCell<Virtualized<()>>,
    sidebar_list_state: RefCell<VirtualizedListState>,
    last_sidebar_mouse_scroll_at: Option<Instant>,
    workspace_diff_stats: HashMap<PathBuf, DiffStatBadge>,
    last_sidebar_mouse_scroll_delta: i8,
    focus_manager: FocusManager,
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

    fn subscriptions(&self) -> Vec<Box<dyn ftui::runtime::Subscription<Self::Message>>> {
        self.preview_stream_subscription().into_iter().collect()
    }
}

impl GroveApp {
    fn build_main_focus_manager() -> FocusManager {
        let mut focus_manager = FocusManager::new();
        focus_manager.graph_mut().insert(
            FocusNode::new(FOCUS_ID_WORKSPACE_LIST, Rect::new(0, 0, 1, 1)).with_tab_index(0),
        );
        focus_manager
            .graph_mut()
            .insert(FocusNode::new(FOCUS_ID_PREVIEW, Rect::new(0, 0, 1, 1)).with_tab_index(1));
        focus_manager.graph_mut().connect(
            FOCUS_ID_WORKSPACE_LIST,
            NavDirection::Right,
            FOCUS_ID_PREVIEW,
        );
        focus_manager.graph_mut().connect(
            FOCUS_ID_PREVIEW,
            NavDirection::Left,
            FOCUS_ID_WORKSPACE_LIST,
        );
        focus_manager.create_group(
            FOCUS_GROUP_MAIN_PANES,
            vec![FOCUS_ID_WORKSPACE_LIST, FOCUS_ID_PREVIEW],
        );
        let _ = focus_manager.focus(FOCUS_ID_WORKSPACE_LIST);
        focus_manager
    }

    fn sync_main_pane_state_from_focus_manager(&mut self) {
        match self.focus_manager.current() {
            Some(FOCUS_ID_WORKSPACE_LIST) => {
                self.state.mode = UiMode::List;
            }
            Some(FOCUS_ID_PREVIEW) => {
                self.state.mode = UiMode::Preview;
            }
            _ => {}
        }
    }

    pub(super) fn active_main_pane_mode(&self) -> UiMode {
        match self.focus_manager.current() {
            Some(FOCUS_ID_WORKSPACE_LIST) => UiMode::List,
            Some(FOCUS_ID_PREVIEW) => UiMode::Preview,
            _ => self.state.mode,
        }
    }

    pub(super) fn workspace_list_focused(&self) -> bool {
        self.focus_manager.current() == Some(FOCUS_ID_WORKSPACE_LIST)
    }

    pub(super) fn preview_focused(&self) -> bool {
        self.focus_manager.current() == Some(FOCUS_ID_PREVIEW)
    }

    pub(super) fn focus_main_pane(&mut self, focus_id: u64) -> bool {
        let moved = self.focus_manager.current() != Some(focus_id);
        let _ = self.focus_manager.focus(focus_id);
        self.sync_main_pane_state_from_focus_manager();
        moved
    }

    pub(super) fn navigate_main_panes(&mut self, dir: NavDirection) -> bool {
        let moved = self.focus_manager.navigate(dir);
        self.sync_main_pane_state_from_focus_manager();
        moved
    }

    pub(super) fn sync_main_focus_nodes(&mut self) {
        let (sidebar_rect, _, preview_rect) = self.effective_workspace_rects();
        self.focus_manager.graph_mut().insert(
            FocusNode::new(FOCUS_ID_WORKSPACE_LIST, sidebar_rect)
                .with_tab_index(0)
                .with_focusable(!self.sidebar_hidden && !sidebar_rect.is_empty()),
        );
        self.focus_manager.graph_mut().insert(
            FocusNode::new(FOCUS_ID_PREVIEW, preview_rect)
                .with_tab_index(1)
                .with_focusable(!preview_rect.is_empty()),
        );

        if self.sidebar_hidden && self.focus_manager.current() == Some(FOCUS_ID_WORKSPACE_LIST) {
            let _ = self.focus_manager.focus(FOCUS_ID_PREVIEW);
        }

        self.sync_main_pane_state_from_focus_manager();
    }

    pub(super) fn activate_focus_trap(&mut self, group_id: u32, members: &[u64], initial: u64) {
        for (index, id) in members.iter().copied().enumerate() {
            let Some(tab_index) = i32::try_from(index).ok() else {
                continue;
            };
            self.focus_manager.graph_mut().insert(
                FocusNode::new(id, Rect::new(0, 0, 1, 1))
                    .with_tab_index(tab_index)
                    .with_group(group_id),
            );
        }
        self.focus_manager.create_group(group_id, members.to_vec());
        self.focus_manager.push_trap(group_id);
        let _ = self.focus_manager.focus(initial);
    }

    pub(super) fn deactivate_focus_trap(&mut self, members: &[u64]) {
        let _ = self.focus_manager.pop_trap();
        for id in members {
            let _ = self.focus_manager.graph_mut().remove(*id);
        }
        self.sync_main_pane_state_from_focus_manager();
    }

    #[cfg(test)]
    pub(super) fn current_focus_id(&self) -> Option<u64> {
        self.focus_manager.current()
    }

    #[cfg(test)]
    pub(super) fn focus_node_bounds(&self, id: u64) -> Option<Rect> {
        self.focus_manager.graph().get(id).map(|node| node.bounds)
    }

    #[cfg(test)]
    pub(super) fn focus_node_is_focusable(&self, id: u64) -> Option<bool> {
        self.focus_manager.graph().get(id).map(|node| node.is_focusable)
    }

    #[cfg(test)]
    pub(super) fn focus_is_trapped(&self) -> bool {
        self.focus_manager.is_trapped()
    }

    #[cfg(test)]
    pub(super) fn focus_next_for_test(&mut self) -> bool {
        self.focus_manager.focus_next()
    }

    #[cfg(test)]
    pub(super) fn focus_prev_for_test(&mut self) -> bool {
        self.focus_manager.focus_prev()
    }
}
