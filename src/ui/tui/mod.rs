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
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::{Frame, HitGrid, HitId, HitRegion as FrameHitRegion};
use ftui::text::{
    Line as FtLine, Span as FtSpan, Text as FtText, display_width as text_display_width,
};
use ftui::widgets::Widget;
use ftui::widgets::block::{Alignment as BlockAlignment, Block};
use ftui::widgets::borders::Borders;
use ftui::widgets::command_palette::{
    ActionItem as PaletteActionItem, CommandPalette, PaletteAction,
};
use ftui::widgets::modal::{BackdropConfig, Modal, ModalSizeConstraints};
use ftui::widgets::notification_queue::{
    NotificationPriority, NotificationQueue, NotificationStack, QueueConfig,
};
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::toast::{Toast, ToastIcon, ToastPosition, ToastStyle};
use ftui::widgets::virtualized::VirtualizedListState;
use ftui::{Cmd, Model, PackedRgba, Style};
use ftui_extras::text_effects::{ColorGradient, StyledText, TextEffect};
use serde_json::Value;

use crate::application::agent_runtime::{
    CommandExecutionMode, LivePreviewTarget, OutputDigest, SessionActivity, ShellLaunchRequest,
    WorkspaceStatusTarget, agent_supports_in_pane_restart, detect_status_with_session_override,
    evaluate_capture_change, execute_command_with, execute_launch_request_with_result_for_mode,
    execute_restart_workspace_in_pane_with_result, execute_shell_launch_request_for_mode,
    execute_stop_workspace_with_result_for_mode, git_session_name_for_workspace,
    infer_workspace_skip_permissions, latest_assistant_attention_marker,
    launch_request_for_workspace, poll_interval, restart_workspace_in_pane_with_io,
    session_name_for_workspace_ref, shell_launch_request_for_workspace,
    shell_session_name_for_workspace, tmux_capture_error_indicates_missing_session,
    tmux_launch_error_indicates_duplicate_session, trimmed_nonempty,
    workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
    workspace_status_targets_for_polling_with_live_preview,
};
#[cfg(test)]
use crate::application::interactive::render_cursor_overlay;
use crate::application::interactive::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload,
    multiplexer_send_input_command, render_cursor_overlay_ansi,
};
use crate::application::preview::PreviewState;
use crate::application::workspace_lifecycle::{
    BranchMode, CommandGitRunner, CommandSetupCommandRunner, CommandSetupScriptRunner,
    CreateWorkspaceRequest, CreateWorkspaceResult, DeleteWorkspaceRequest, MergeWorkspaceRequest,
    UpdateWorkspaceFromBaseRequest, WorkspaceLifecycleError, create_workspace_with_template,
    delete_workspace, merge_workspace, update_workspace_from_base,
    workspace_lifecycle_error_message, write_workspace_agent_marker, write_workspace_base_marker,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
use crate::infrastructure::config::{
    AgentEnvDefaults, GroveConfig, ProjectConfig, WorkspaceAttentionAckConfig,
};
use crate::infrastructure::event_log::{Event as LogEvent, EventLogger};
use crate::ui::mouse::{clamp_sidebar_ratio, ratio_from_drag};
use crate::ui::state::{Action, AppState, PaneFocus, UiMode, reduce};

mod ansi;
#[cfg(test)]
use ansi::ansi_16_color;
use ansi::ansi_lines_to_styled_lines;
#[path = "bootstrap/bootstrap_app.rs"]
mod bootstrap_app;
#[path = "bootstrap/bootstrap_config.rs"]
mod bootstrap_config;
#[path = "bootstrap/bootstrap_discovery.rs"]
mod bootstrap_discovery;
use crate::infrastructure::paths::refer_to_same_location;
#[cfg(test)]
use bootstrap_config::AppDependencies;
use bootstrap_config::{
    filter_branches, load_local_branches, project_display_name, read_workspace_init_command,
    read_workspace_launch_prompt, read_workspace_skip_permissions, write_workspace_init_command,
    write_workspace_skip_permissions,
};
use bootstrap_discovery::bootstrap_data_for_projects;
mod terminal;
use terminal::{
    ClipboardAccess, CommandTmuxInput, SystemClipboardAccess, TmuxInput, parse_cursor_metadata,
};
#[macro_use]
mod shared;
use shared::*;
#[path = "dialogs/dialogs.rs"]
mod dialogs;
#[path = "dialogs/dialogs_confirm.rs"]
mod dialogs_confirm;
#[path = "dialogs/dialogs_create_key.rs"]
mod dialogs_create_key;
#[path = "dialogs/dialogs_create_setup.rs"]
mod dialogs_create_setup;
#[path = "dialogs/dialogs_delete.rs"]
mod dialogs_delete;
#[path = "dialogs/dialogs_edit.rs"]
mod dialogs_edit;
#[path = "dialogs/dialogs_launch.rs"]
mod dialogs_launch;
#[path = "dialogs/dialogs_merge.rs"]
mod dialogs_merge;
#[path = "dialogs/dialogs_projects_add.rs"]
mod dialogs_projects_add;
#[path = "dialogs/dialogs_projects_key.rs"]
mod dialogs_projects_key;
#[path = "dialogs/dialogs_projects_state.rs"]
mod dialogs_projects_state;
#[path = "dialogs/dialogs_settings.rs"]
mod dialogs_settings;
#[path = "dialogs/state.rs"]
mod dialogs_state;
#[path = "dialogs/dialogs_stop.rs"]
mod dialogs_stop;
#[path = "dialogs/dialogs_update_from_base.rs"]
mod dialogs_update_from_base;
use dialogs::*;
use dialogs_state::*;
#[path = "commands/catalog.rs"]
mod commands;
#[path = "commands/help.rs"]
mod commands_hints;
#[path = "commands/palette.rs"]
mod commands_palette;
use commands::*;
mod msg;
use msg::*;
#[path = "logging/logging_frame.rs"]
mod logging_frame;
#[path = "logging/logging_input.rs"]
mod logging_input;
#[path = "logging/logging_state.rs"]
mod logging_state;
mod selection;
use selection::{TextSelectionPoint, TextSelectionState};
mod runner;
pub use runner::{run_with_debug_record, run_with_event_log};
mod replay;
pub use replay::{ReplayOptions, emit_replay_fixture, replay_debug_record};
mod text;
use text::{
    ansi_line_to_plain_text, chrome_bar_line, keybind_hint_spans, line_visual_width,
    pad_or_truncate_to_display_width, truncate_for_log, truncate_to_display_width,
    visual_grapheme_at, visual_substring,
};
#[path = "update/update.rs"]
mod update;
#[path = "update/update_core.rs"]
mod update_core;
#[path = "update/update_input_interactive.rs"]
mod update_input_interactive;
#[path = "update/update_input_interactive_clipboard.rs"]
mod update_input_interactive_clipboard;
#[path = "update/update_input_interactive_send.rs"]
mod update_input_interactive_send;
#[path = "update/update_input_key_events.rs"]
mod update_input_key_events;
#[path = "update/update_input_keybinding.rs"]
mod update_input_keybinding;
#[path = "update/update_input_mouse.rs"]
mod update_input_mouse;
#[path = "update/update_lifecycle_create.rs"]
mod update_lifecycle_create;
#[path = "update/update_lifecycle_start.rs"]
mod update_lifecycle_start;
#[path = "update/update_lifecycle_stop.rs"]
mod update_lifecycle_stop;
#[path = "update/update_lifecycle_workspace_completion.rs"]
mod update_lifecycle_workspace_completion;
#[path = "update/update_lifecycle_workspace_refresh.rs"]
mod update_lifecycle_workspace_refresh;
#[path = "update/update_navigation_commands.rs"]
mod update_navigation_commands;
#[path = "update/update_navigation_palette.rs"]
mod update_navigation_palette;
#[path = "update/update_navigation_preview.rs"]
mod update_navigation_preview;
#[path = "update/update_polling_capture_cursor.rs"]
mod update_polling_capture_cursor;
#[path = "update/update_polling_capture_dispatch.rs"]
mod update_polling_capture_dispatch;
#[path = "update/update_polling_capture_live.rs"]
mod update_polling_capture_live;
#[path = "update/update_polling_capture_task.rs"]
mod update_polling_capture_task;
#[path = "update/update_polling_capture_workspace.rs"]
mod update_polling_capture_workspace;
#[path = "update/update_polling_state.rs"]
mod update_polling_state;
#[path = "update/update_tick.rs"]
mod update_tick;
#[path = "view/view.rs"]
mod view;
#[path = "view/view_chrome_divider.rs"]
mod view_chrome_divider;
#[path = "view/view_chrome_header.rs"]
mod view_chrome_header;
#[path = "view/view_chrome_shared.rs"]
mod view_chrome_shared;
#[path = "view/view_chrome_sidebar.rs"]
mod view_chrome_sidebar;
#[path = "view/view_layout.rs"]
mod view_layout;
#[path = "view/view_overlays_confirm.rs"]
mod view_overlays_confirm;
#[path = "view/view_overlays_create.rs"]
mod view_overlays_create;
#[path = "view/view_overlays_edit.rs"]
mod view_overlays_edit;
#[path = "view/view_overlays_help.rs"]
mod view_overlays_help;
#[path = "view/view_overlays_projects.rs"]
mod view_overlays_projects;
#[path = "view/view_overlays_settings.rs"]
mod view_overlays_settings;
#[path = "view/view_overlays_workspace_delete.rs"]
mod view_overlays_workspace_delete;
#[path = "view/view_overlays_workspace_launch.rs"]
mod view_overlays_workspace_launch;
#[path = "view/view_overlays_workspace_merge.rs"]
mod view_overlays_workspace_merge;
#[path = "view/view_overlays_workspace_stop.rs"]
mod view_overlays_workspace_stop;
#[path = "view/view_overlays_workspace_update.rs"]
mod view_overlays_workspace_update;
#[path = "view/view_preview.rs"]
mod view_preview;
#[path = "view/view_preview_content.rs"]
mod view_preview_content;
#[path = "view/view_preview_shell.rs"]
mod view_preview_shell;
#[path = "view/view_selection_interaction.rs"]
mod view_selection_interaction;
#[path = "view/view_selection_logging.rs"]
mod view_selection_logging;
#[path = "view/view_selection_mapping.rs"]
mod view_selection_mapping;
#[path = "view/view_status.rs"]
mod view_status;

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
        self.update_model(msg)
    }

    fn view(&self, frame: &mut Frame) {
        self.render_model(frame);
    }
}

#[cfg(test)]
mod tests;
