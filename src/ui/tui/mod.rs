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
use ftui::{Cmd, Model, PackedRgba, Style};
use ftui_extras::text_effects::{ColorGradient, StyledText, TextEffect};
use serde_json::Value;

use crate::application::agent_runtime::{
    CommandExecutionMode, LivePreviewTarget, OutputDigest, SessionActivity,
    detect_status_with_session_override, evaluate_capture_change, execute_command_with,
    execute_launch_request_with_result_for_mode, execute_shell_launch_request_for_mode,
    execute_stop_workspace_with_result_for_mode, git_session_name_for_workspace,
    launch_request_for_workspace, poll_interval, session_name_for_workspace_ref,
    shell_launch_request_for_workspace, shell_session_name_for_workspace,
    tmux_capture_error_indicates_missing_session, tmux_launch_error_indicates_duplicate_session,
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
    UpdateWorkspaceFromBaseRequest, WorkspaceLifecycleError, WorkspaceSetupTemplate,
    create_workspace_with_template, delete_workspace, merge_workspace, update_workspace_from_base,
    workspace_lifecycle_error_message, write_workspace_agent_marker, write_workspace_base_marker,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
use crate::infrastructure::config::{GroveConfig, MultiplexerKind, ProjectConfig, ProjectDefaults};
use crate::infrastructure::event_log::{Event as LogEvent, EventLogger};
use crate::ui::mouse::{clamp_sidebar_ratio, ratio_from_drag};
use crate::ui::state::{Action, AppState, PaneFocus, UiMode, reduce};

mod ansi;
#[cfg(test)]
use ansi::ansi_16_color;
use ansi::ansi_line_to_styled_line;
mod bootstrap_app;
mod bootstrap_config;
mod bootstrap_discovery;
#[cfg(test)]
use bootstrap_config::{AppDependencies, AppPaths};
use bootstrap_config::{
    filter_branches, input_for_multiplexer, load_local_branches, project_display_name,
    project_paths_equal, read_workspace_launch_prompt,
};
use bootstrap_discovery::bootstrap_data_for_projects;
mod terminal;
use terminal::{
    ClipboardAccess, CommandTmuxInput, SystemClipboardAccess, TmuxInput, parse_cursor_metadata,
};
mod dialogs;
mod dialogs_create_key;
mod dialogs_create_setup;
mod dialogs_delete;
mod dialogs_edit;
mod dialogs_launch;
mod dialogs_merge;
mod dialogs_projects_add;
mod dialogs_projects_key;
mod dialogs_projects_state;
mod dialogs_settings;
mod dialogs_state_create_edit;
mod dialogs_state_lifecycle;
mod dialogs_state_project_settings;
mod dialogs_update_from_base;
use dialogs::*;
use dialogs_state_create_edit::*;
use dialogs_state_lifecycle::*;
use dialogs_state_project_settings::*;
mod commands;
mod commands_hints;
mod commands_palette;
use commands::*;
mod msg;
use msg::*;
mod logging_frame;
mod logging_input;
mod logging_state;
mod selection;
use selection::{TextSelectionPoint, TextSelectionState};
mod runner;
pub use runner::{run, run_with_debug_record, run_with_event_log};
mod shared;
use shared::*;
mod text;
use text::{
    ansi_line_to_plain_text, chrome_bar_line, keybind_hint_spans, line_visual_width,
    pad_or_truncate_to_display_width, truncate_for_log, truncate_to_display_width,
    visual_grapheme_at, visual_substring,
};
mod update;
mod update_core;
mod update_input_interactive;
mod update_input_interactive_clipboard;
mod update_input_interactive_send;
mod update_input_key_events;
mod update_input_keybinding;
mod update_input_keys;
mod update_input_mouse;
mod update_lifecycle_create;
mod update_lifecycle_start;
mod update_lifecycle_stop;
mod update_lifecycle_workspace_completion;
mod update_lifecycle_workspace_refresh;
mod update_navigation_commands;
mod update_navigation_palette;
mod update_navigation_preview;
mod update_navigation_summary;
mod update_polling_capture_cursor;
mod update_polling_capture_dispatch;
mod update_polling_capture_live;
mod update_polling_capture_scroll;
mod update_polling_capture_task;
mod update_polling_capture_workspace;
mod update_polling_state;
mod update_tick;
mod view;
mod view_chrome_divider;
mod view_chrome_header;
mod view_chrome_shared;
mod view_chrome_sidebar;
mod view_layout;
mod view_overlays_create;
mod view_overlays_edit;
mod view_overlays_help;
mod view_overlays_projects;
mod view_overlays_settings;
mod view_overlays_workspace_delete;
mod view_overlays_workspace_launch;
mod view_overlays_workspace_merge;
mod view_overlays_workspace_update;
mod view_preview;
mod view_preview_content;
mod view_preview_shell;
mod view_selection_interaction;
mod view_selection_logging;
mod view_selection_mapping;
mod view_status;

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedDeleteWorkspace {
    request: DeleteWorkspaceRequest,
    workspace_name: String,
    workspace_path: PathBuf,
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
    launch_dialog: Option<LaunchDialogState>,
    delete_dialog: Option<DeleteDialogState>,
    merge_dialog: Option<MergeDialogState>,
    update_from_base_dialog: Option<UpdateFromBaseDialogState>,
    create_dialog: Option<CreateDialogState>,
    edit_dialog: Option<EditDialogState>,
    project_dialog: Option<ProjectDialogState>,
    settings_dialog: Option<SettingsDialogState>,
    keybind_help_open: bool,
    command_palette: CommandPalette,
    create_branch_all: Vec<String>,
    create_branch_filtered: Vec<String>,
    create_branch_index: usize,
    multiplexer: MultiplexerKind,
    tmux_input: Box<dyn TmuxInput>,
    config_path: PathBuf,
    clipboard: Box<dyn ClipboardAccess>,
    last_tmux_error: Option<String>,
    output_changing: bool,
    agent_output_changing: bool,
    agent_activity_frames: VecDeque<bool>,
    workspace_status_digests: HashMap<String, OutputDigest>,
    workspace_output_changing: HashMap<String, bool>,
    lazygit_ready_sessions: HashSet<String>,
    lazygit_failed_sessions: HashSet<String>,
    lazygit_launch_in_flight: HashSet<String>,
    shell_ready_sessions: HashSet<String>,
    shell_failed_sessions: HashSet<String>,
    shell_launch_in_flight: HashSet<String>,
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
    next_tick_due_at: Option<Instant>,
    next_tick_interval_ms: Option<u64>,
    next_poll_due_at: Option<Instant>,
    preview_poll_in_flight: bool,
    preview_poll_requested: bool,
    next_visual_due_at: Option<Instant>,
    interactive_poll_due_at: Option<Instant>,
    fast_animation_frame: usize,
    poll_generation: u64,
    debug_record_start_ts: Option<u64>,
    frame_render_seq: RefCell<u64>,
    input_seq_counter: u64,
    pending_interactive_inputs: VecDeque<PendingInteractiveInput>,
    pending_interactive_sends: VecDeque<QueuedInteractiveSend>,
    interactive_send_in_flight: bool,
    pending_resize_verification: Option<PendingResizeVerification>,
    refresh_in_flight: bool,
    project_delete_in_flight: bool,
    delete_in_flight: bool,
    delete_in_flight_workspace: Option<PathBuf>,
    pending_delete_workspaces: VecDeque<QueuedDeleteWorkspace>,
    delete_requested_workspaces: HashSet<PathBuf>,
    merge_in_flight: bool,
    update_from_base_in_flight: bool,
    create_in_flight: bool,
    pending_auto_start_workspace_path: Option<PathBuf>,
    pending_auto_launch_shell_workspace_path: Option<PathBuf>,
    start_in_flight: bool,
    stop_in_flight: bool,
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
