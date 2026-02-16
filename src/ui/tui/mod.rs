use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque, hash_map::DefaultHasher};
use std::fs;
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
    launch_request_for_workspace, live_preview_capture_target_for_tab, poll_interval,
    session_name_for_workspace_ref, shell_launch_request_for_workspace,
    tmux_capture_error_indicates_missing_session, workspace_can_enter_interactive,
    workspace_can_start_agent, workspace_can_stop_agent, workspace_session_for_preview_tab,
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
    BranchMode, CommandGitRunner, CommandSetupScriptRunner, CreateWorkspaceRequest,
    CreateWorkspaceResult, DeleteWorkspaceRequest, MergeWorkspaceRequest,
    UpdateWorkspaceFromBaseRequest, WorkspaceLifecycleError, create_workspace, delete_workspace,
    merge_workspace, update_workspace_from_base, workspace_lifecycle_error_message,
    write_workspace_agent_marker,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
use crate::infrastructure::config::{GroveConfig, MultiplexerKind, ProjectConfig};
use crate::infrastructure::event_log::{Event as LogEvent, EventLogger};
use crate::ui::mouse::{clamp_sidebar_ratio, ratio_from_drag, serialize_sidebar_ratio};
use crate::ui::state::{Action, AppState, PaneFocus, UiMode, reduce};

mod ansi;
#[cfg(test)]
use ansi::ansi_16_color;
use ansi::ansi_line_to_styled_line;
mod bootstrap;
#[cfg(test)]
use bootstrap::{AppDependencies, AppPaths};
use bootstrap::{
    bootstrap_data_for_projects, filter_branches, input_for_multiplexer, load_local_branches,
    project_display_name, project_paths_equal, read_workspace_launch_prompt,
};
mod terminal;
use terminal::{
    ClipboardAccess, CommandTmuxInput, SystemClipboardAccess, TmuxInput, parse_cursor_metadata,
};
mod dialogs;
use dialogs::*;
mod commands;
use commands::*;
mod msg;
use msg::*;
mod logging;
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
mod view;

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
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_hidden: bool,
    launch_skip_permissions: bool,
    sidebar_ratio_path: PathBuf,
    divider_drag_active: bool,
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
    delete_in_flight: bool,
    merge_in_flight: bool,
    update_from_base_in_flight: bool,
    create_in_flight: bool,
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
