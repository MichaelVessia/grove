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
use ftui::render::budget::FrameBudgetConfig;
use ftui::render::frame::{Frame, HitGrid, HitId, HitRegion as FrameHitRegion};
use ftui::runtime::WidgetRefreshConfig;
use ftui::text::{
    Line as FtLine, Span as FtSpan, Text as FtText, display_width as text_display_width,
    graphemes as text_graphemes,
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
use ftui::{App, Cmd, Model, PackedRgba, ScreenMode, Style};
use ftui_extras::text_effects::{ColorGradient, StyledText, TextEffect};
use serde_json::Value;

use crate::adapters::{BootstrapData, DiscoveryState};
use crate::agent_runtime::{
    LaunchRequest, OutputDigest, SessionActivity, ShellLaunchRequest, build_launch_plan,
    build_shell_launch_plan, detect_status_with_session_override, evaluate_capture_change,
    poll_interval, session_name_for_workspace_in_project, stop_plan, zellij_config_path,
};
use crate::config::{GroveConfig, MultiplexerKind, ProjectConfig};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::event_log::{Event as LogEvent, EventLogger, FileEventLogger, NullEventLogger};
#[cfg(test)]
use crate::interactive::render_cursor_overlay;
use crate::interactive::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload,
    multiplexer_send_input_command, render_cursor_overlay_ansi,
};
use crate::mouse::{clamp_sidebar_ratio, ratio_from_drag, serialize_sidebar_ratio};
use crate::preview::PreviewState;
use crate::state::{Action, AppState, PaneFocus, UiMode, reduce};
use crate::workspace_lifecycle::{
    BranchMode, CommandGitRunner, CommandSetupScriptRunner, CreateWorkspaceRequest,
    CreateWorkspaceResult, WorkspaceLifecycleError, create_workspace, write_workspace_agent_marker,
};

mod ansi;
#[cfg(test)]
use ansi::ansi_16_color;
use ansi::ansi_line_to_styled_line;
mod bootstrap;
use bootstrap::{
    bootstrap_data_for_projects, default_sidebar_ratio_path, filter_branches,
    input_for_multiplexer, load_local_branches, load_runtime_config, load_sidebar_ratio,
    project_display_name, project_paths_equal, read_workspace_launch_prompt,
};
mod terminal;
use terminal::{
    ClipboardAccess, CommandTmuxInput, CommandZellijInput, SystemClipboardAccess, TmuxInput,
    parse_cursor_metadata,
};
mod dialogs;
use dialogs::*;
mod msg;
use msg::*;
mod update;
mod view;

const DEFAULT_SIDEBAR_WIDTH_PCT: u16 = 33;
const SIDEBAR_RATIO_FILENAME: &str = ".grove-sidebar-width";
const WORKSPACE_LAUNCH_PROMPT_FILENAME: &str = ".grove-prompt";
const HEADER_HEIGHT: u16 = 1;
const STATUS_HEIGHT: u16 = 1;
const DIVIDER_WIDTH: u16 = 1;
const WORKSPACE_ITEM_HEIGHT: u16 = 1;
const PREVIEW_METADATA_ROWS: u16 = 2;
const TICK_EARLY_TOLERANCE_MS: u64 = 5;
const HIT_ID_HEADER: u32 = 1;
const HIT_ID_WORKSPACE_LIST: u32 = 2;
const HIT_ID_PREVIEW: u32 = 3;
const HIT_ID_DIVIDER: u32 = 4;
const HIT_ID_STATUS: u32 = 5;
const HIT_ID_WORKSPACE_ROW: u32 = 6;
const HIT_ID_CREATE_DIALOG: u32 = 7;
const HIT_ID_LAUNCH_DIALOG: u32 = 8;
const HIT_ID_DELETE_DIALOG: u32 = 9;
const HIT_ID_KEYBIND_HELP_DIALOG: u32 = 10;
const HIT_ID_SETTINGS_DIALOG: u32 = 11;
const HIT_ID_PROJECT_DIALOG: u32 = 12;
const HIT_ID_PROJECT_ADD_DIALOG: u32 = 13;
const HIT_ID_EDIT_DIALOG: u32 = 14;
const PALETTE_CMD_TOGGLE_FOCUS: &str = "palette:toggle_focus";
const PALETTE_CMD_OPEN_PREVIEW: &str = "palette:open_preview";
const PALETTE_CMD_ENTER_INTERACTIVE: &str = "palette:enter_interactive";
const PALETTE_CMD_FOCUS_LIST: &str = "palette:focus_list";
const PALETTE_CMD_MOVE_SELECTION_UP: &str = "palette:move_selection_up";
const PALETTE_CMD_MOVE_SELECTION_DOWN: &str = "palette:move_selection_down";
const PALETTE_CMD_SCROLL_UP: &str = "palette:scroll_up";
const PALETTE_CMD_SCROLL_DOWN: &str = "palette:scroll_down";
const PALETTE_CMD_PAGE_UP: &str = "palette:page_up";
const PALETTE_CMD_PAGE_DOWN: &str = "palette:page_down";
const PALETTE_CMD_SCROLL_BOTTOM: &str = "palette:scroll_bottom";
const PALETTE_CMD_NEW_WORKSPACE: &str = "palette:new_workspace";
const PALETTE_CMD_EDIT_WORKSPACE: &str = "palette:edit_workspace";
const PALETTE_CMD_START_AGENT: &str = "palette:start_agent";
const PALETTE_CMD_STOP_AGENT: &str = "palette:stop_agent";
const PALETTE_CMD_DELETE_WORKSPACE: &str = "palette:delete_workspace";
const PALETTE_CMD_OPEN_SETTINGS: &str = "palette:open_settings";
const PALETTE_CMD_TOGGLE_UNSAFE: &str = "palette:toggle_unsafe";
const PALETTE_CMD_OPEN_HELP: &str = "palette:open_help";
const PALETTE_CMD_QUIT: &str = "palette:quit";
const MAX_PENDING_INPUT_TRACES: usize = 256;
const INTERACTIVE_KEYSTROKE_DEBOUNCE_MS: u64 = 20;
const FAST_ANIMATION_INTERVAL_MS: u64 = 100;
const TOAST_TICK_INTERVAL_MS: u64 = 100;
const LAZYGIT_COMMAND: &str = "lazygit";
const AGENT_ACTIVITY_WINDOW_FRAMES: usize = 6;
const LOCAL_TYPING_SUPPRESS_MS: u64 = 400;

#[derive(Debug, Clone, Copy)]
struct UiTheme {
    base: PackedRgba,
    mantle: PackedRgba,
    crust: PackedRgba,
    surface0: PackedRgba,
    surface1: PackedRgba,
    overlay0: PackedRgba,
    text: PackedRgba,
    subtext0: PackedRgba,
    blue: PackedRgba,
    lavender: PackedRgba,
    yellow: PackedRgba,
    red: PackedRgba,
    peach: PackedRgba,
    mauve: PackedRgba,
    teal: PackedRgba,
}

fn ui_theme() -> UiTheme {
    UiTheme {
        base: PackedRgba::rgb(30, 30, 46),
        mantle: PackedRgba::rgb(24, 24, 37),
        crust: PackedRgba::rgb(17, 17, 27),
        surface0: PackedRgba::rgb(49, 50, 68),
        surface1: PackedRgba::rgb(69, 71, 90),
        overlay0: PackedRgba::rgb(108, 112, 134),
        text: PackedRgba::rgb(205, 214, 244),
        subtext0: PackedRgba::rgb(166, 173, 200),
        blue: PackedRgba::rgb(137, 180, 250),
        lavender: PackedRgba::rgb(180, 190, 254),
        yellow: PackedRgba::rgb(249, 226, 175),
        red: PackedRgba::rgb(243, 139, 168),
        peach: PackedRgba::rgb(250, 179, 135),
        mauve: PackedRgba::rgb(203, 166, 247),
        teal: PackedRgba::rgb(148, 226, 213),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitRegion {
    WorkspaceList,
    Preview,
    Divider,
    StatusLine,
    Header,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PreviewTab {
    #[default]
    Agent,
    Git,
}

impl PreviewTab {
    const fn label(self) -> &'static str {
        match self {
            Self::Agent => "Agent",
            Self::Git => "Git",
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::Agent => Self::Git,
            Self::Git => Self::Agent,
        }
    }

    const fn previous(self) -> Self {
        match self {
            Self::Agent => Self::Git,
            Self::Git => Self::Agent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ViewLayout {
    header: Rect,
    sidebar: Rect,
    divider: Rect,
    preview: Rect,
    status: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorMetadata {
    cursor_visible: bool,
    cursor_col: u16,
    cursor_row: u16,
    pane_width: u16,
    pane_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextSelectionPoint {
    line: usize,
    col: usize,
}

impl TextSelectionPoint {
    fn before(self, other: Self) -> bool {
        self.line < other.line || (self.line == other.line && self.col < other.col)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TextSelectionState {
    active: bool,
    start: Option<TextSelectionPoint>,
    end: Option<TextSelectionPoint>,
    anchor: Option<TextSelectionPoint>,
}

impl TextSelectionState {
    fn clear(&mut self) {
        self.active = false;
        self.start = None;
        self.end = None;
        self.anchor = None;
    }

    fn has_selection(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }

    fn prepare_drag(&mut self, point: TextSelectionPoint) {
        self.active = false;
        self.start = None;
        self.end = None;
        self.anchor = Some(point);
    }

    fn handle_drag(&mut self, point: TextSelectionPoint) {
        let Some(anchor) = self.anchor else {
            return;
        };
        if self.start.is_none() {
            self.start = Some(anchor);
            self.end = Some(anchor);
        }

        self.active = true;
        if point.before(anchor) {
            self.start = Some(point);
            self.end = Some(anchor);
        } else {
            self.start = Some(anchor);
            self.end = Some(point);
        }
    }

    fn finish_drag(&mut self) {
        if self.start.is_none() {
            self.clear();
            return;
        }

        self.active = false;
        self.anchor = None;
    }

    fn bounds(&self) -> Option<(TextSelectionPoint, TextSelectionPoint)> {
        Some((self.start?, self.end?))
    }

    fn line_selection_cols(&self, line_idx: usize) -> Option<(usize, Option<usize>)> {
        let (start, end) = self.bounds()?;
        if line_idx < start.line || line_idx > end.line {
            return None;
        }

        if start.line == end.line {
            return Some((start.col, Some(end.col)));
        }
        if line_idx == start.line {
            return Some((start.col, None));
        }
        if line_idx == end.line {
            return Some((0, Some(end.col)));
        }

        Some((0, None))
    }
}

fn line_visual_width(line: &str) -> usize {
    text_display_width(line)
}

fn visual_substring(line: &str, start_col: usize, end_col_inclusive: Option<usize>) -> String {
    let mut out = String::new();
    let end_col_exclusive = end_col_inclusive.map(|end| end.saturating_add(1));
    let mut visual_col = 0usize;

    for grapheme in text_graphemes(line) {
        if end_col_exclusive.is_some_and(|end| visual_col >= end) {
            break;
        }

        let width = line_visual_width(grapheme);
        let next_col = visual_col.saturating_add(width);
        let intersects = if width == 0 {
            visual_col >= start_col
        } else {
            next_col > start_col
        };

        if intersects {
            out.push_str(grapheme);
        }

        visual_col = next_col;
    }

    out
}

fn visual_grapheme_at(line: &str, target_col: usize) -> Option<(String, usize, usize)> {
    let mut visual_col = 0usize;
    for grapheme in text_graphemes(line) {
        let width = line_visual_width(grapheme);
        let start_col = visual_col;
        let end_col = if width == 0 {
            start_col
        } else {
            start_col.saturating_add(width.saturating_sub(1))
        };

        if (width == 0 && target_col == start_col) || (width > 0 && target_col <= end_col) {
            return Some((grapheme.to_string(), start_col, end_col));
        }

        visual_col = visual_col.saturating_add(width);
    }

    None
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn truncate_to_display_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text_display_width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let target_width = max_width.saturating_sub(1);
    for grapheme in text_graphemes(value) {
        let grapheme_width = line_visual_width(grapheme);
        if width.saturating_add(grapheme_width) > target_width {
            break;
        }
        out.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    out.push('…');
    out
}

fn pad_or_truncate_to_display_width(value: &str, width: usize) -> String {
    let mut out = truncate_to_display_width(value, width);
    let used = text_display_width(out.as_str());
    if used < width {
        out.push_str(&" ".repeat(width.saturating_sub(used)));
    }
    out
}

fn clip_to_display_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text_display_width(value) <= max_width {
        return value.to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    for grapheme in text_graphemes(value) {
        let grapheme_width = line_visual_width(grapheme);
        if width.saturating_add(grapheme_width) > max_width {
            break;
        }
        out.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    out
}

fn spans_display_width(spans: &[FtSpan<'_>]) -> usize {
    spans
        .iter()
        .map(|span| text_display_width(span.content.as_ref()))
        .sum()
}

fn truncate_spans_to_width(spans: &[FtSpan<'_>], max_width: usize) -> Vec<FtSpan<'static>> {
    if max_width == 0 {
        return Vec::new();
    }

    let mut rendered: Vec<FtSpan<'static>> = Vec::new();
    let mut used = 0usize;
    for span in spans {
        if used >= max_width {
            break;
        }

        let remaining = max_width.saturating_sub(used);
        let rendered_text = clip_to_display_width(span.content.as_ref(), remaining);
        if rendered_text.is_empty() {
            continue;
        }

        let rendered_span = match span.style {
            Some(style) => FtSpan::styled(rendered_text, style),
            None => FtSpan::raw(rendered_text),
        };
        used = used.saturating_add(text_display_width(rendered_span.content.as_ref()));
        rendered.push(rendered_span);
    }

    rendered
}

fn chrome_bar_line(
    width: usize,
    base_style: Style,
    left: Vec<FtSpan<'static>>,
    center: Vec<FtSpan<'static>>,
    right: Vec<FtSpan<'static>>,
) -> FtLine {
    if width == 0 {
        return FtLine::raw("");
    }

    let right = truncate_spans_to_width(&right, width);
    let right_width = spans_display_width(&right);
    let right_start = width.saturating_sub(right_width);

    let center = truncate_spans_to_width(&center, width);
    let center_width = spans_display_width(&center);
    let center_start = width.saturating_sub(center_width) / 2;
    let center_can_render =
        center_width > 0 && center_start.saturating_add(center_width) <= right_start;

    let left_max_width = if center_can_render {
        center_start
    } else {
        right_start
    };
    let left = truncate_spans_to_width(&left, left_max_width);
    let left_width = spans_display_width(&left);

    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    spans.extend(left);
    let mut cursor = left_width;

    if center_can_render {
        if center_start > cursor {
            spans.push(FtSpan::styled(
                " ".repeat(center_start.saturating_sub(cursor)),
                base_style,
            ));
        }
        spans.extend(center);
        cursor = center_start.saturating_add(center_width);
    }

    if right_start > cursor {
        spans.push(FtSpan::styled(
            " ".repeat(right_start.saturating_sub(cursor)),
            base_style,
        ));
    }
    spans.extend(right);
    cursor = right_start.saturating_add(right_width);

    if width > cursor {
        spans.push(FtSpan::styled(
            " ".repeat(width.saturating_sub(cursor)),
            base_style,
        ));
    }

    FtLine::from_spans(spans)
}

fn keybind_hint_spans(
    hints: &str,
    base_style: Style,
    key_style: Style,
    sep_style: Style,
) -> Vec<FtSpan<'static>> {
    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    for (chunk_index, chunk) in hints.split(", ").enumerate() {
        if chunk_index > 0 {
            spans.push(FtSpan::styled(", ", sep_style));
        }

        if let Some(split_index) = chunk.rfind(' ') {
            let key = &chunk[..split_index];
            let action = &chunk[split_index..];
            if !key.is_empty() {
                spans.push(FtSpan::styled(key.to_string(), key_style));
            }
            if !action.is_empty() {
                spans.push(FtSpan::styled(action.to_string(), base_style));
            }
            continue;
        }

        spans.push(FtSpan::styled(chunk.to_string(), key_style));
    }

    spans
}

fn ansi_line_to_plain_text(line: &str) -> String {
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            plain.push(character);
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                for value in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        break;
                    }
                }
            }
            ']' => {
                while let Some(value) = chars.next() {
                    if value == '\u{7}' {
                        break;
                    }
                    if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                        break;
                    }
                }
            }
            'P' | 'X' | '^' | '_' => {
                while let Some(value) = chars.next() {
                    if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                        break;
                    }
                }
            }
            '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' => {
                let _ = chars.next();
            }
            _ => {}
        }
    }

    plain
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreviewContentViewport {
    output_x: u16,
    output_y: u16,
    visible_start: usize,
    visible_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransitionSnapshot {
    selected_index: usize,
    selected_workspace: Option<String>,
    focus: PaneFocus,
    mode: UiMode,
    interactive_session: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InputTraceContext {
    seq: u64,
    received_at: Instant,
}

#[derive(Debug, Clone)]
struct PendingInteractiveInput {
    seq: u64,
    session: String,
    received_at: Instant,
    forwarded_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingResizeVerification {
    session: String,
    expected_width: u16,
    expected_height: u16,
    retried: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedInteractiveSend {
    command: Vec<String>,
    target_session: String,
    action_kind: String,
    trace_context: Option<InputTraceContext>,
    literal_chars: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InteractiveSendCompletion {
    send: QueuedInteractiveSend,
    tmux_send_ms: u64,
    error: Option<String>,
}

#[derive(Debug)]
struct AppPaths {
    sidebar_ratio_path: PathBuf,
    config_path: PathBuf,
}

impl AppPaths {
    fn new(sidebar_ratio_path: PathBuf, config_path: PathBuf) -> Self {
        Self {
            sidebar_ratio_path,
            config_path,
        }
    }
}

struct AppDependencies {
    tmux_input: Box<dyn TmuxInput>,
    clipboard: Box<dyn ClipboardAccess>,
    paths: AppPaths,
    multiplexer: MultiplexerKind,
    event_log: Box<dyn EventLogger>,
    debug_record_start_ts: Option<u64>,
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
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
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
    create_in_flight: bool,
    start_in_flight: bool,
    stop_in_flight: bool,
    deferred_cmds: Vec<Cmd<Msg>>,
}

impl GroveApp {
    fn new_with_event_logger(event_log: Box<dyn EventLogger>) -> Self {
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

    fn new_with_debug_recorder(event_log: Box<dyn EventLogger>, app_start_ts: u64) -> Self {
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
    fn from_parts(
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

    fn from_parts_with_clipboard_and_projects(
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
            viewport_width: 120,
            viewport_height: 40,
            sidebar_width_pct,
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
            create_in_flight: false,
            start_in_flight: false,
            stop_in_flight: false,
            deferred_cmds: Vec::new(),
        };
        app.refresh_preview_summary();
        app
    }

    fn mode_label(&self) -> &'static str {
        if self.interactive.is_some() {
            return "Interactive";
        }

        match self.state.mode {
            UiMode::List => "List",
            UiMode::Preview => "Preview",
        }
    }

    fn focus_label(&self) -> &'static str {
        match self.state.focus {
            PaneFocus::WorkspaceList => "WorkspaceList",
            PaneFocus::Preview => "Preview",
        }
    }

    fn focus_name(focus: PaneFocus) -> &'static str {
        match focus {
            PaneFocus::WorkspaceList => "workspace_list",
            PaneFocus::Preview => "preview",
        }
    }

    fn mode_name(mode: UiMode) -> &'static str {
        match mode {
            UiMode::List => "list",
            UiMode::Preview => "preview",
        }
    }

    fn hit_region_name(region: HitRegion) -> &'static str {
        match region {
            HitRegion::WorkspaceList => "workspace_list",
            HitRegion::Preview => "preview",
            HitRegion::Divider => "divider",
            HitRegion::StatusLine => "status_line",
            HitRegion::Header => "header",
            HitRegion::Outside => "outside",
        }
    }

    fn selected_workspace_name(&self) -> Option<String> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
    }

    fn selected_workspace_path(&self) -> Option<PathBuf> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.path.clone())
    }

    fn workspace_session_name(workspace: &Workspace) -> String {
        session_name_for_workspace_in_project(workspace.project_name.as_deref(), &workspace.name)
    }

    fn capture_transition_snapshot(&self) -> TransitionSnapshot {
        TransitionSnapshot {
            selected_index: self.state.selected_index,
            selected_workspace: self.selected_workspace_name(),
            focus: self.state.focus,
            mode: self.state.mode,
            interactive_session: self.interactive_target_session(),
        }
    }

    fn emit_transition_events(&mut self, before: &TransitionSnapshot) {
        let after = self.capture_transition_snapshot();
        if after.selected_index != before.selected_index {
            let selection_index = u64::try_from(after.selected_index).unwrap_or(u64::MAX);
            let workspace_value = after
                .selected_workspace
                .clone()
                .map(Value::from)
                .unwrap_or(Value::Null);
            self.event_log.log(
                LogEvent::new("state_change", "selection_changed")
                    .with_data("index", Value::from(selection_index))
                    .with_data("workspace", workspace_value),
            );
        }
        if after.focus != before.focus {
            self.event_log.log(
                LogEvent::new("state_change", "focus_changed")
                    .with_data("focus", Value::from(Self::focus_name(after.focus))),
            );
        }
        if after.mode != before.mode {
            self.event_log.log(
                LogEvent::new("mode_change", "mode_changed")
                    .with_data("mode", Value::from(Self::mode_name(after.mode))),
            );
        }
        match (&before.interactive_session, &after.interactive_session) {
            (None, Some(session)) => {
                self.event_log.log(
                    LogEvent::new("mode_change", "interactive_entered")
                        .with_data("session", Value::from(session.clone())),
                );
            }
            (Some(session), None) => {
                self.event_log.log(
                    LogEvent::new("mode_change", "interactive_exited")
                        .with_data("session", Value::from(session.clone())),
                );
                self.interactive_poll_due_at = None;
                self.pending_resize_verification = None;
                let pending_before = self.pending_interactive_inputs.len();
                self.clear_pending_inputs_for_session(session);
                let pending_after = self.pending_interactive_inputs.len();
                self.clear_pending_sends_for_session(session);
                if pending_before != pending_after {
                    self.event_log.log(
                        LogEvent::new("input", "pending_inputs_cleared")
                            .with_data("session", Value::from(session.clone()))
                            .with_data(
                                "cleared",
                                Value::from(
                                    u64::try_from(pending_before.saturating_sub(pending_after))
                                        .unwrap_or(u64::MAX),
                                ),
                            ),
                    );
                }
            }
            _ => {}
        }
    }

    fn log_dialog_event_with_fields(
        &self,
        kind: &str,
        action: &str,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) {
        let event = LogEvent::new("dialog", action)
            .with_data("kind", Value::from(kind.to_string()))
            .with_data_fields(fields);
        self.event_log.log(event);
    }

    fn log_dialog_event(&self, kind: &str, action: &str) {
        self.log_dialog_event_with_fields(kind, action, std::iter::empty());
    }

    fn log_tmux_error(&self, message: String) {
        self.event_log
            .log(LogEvent::new("error", "tmux_error").with_data("message", Value::from(message)));
    }

    fn execute_tmux_command(&mut self, command: &[String]) -> std::io::Result<()> {
        let started_at = Instant::now();
        self.event_log.log(
            LogEvent::new("tmux_cmd", "execute")
                .with_data("command", Value::from(command.join(" "))),
        );
        let result = self.tmux_input.execute(command);
        let duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(started_at));
        let mut completion_event = LogEvent::new("tmux_cmd", "completed")
            .with_data("command", Value::from(command.join(" ")))
            .with_data("duration_ms", Value::from(duration_ms))
            .with_data("ok", Value::from(result.is_ok()));
        if let Err(error) = &result {
            completion_event = completion_event.with_data("error", Value::from(error.to_string()));
            self.log_tmux_error(error.to_string());
        }
        self.event_log.log(completion_event);
        result
    }

    fn show_toast(&mut self, text: impl Into<String>, is_error: bool) {
        let message = text.into();
        self.event_log.log(
            LogEvent::new("toast", "toast_shown")
                .with_data("text", Value::from(message.clone()))
                .with_data("is_error", Value::from(is_error)),
        );

        let toast = if is_error {
            Toast::new(message)
                .title("Error")
                .icon(ToastIcon::Error)
                .style_variant(ToastStyle::Error)
                .duration(Duration::from_secs(3))
        } else {
            Toast::new(message)
                .icon(ToastIcon::Success)
                .style_variant(ToastStyle::Success)
                .duration(Duration::from_secs(3))
        };
        let priority = if is_error {
            NotificationPriority::High
        } else {
            NotificationPriority::Normal
        };
        let _ = self.notifications.push(toast, priority);
        let _ = self.notifications.tick(Duration::ZERO);
    }

    fn duration_millis(duration: Duration) -> u64 {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    }

    fn msg_kind(msg: &Msg) -> &'static str {
        match msg {
            Msg::Tick => "tick",
            Msg::Key(_) => "key",
            Msg::Mouse(_) => "mouse",
            Msg::Paste(_) => "paste",
            Msg::Resize { .. } => "resize",
            Msg::PreviewPollCompleted(_) => "preview_poll_completed",
            Msg::RefreshWorkspacesCompleted(_) => "refresh_workspaces_completed",
            Msg::DeleteWorkspaceCompleted(_) => "delete_workspace_completed",
            Msg::CreateWorkspaceCompleted(_) => "create_workspace_completed",
            Msg::StartAgentCompleted(_) => "start_agent_completed",
            Msg::StopAgentCompleted(_) => "stop_agent_completed",
            Msg::InteractiveSendCompleted(_) => "interactive_send_completed",
            Msg::Noop => "noop",
        }
    }

    fn queue_cmd(&mut self, cmd: Cmd<Msg>) {
        if matches!(cmd, Cmd::None) {
            return;
        }

        self.deferred_cmds.push(cmd);
    }

    fn merge_deferred_cmds(&mut self, cmd: Cmd<Msg>) -> Cmd<Msg> {
        let deferred_cmds = std::mem::take(&mut self.deferred_cmds);
        if deferred_cmds.is_empty() {
            return cmd;
        }

        if matches!(cmd, Cmd::Quit) {
            return Cmd::Quit;
        }

        if matches!(cmd, Cmd::None) {
            return Cmd::batch(deferred_cmds);
        }

        let mut merged = Vec::with_capacity(deferred_cmds.len().saturating_add(1));
        merged.push(cmd);
        merged.extend(deferred_cmds);
        Cmd::batch(merged)
    }

    fn next_input_seq(&mut self) -> u64 {
        let seq = self.input_seq_counter;
        self.input_seq_counter = self.input_seq_counter.saturating_add(1);
        seq
    }

    fn log_input_event_with_fields(
        &self,
        kind: &str,
        seq: u64,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) {
        self.event_log.log(
            LogEvent::new("input", kind)
                .with_data("seq", Value::from(seq))
                .with_data_fields(fields),
        );
    }

    fn interactive_action_kind(action: &InteractiveAction) -> &'static str {
        match action {
            InteractiveAction::SendNamed(_) => "send_named",
            InteractiveAction::SendLiteral(_) => "send_literal",
            InteractiveAction::ExitInteractive => "exit_interactive",
            InteractiveAction::CopySelection => "copy_selection",
            InteractiveAction::PasteClipboard => "paste_clipboard",
            InteractiveAction::Noop => "noop",
        }
    }

    fn interactive_key_kind(key: &InteractiveKey) -> &'static str {
        match key {
            InteractiveKey::Enter => "enter",
            InteractiveKey::Tab => "tab",
            InteractiveKey::Backspace => "backspace",
            InteractiveKey::Delete => "delete",
            InteractiveKey::Up => "up",
            InteractiveKey::Down => "down",
            InteractiveKey::Left => "left",
            InteractiveKey::Right => "right",
            InteractiveKey::Home => "home",
            InteractiveKey::End => "end",
            InteractiveKey::PageUp => "page_up",
            InteractiveKey::PageDown => "page_down",
            InteractiveKey::Escape => "escape",
            InteractiveKey::CtrlBackslash => "ctrl_backslash",
            InteractiveKey::Ctrl(_) => "ctrl",
            InteractiveKey::Function(_) => "function",
            InteractiveKey::Char(_) => "char",
            InteractiveKey::AltC => "alt_c",
            InteractiveKey::AltV => "alt_v",
        }
    }

    fn track_pending_interactive_input(
        &mut self,
        trace_context: InputTraceContext,
        target_session: &str,
        forwarded_at: Instant,
    ) {
        self.pending_interactive_inputs
            .push_back(PendingInteractiveInput {
                seq: trace_context.seq,
                session: target_session.to_string(),
                received_at: trace_context.received_at,
                forwarded_at,
            });

        if self.pending_interactive_inputs.len() <= MAX_PENDING_INPUT_TRACES {
            return;
        }

        if let Some(dropped) = self.pending_interactive_inputs.pop_front() {
            self.log_input_event_with_fields(
                "pending_input_dropped",
                dropped.seq,
                vec![
                    ("session".to_string(), Value::from(dropped.session)),
                    (
                        "queue_depth".to_string(),
                        Value::from(
                            u64::try_from(self.pending_interactive_inputs.len())
                                .unwrap_or(u64::MAX),
                        ),
                    ),
                ],
            );
        }
    }

    fn clear_pending_inputs_for_session(&mut self, target_session: &str) {
        self.pending_interactive_inputs
            .retain(|input| input.session != target_session);
    }

    fn clear_pending_sends_for_session(&mut self, target_session: &str) {
        self.pending_interactive_sends
            .retain(|send| send.target_session != target_session);
    }

    fn drain_pending_inputs_for_session(
        &mut self,
        target_session: &str,
    ) -> Vec<PendingInteractiveInput> {
        let mut retained = VecDeque::new();
        let mut drained = Vec::new();

        while let Some(input) = self.pending_interactive_inputs.pop_front() {
            if input.session == target_session {
                drained.push(input);
            } else {
                retained.push_back(input);
            }
        }

        self.pending_interactive_inputs = retained;
        drained
    }

    fn pending_input_depth(&self) -> u64 {
        u64::try_from(self.pending_interactive_inputs.len()).unwrap_or(u64::MAX)
    }

    fn oldest_pending_input_age_ms(&self, now: Instant) -> u64 {
        self.pending_interactive_inputs
            .front()
            .map(|trace| Self::duration_millis(now.saturating_duration_since(trace.received_at)))
            .unwrap_or(0)
    }

    fn schedule_interactive_debounced_poll(&mut self, now: Instant) {
        if self.interactive.is_none() {
            return;
        }

        self.interactive_poll_due_at =
            Some(now + Duration::from_millis(INTERACTIVE_KEYSTROKE_DEBOUNCE_MS));
        let next_generation = self.poll_generation.saturating_add(1);
        self.event_log.log(
            LogEvent::new("tick", "interactive_debounce_scheduled")
                .with_data("generation", Value::from(next_generation))
                .with_data("due_in_ms", Value::from(INTERACTIVE_KEYSTROKE_DEBOUNCE_MS))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
    }

    fn frame_lines_hash(lines: &[String]) -> u64 {
        let mut hasher = DefaultHasher::new();
        lines.hash(&mut hasher);
        hasher.finish()
    }

    fn frame_buffer_lines(frame: &mut Frame) -> Vec<String> {
        let height = frame.buffer.height();
        let mut lines = Vec::with_capacity(usize::from(height));
        for y in 0..height {
            let mut row = String::with_capacity(usize::from(frame.buffer.width()));
            for x in 0..frame.buffer.width() {
                let Some(cell) = frame.buffer.get(x, y).copied() else {
                    continue;
                };
                if cell.is_continuation() {
                    continue;
                }
                if let Some(value) = cell.content.as_char() {
                    row.push(value);
                    continue;
                }
                if let Some(grapheme_id) = cell.content.grapheme_id()
                    && let Some(grapheme) = frame.pool.get(grapheme_id)
                {
                    row.push_str(grapheme);
                    continue;
                }
                row.push(' ');
            }
            lines.push(row.trim_end_matches(' ').to_string());
        }

        lines
    }

    fn log_frame_render(&self, frame: &mut Frame) {
        let Some(app_start_ts) = self.debug_record_start_ts else {
            return;
        };

        let lines = Self::frame_buffer_lines(frame);
        let frame_hash = Self::frame_lines_hash(&lines);
        let non_empty_line_count = lines.iter().filter(|line| !line.is_empty()).count();
        let mut seq = self.frame_render_seq.borrow_mut();
        *seq = seq.saturating_add(1);
        let seq_value = *seq;

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default();
        let interactive_session = self
            .interactive
            .as_ref()
            .map(|state| state.target_session.clone())
            .unwrap_or_default();
        let pending_input_depth = self.pending_input_depth();
        let oldest_pending_input_seq = self
            .pending_interactive_inputs
            .front()
            .map(|trace| trace.seq)
            .unwrap_or(0);
        let oldest_pending_input_age_ms = self.oldest_pending_input_age_ms(Instant::now());

        let mut frame_event = LogEvent::new("frame", "rendered")
            .with_data("seq", Value::from(seq_value))
            .with_data("app_start_ts", Value::from(app_start_ts))
            .with_data("width", Value::from(frame.buffer.width()))
            .with_data("height", Value::from(frame.buffer.height()))
            .with_data(
                "line_count",
                Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
            )
            .with_data(
                "non_empty_line_count",
                Value::from(u64::try_from(non_empty_line_count).unwrap_or(u64::MAX)),
            )
            .with_data("frame_hash", Value::from(frame_hash))
            .with_data("degradation", Value::from(frame.degradation.as_str()))
            .with_data("mode", Value::from(self.mode_label()))
            .with_data("focus", Value::from(self.focus_label()))
            .with_data("selected_workspace", Value::from(selected_workspace))
            .with_data("interactive_session", Value::from(interactive_session))
            .with_data("sidebar_width_pct", Value::from(self.sidebar_width_pct))
            .with_data(
                "preview_offset",
                Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
            )
            .with_data("preview_auto_scroll", Value::from(self.preview.auto_scroll))
            .with_data("output_changing", Value::from(self.output_changing))
            .with_data("pending_input_depth", Value::from(pending_input_depth))
            .with_data(
                "oldest_pending_input_seq",
                Value::from(oldest_pending_input_seq),
            )
            .with_data(
                "oldest_pending_input_age_ms",
                Value::from(oldest_pending_input_age_ms),
            )
            .with_data("frame_cursor_visible", Value::from(frame.cursor_visible))
            .with_data(
                "frame_cursor_has_position",
                Value::from(frame.cursor_position.is_some()),
            );
        if let Some((cursor_col, cursor_row)) = frame.cursor_position {
            frame_event = frame_event
                .with_data("frame_cursor_col", Value::from(cursor_col))
                .with_data("frame_cursor_row", Value::from(cursor_row));
        }
        if let Some(interactive) = self.interactive.as_ref() {
            frame_event = frame_event
                .with_data(
                    "interactive_cursor_visible",
                    Value::from(interactive.cursor_visible),
                )
                .with_data(
                    "interactive_cursor_row",
                    Value::from(interactive.cursor_row),
                )
                .with_data(
                    "interactive_cursor_col",
                    Value::from(interactive.cursor_col),
                )
                .with_data(
                    "interactive_pane_width",
                    Value::from(interactive.pane_width),
                )
                .with_data(
                    "interactive_pane_height",
                    Value::from(interactive.pane_height),
                );

            let layout = Self::view_layout_for_size(
                frame.buffer.width(),
                frame.buffer.height(),
                self.sidebar_width_pct,
            );
            let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
            let preview_height = usize::from(
                preview_inner
                    .height
                    .saturating_sub(PREVIEW_METADATA_ROWS)
                    .max(1),
            );
            let cursor_target = self.interactive_cursor_target(preview_height);
            frame_event = frame_event.with_data(
                "interactive_cursor_in_viewport",
                Value::from(cursor_target.is_some()),
            );
            if let Some((visible_index, target_col, target_visible)) = cursor_target {
                frame_event = frame_event
                    .with_data(
                        "interactive_cursor_visible_index",
                        Value::from(u64::try_from(visible_index).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "interactive_cursor_target_col",
                        Value::from(u64::try_from(target_col).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "interactive_cursor_target_visible",
                        Value::from(target_visible),
                    );
            }
        }
        frame_event = frame_event.with_data(
            "frame_lines",
            Value::Array(lines.into_iter().map(Value::from).collect()),
        );
        self.event_log.log(frame_event);
    }

    fn cycle_preview_tab(&mut self, direction: i8) {
        let next_tab = if direction.is_negative() {
            self.preview_tab.previous()
        } else {
            self.preview_tab.next()
        };
        if next_tab == self.preview_tab {
            return;
        }

        self.preview_tab = next_tab;
        self.clear_preview_selection();
        if self.preview_tab == PreviewTab::Git
            && let Some(workspace) = self.state.selected_workspace()
        {
            let session_name = Self::git_tab_session_name(workspace);
            self.lazygit_failed_sessions.remove(&session_name);
        }
        self.poll_preview();
    }

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                if workspace.is_main && !workspace.status.has_session() {
                    return self.main_worktree_splash();
                }
                format!(
                    "Workspace: {}\nBranch: {}\nPath: {}\nAgent: {}\nOrphaned session: {}",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display(),
                    workspace.agent.label(),
                    if workspace.is_orphaned { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn main_worktree_splash(&self) -> String {
        // Catppuccin Mocha: green (166,227,161) for canopy, peach (250,179,135) for trunk
        const G: &str = "\x1b[38;2;166;227;161m";
        const T: &str = "\x1b[38;2;250;179;135m";
        const R: &str = "\x1b[0m";

        [
            String::new(),
            format!("{G}                    .@@@.{R}"),
            format!("{G}                 .@@@@@@@@@.{R}"),
            format!("{G}               .@@@@@@@@@@@@@.{R}"),
            format!("{G}    .@@@.     @@@@@@@@@@@@@@@@@        .@@.{R}"),
            format!("{G}  .@@@@@@@.  @@@@@@@@@@@@@@@@@@@    .@@@@@@@@.{R}"),
            format!("{G} @@@@@@@@@@@ @@@@@@@@@@@@@@@@@@@@  @@@@@@@@@@@@@{R}"),
            format!("{G} @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@'{R}"),
            format!("{G}    '@@@@@@@@  '@@@@@@@@@@@@@@@' @@@@@@@@@@@@@@'{R}"),
            format!("{G}      '@@@@'     '@@@@@@@@@@@'    '@@@@@@@@@@'{R}"),
            format!("         {T}||{R}        {G}'@@@@@@@'{R}        {G}'@@@@'{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("        {T}/||\\{R}         {T}/|||\\{R}            {T}/||\\{R}"),
            String::new(),
            "Base Worktree".to_string(),
            String::new(),
            "This is your repo root.".to_string(),
            "Create focused workspaces from here when you start new work.".to_string(),
            String::new(),
            "--------------------------------------------------".to_string(),
            String::new(),
            "Press 'n' to create a workspace".to_string(),
            String::new(),
            "Each workspace has its own directory and branch.".to_string(),
            "Run agents in parallel without branch hopping.".to_string(),
        ]
        .join("\n")
    }

    fn has_non_palette_modal_open(&self) -> bool {
        self.launch_dialog.is_some()
            || self.create_dialog.is_some()
            || self.edit_dialog.is_some()
            || self.delete_dialog.is_some()
            || self.settings_dialog.is_some()
            || self.project_dialog.is_some()
            || self.keybind_help_open
    }

    fn can_open_command_palette(&self) -> bool {
        !self.has_non_palette_modal_open() && self.interactive.is_none()
    }

    fn palette_action(
        id: &'static str,
        title: &'static str,
        description: &'static str,
        tags: &[&str],
        category: &'static str,
    ) -> PaletteActionItem {
        PaletteActionItem::new(id, title)
            .with_description(description)
            .with_tags(tags)
            .with_category(category)
    }

    fn build_command_palette_actions(&self) -> Vec<PaletteActionItem> {
        let mut actions: Vec<PaletteActionItem> = vec![
            Self::palette_action(
                PALETTE_CMD_TOGGLE_FOCUS,
                "Toggle Pane Focus",
                "Switch focus between workspace list and preview (Tab)",
                &["tab", "focus", "pane"],
                "Navigation",
            ),
            Self::palette_action(
                PALETTE_CMD_NEW_WORKSPACE,
                "New Workspace",
                "Open workspace creation dialog (n)",
                &["new", "workspace", "create", "n"],
                "Workspace",
            ),
            Self::palette_action(
                PALETTE_CMD_EDIT_WORKSPACE,
                "Edit Workspace",
                "Open workspace edit dialog (e)",
                &["edit", "workspace", "agent", "e"],
                "Workspace",
            ),
            Self::palette_action(
                PALETTE_CMD_OPEN_SETTINGS,
                "Settings",
                "Open settings dialog (S)",
                &["settings", "multiplexer", "S"],
                "Workspace",
            ),
            Self::palette_action(
                PALETTE_CMD_TOGGLE_UNSAFE,
                "Toggle Unsafe Launch",
                "Toggle launch skip-permissions default (!)",
                &["unsafe", "permissions", "!"],
                "Workspace",
            ),
            Self::palette_action(
                PALETTE_CMD_OPEN_HELP,
                "Keybind Help",
                "Open keyboard shortcut help (?)",
                &["help", "shortcuts", "?"],
                "System",
            ),
            Self::palette_action(
                PALETTE_CMD_QUIT,
                "Quit Grove",
                "Exit application (q)",
                &["quit", "exit", "q"],
                "System",
            ),
        ];

        if self.preview_agent_tab_is_focused() && self.can_start_selected_workspace() {
            actions.push(Self::palette_action(
                PALETTE_CMD_START_AGENT,
                "Start Agent",
                "Open start-agent dialog for selected workspace (s)",
                &["start", "agent", "workspace", "s"],
                "Workspace",
            ));
        }

        if self.preview_agent_tab_is_focused() && self.can_stop_selected_workspace() {
            actions.push(Self::palette_action(
                PALETTE_CMD_STOP_AGENT,
                "Stop Agent",
                "Stop selected workspace agent (x)",
                &["stop", "agent", "workspace", "x"],
                "Workspace",
            ));
        }

        if !self.delete_in_flight
            && self
                .state
                .selected_workspace()
                .is_some_and(|workspace| !workspace.is_main)
        {
            actions.push(Self::palette_action(
                PALETTE_CMD_DELETE_WORKSPACE,
                "Delete Workspace",
                "Open delete dialog for selected workspace (D)",
                &["delete", "workspace", "worktree", "D"],
                "Workspace",
            ));
        }

        if self.state.focus == PaneFocus::WorkspaceList {
            actions.push(Self::palette_action(
                PALETTE_CMD_MOVE_SELECTION_UP,
                "Select Previous Workspace",
                "Move workspace selection up (k / Up)",
                &["up", "previous", "workspace", "k"],
                "List",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_MOVE_SELECTION_DOWN,
                "Select Next Workspace",
                "Move workspace selection down (j / Down)",
                &["down", "next", "workspace", "j"],
                "List",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_OPEN_PREVIEW,
                "Open Preview",
                "Focus preview pane for selected workspace (Enter/l)",
                &["open", "preview", "enter", "l"],
                "List",
            ));
        } else {
            actions.push(Self::palette_action(
                PALETTE_CMD_FOCUS_LIST,
                "Focus Workspace List",
                "Return focus to workspace list (h/Esc)",
                &["list", "focus", "h", "esc"],
                "Navigation",
            ));
            if self.can_enter_interactive() {
                actions.push(Self::palette_action(
                    PALETTE_CMD_ENTER_INTERACTIVE,
                    "Enter Interactive Mode",
                    "Attach to selected workspace session (Enter)",
                    &["interactive", "attach", "enter"],
                    "Preview",
                ));
            }
        }

        if self.preview_agent_tab_is_focused() {
            actions.push(Self::palette_action(
                PALETTE_CMD_SCROLL_UP,
                "Scroll Up",
                "Scroll preview output up (k / Up)",
                &["scroll", "up", "k"],
                "Preview",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_SCROLL_DOWN,
                "Scroll Down",
                "Scroll preview output down (j / Down)",
                &["scroll", "down", "j"],
                "Preview",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_PAGE_UP,
                "Page Up",
                "Scroll preview up by one page (PgUp)",
                &["pageup", "pgup", "scroll"],
                "Preview",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_PAGE_DOWN,
                "Page Down",
                "Scroll preview down by one page (PgDn)",
                &["pagedown", "pgdn", "scroll"],
                "Preview",
            ));
            actions.push(Self::palette_action(
                PALETTE_CMD_SCROLL_BOTTOM,
                "Jump To Bottom",
                "Jump preview output to bottom (G)",
                &["bottom", "latest", "G"],
                "Preview",
            ));
        }

        actions
    }

    fn refresh_command_palette_actions(&mut self) {
        self.command_palette
            .replace_actions(self.build_command_palette_actions());
    }

    fn open_command_palette(&mut self) {
        if !self.can_open_command_palette() {
            return;
        }

        self.refresh_command_palette_actions();
        self.command_palette.open();
    }

    fn execute_command_palette_action(&mut self, id: &str) -> bool {
        match id {
            PALETTE_CMD_TOGGLE_FOCUS => {
                reduce(&mut self.state, Action::ToggleFocus);
                false
            }
            PALETTE_CMD_OPEN_PREVIEW => {
                self.enter_preview_or_interactive();
                false
            }
            PALETTE_CMD_ENTER_INTERACTIVE => {
                self.enter_interactive(Instant::now());
                false
            }
            PALETTE_CMD_FOCUS_LIST => {
                reduce(&mut self.state, Action::EnterListMode);
                false
            }
            PALETTE_CMD_MOVE_SELECTION_UP => {
                self.move_selection(Action::MoveSelectionUp);
                false
            }
            PALETTE_CMD_MOVE_SELECTION_DOWN => {
                self.move_selection(Action::MoveSelectionDown);
                false
            }
            PALETTE_CMD_SCROLL_UP => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(-1);
                }
                false
            }
            PALETTE_CMD_SCROLL_DOWN => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(1);
                }
                false
            }
            PALETTE_CMD_PAGE_UP => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(-5);
                }
                false
            }
            PALETTE_CMD_PAGE_DOWN => {
                if self.preview_agent_tab_is_focused() {
                    self.scroll_preview(5);
                }
                false
            }
            PALETTE_CMD_SCROLL_BOTTOM => {
                if self.preview_agent_tab_is_focused() {
                    self.jump_preview_to_bottom();
                }
                false
            }
            PALETTE_CMD_NEW_WORKSPACE => {
                self.open_create_dialog();
                false
            }
            PALETTE_CMD_EDIT_WORKSPACE => {
                self.open_edit_dialog();
                false
            }
            PALETTE_CMD_START_AGENT => {
                if self.preview_agent_tab_is_focused() {
                    self.open_start_dialog();
                }
                false
            }
            PALETTE_CMD_STOP_AGENT => {
                if self.preview_agent_tab_is_focused() {
                    self.stop_selected_workspace_agent();
                }
                false
            }
            PALETTE_CMD_DELETE_WORKSPACE => {
                self.open_delete_dialog();
                false
            }
            PALETTE_CMD_OPEN_SETTINGS => {
                self.open_settings_dialog();
                false
            }
            PALETTE_CMD_TOGGLE_UNSAFE => {
                self.launch_skip_permissions = !self.launch_skip_permissions;
                false
            }
            PALETTE_CMD_OPEN_HELP => {
                self.open_keybind_help();
                false
            }
            PALETTE_CMD_QUIT => true,
            _ => false,
        }
    }

    fn modal_open(&self) -> bool {
        self.has_non_palette_modal_open() || self.command_palette.is_visible()
    }

    fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }

    fn preview_output_dimensions(&self) -> Option<(u16, u16)> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }

        let inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if inner.is_empty() || inner.width == 0 {
            return None;
        }

        let output_height = inner.height.saturating_sub(PREVIEW_METADATA_ROWS).max(1);
        Some((inner.width, output_height))
    }

    fn git_tab_session_name(workspace: &Workspace) -> String {
        format!("{}-git", Self::workspace_session_name(workspace))
    }

    fn ensure_lazygit_session_for_selected_workspace(&mut self) -> Option<String> {
        let (workspace_path, session_name) = self.state.selected_workspace().map(|workspace| {
            (
                workspace.path.clone(),
                Self::git_tab_session_name(workspace),
            )
        })?;

        if self.lazygit_ready_sessions.contains(&session_name) {
            return Some(session_name);
        }
        if self.lazygit_failed_sessions.contains(&session_name) {
            return None;
        }

        let capture_cols = self
            .preview_output_dimensions()
            .map_or(self.viewport_width.saturating_sub(4), |(width, _)| width)
            .max(80);
        let capture_rows = self.viewport_height.saturating_sub(4).max(1);
        let launch_request = ShellLaunchRequest {
            session_name: session_name.clone(),
            workspace_path,
            command: LAZYGIT_COMMAND.to_string(),
            capture_cols: Some(capture_cols),
            capture_rows: Some(capture_rows),
        };
        let launch_plan = build_shell_launch_plan(&launch_request, self.multiplexer);

        if let Some(script) = &launch_plan.launcher_script
            && let Err(error) = fs::write(&script.path, &script.contents)
        {
            self.last_tmux_error = Some(format!("launcher script write failed: {error}"));
            self.show_toast("lazygit launch failed", true);
            self.lazygit_failed_sessions.insert(session_name);
            return None;
        }

        for command in &launch_plan.pre_launch_cmds {
            if let Err(error) = self.execute_tmux_command(command) {
                self.last_tmux_error = Some(error.to_string());
                self.show_toast("lazygit launch failed", true);
                self.lazygit_failed_sessions.insert(session_name);
                return None;
            }
        }
        if let Err(error) = self.execute_tmux_command(&launch_plan.launch_cmd) {
            self.last_tmux_error = Some(error.to_string());
            self.show_toast("lazygit launch failed", true);
            self.lazygit_failed_sessions.insert(session_name);
            return None;
        }

        self.lazygit_failed_sessions.remove(&session_name);
        self.lazygit_ready_sessions.insert(session_name.clone());
        Some(session_name)
    }

    fn selected_session_for_live_preview(&self) -> Option<(String, bool)> {
        if self.preview_tab == PreviewTab::Git {
            let workspace = self.state.selected_workspace()?;
            let session_name = Self::git_tab_session_name(workspace);
            if self.lazygit_ready_sessions.contains(&session_name) {
                return Some((session_name, true));
            }
            return None;
        }

        let workspace = self.state.selected_workspace()?;
        if workspace.status.has_session() {
            return Some((Self::workspace_session_name(workspace), true));
        }

        None
    }

    fn prepare_live_preview_session(&mut self) -> Option<(String, bool)> {
        if self.preview_tab == PreviewTab::Git {
            return self
                .ensure_lazygit_session_for_selected_workspace()
                .map(|session| (session, true));
        }
        self.selected_session_for_live_preview()
    }

    fn interactive_target_session(&self) -> Option<String> {
        self.interactive
            .as_ref()
            .map(|state| state.target_session.clone())
    }

    fn workspace_status_poll_targets(
        &self,
        selected_live_session: Option<&str>,
    ) -> Vec<WorkspaceStatusPollTarget> {
        self.state
            .workspaces
            .iter()
            .filter(|workspace| {
                if !workspace.supported_agent {
                    return false;
                }

                if self.multiplexer == MultiplexerKind::Zellij {
                    if workspace.is_main {
                        return workspace.status.has_session();
                    }
                    return true;
                }

                workspace.status.has_session()
            })
            .map(|workspace| WorkspaceStatusPollTarget {
                workspace_name: workspace.name.clone(),
                workspace_path: workspace.path.clone(),
                session_name: Self::workspace_session_name(workspace),
                supported_agent: workspace.supported_agent,
            })
            .filter(|target| selected_live_session != Some(target.session_name.as_str()))
            .collect()
    }

    fn tmux_capture_error_indicates_missing_session(error: &str) -> bool {
        let lower = error.to_ascii_lowercase();
        lower.contains("can't find pane")
            || lower.contains("can't find session")
            || lower.contains("no server running")
            || lower.contains("no sessions")
            || lower.contains("failed to connect to server")
            || lower.contains("no active session")
            || lower.contains("session not found")
    }

    fn apply_workspace_status_capture(&mut self, capture: WorkspaceStatusCapture) {
        let supported_agent = capture.supported_agent;
        let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == capture.workspace_path)
        else {
            return;
        };

        match capture.result {
            Ok(output) => {
                self.capture_changed_cleaned_for_workspace(&capture.workspace_path, &output);
                let workspace_path = self.state.workspaces[workspace_index].path.clone();
                let workspace_agent = self.state.workspaces[workspace_index].agent;
                let workspace_is_main = self.state.workspaces[workspace_index].is_main;
                let workspace = &mut self.state.workspaces[workspace_index];
                workspace.status = detect_status_with_session_override(
                    output.as_str(),
                    SessionActivity::Active,
                    workspace_is_main,
                    true,
                    supported_agent,
                    workspace_agent,
                    &workspace_path,
                );
                workspace.is_orphaned = false;
            }
            Err(error) => {
                if Self::tmux_capture_error_indicates_missing_session(&error) {
                    let workspace = &mut self.state.workspaces[workspace_index];
                    let previously_had_live_session = workspace.status.has_session();
                    workspace.status = if workspace.is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    workspace.is_orphaned = if workspace.is_main {
                        false
                    } else {
                        previously_had_live_session || workspace.is_orphaned
                    };
                    self.clear_status_tracking_for_workspace_path(&capture.workspace_path);
                }
            }
        }
    }

    fn poll_interactive_cursor_sync(&mut self, target_session: &str) {
        let started_at = Instant::now();
        let result = self
            .tmux_input
            .capture_cursor_metadata(target_session)
            .map_err(|error| error.to_string());
        let capture_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(started_at));
        self.apply_cursor_capture_result(CursorCapture {
            session: target_session.to_string(),
            capture_ms,
            result,
        });
    }

    fn sync_interactive_session_geometry(&mut self) {
        let Some(target_session) = self.interactive_target_session() else {
            return;
        };
        let Some((pane_width, pane_height)) = self.preview_output_dimensions() else {
            return;
        };

        let needs_resize = self.interactive.as_ref().is_some_and(|state| {
            state.pane_width != pane_width || state.pane_height != pane_height
        });
        if !needs_resize {
            return;
        }

        if let Some(state) = self.interactive.as_mut() {
            state.update_cursor(
                state.cursor_row,
                state.cursor_col,
                state.cursor_visible,
                pane_height,
                pane_width,
            );
        }

        if let Err(error) = self
            .tmux_input
            .resize_session(&target_session, pane_width, pane_height)
        {
            let message = error.to_string();
            self.last_tmux_error = Some(message.clone());
            self.log_tmux_error(message);
            self.pending_resize_verification = None;
            return;
        }

        self.pending_resize_verification = Some(PendingResizeVerification {
            session: target_session,
            expected_width: pane_width,
            expected_height: pane_height,
            retried: false,
        });
    }

    fn apply_live_preview_capture(
        &mut self,
        session_name: &str,
        include_escape_sequences: bool,
        capture_ms: u64,
        base_total_ms: u64,
        result: Result<String, String>,
    ) {
        match result {
            Ok(output) => {
                let apply_started_at = Instant::now();
                let update = self.preview.apply_capture(&output);
                let apply_capture_ms = Self::duration_millis(
                    Instant::now().saturating_duration_since(apply_started_at),
                );
                let consumed_inputs = if update.changed_cleaned {
                    self.drain_pending_inputs_for_session(session_name)
                } else {
                    Vec::new()
                };
                self.output_changing = update.changed_cleaned;
                self.agent_output_changing = update.changed_cleaned && consumed_inputs.is_empty();
                self.push_agent_activity_frame(self.agent_output_changing);
                let selected_workspace_index =
                    self.state.selected_workspace().and_then(|workspace| {
                        if Self::workspace_session_name(workspace) != session_name {
                            return None;
                        }
                        Some(self.state.selected_index)
                    });
                if let Some(index) = selected_workspace_index {
                    let supported_agent = self.state.workspaces[index].supported_agent;
                    let workspace_path = self.state.workspaces[index].path.clone();
                    let workspace_agent = self.state.workspaces[index].agent;
                    let workspace_is_main = self.state.workspaces[index].is_main;
                    self.capture_changed_cleaned_for_workspace(&workspace_path, output.as_str());
                    let resolved_status = detect_status_with_session_override(
                        output.as_str(),
                        SessionActivity::Active,
                        workspace_is_main,
                        true,
                        supported_agent,
                        workspace_agent,
                        &workspace_path,
                    );
                    let workspace = &mut self.state.workspaces[index];
                    workspace.status = resolved_status;
                    workspace.is_orphaned = false;
                }
                self.last_tmux_error = None;
                self.event_log.log(
                    LogEvent::new("preview_poll", "capture_completed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data("apply_capture_ms", Value::from(apply_capture_ms))
                        .with_data(
                            "total_ms",
                            Value::from(base_total_ms.saturating_add(apply_capture_ms)),
                        )
                        .with_data(
                            "output_bytes",
                            Value::from(u64::try_from(output.len()).unwrap_or(u64::MAX)),
                        )
                        .with_data("changed", Value::from(update.changed_cleaned))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        ),
                );
                if update.changed_cleaned {
                    let line_count = u64::try_from(self.preview.lines.len()).unwrap_or(u64::MAX);
                    let now = Instant::now();
                    let mut output_event = LogEvent::new("preview_update", "output_changed")
                        .with_data("line_count", Value::from(line_count))
                        .with_data("session", Value::from(session_name.to_string()));
                    if let Some(first_input) = consumed_inputs.first() {
                        let last_index = consumed_inputs.len().saturating_sub(1);
                        let last_input = &consumed_inputs[last_index];
                        let oldest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.received_at),
                        );
                        let newest_input_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.received_at),
                        );
                        let oldest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(first_input.forwarded_at),
                        );
                        let newest_tmux_to_preview_ms = Self::duration_millis(
                            now.saturating_duration_since(last_input.forwarded_at),
                        );
                        let consumed_count =
                            u64::try_from(consumed_inputs.len()).unwrap_or(u64::MAX);
                        let consumed_seq_first = first_input.seq;
                        let consumed_seq_last = last_input.seq;

                        output_event = output_event
                            .with_data("input_seq", Value::from(consumed_seq_first))
                            .with_data(
                                "input_to_preview_ms",
                                Value::from(oldest_input_to_preview_ms),
                            )
                            .with_data("tmux_to_preview_ms", Value::from(oldest_tmux_to_preview_ms))
                            .with_data("consumed_input_count", Value::from(consumed_count))
                            .with_data("consumed_input_seq_first", Value::from(consumed_seq_first))
                            .with_data("consumed_input_seq_last", Value::from(consumed_seq_last))
                            .with_data(
                                "newest_input_to_preview_ms",
                                Value::from(newest_input_to_preview_ms),
                            )
                            .with_data(
                                "newest_tmux_to_preview_ms",
                                Value::from(newest_tmux_to_preview_ms),
                            );

                        self.log_input_event_with_fields(
                            "interactive_input_to_preview",
                            consumed_seq_first,
                            vec![
                                ("session".to_string(), Value::from(session_name.to_string())),
                                (
                                    "input_to_preview_ms".to_string(),
                                    Value::from(oldest_input_to_preview_ms),
                                ),
                                (
                                    "tmux_to_preview_ms".to_string(),
                                    Value::from(oldest_tmux_to_preview_ms),
                                ),
                                (
                                    "newest_input_to_preview_ms".to_string(),
                                    Value::from(newest_input_to_preview_ms),
                                ),
                                (
                                    "newest_tmux_to_preview_ms".to_string(),
                                    Value::from(newest_tmux_to_preview_ms),
                                ),
                                (
                                    "consumed_input_count".to_string(),
                                    Value::from(consumed_count),
                                ),
                                (
                                    "consumed_input_seq_first".to_string(),
                                    Value::from(consumed_seq_first),
                                ),
                                (
                                    "consumed_input_seq_last".to_string(),
                                    Value::from(consumed_seq_last),
                                ),
                                (
                                    "queue_depth".to_string(),
                                    Value::from(self.pending_input_depth()),
                                ),
                            ],
                        );
                        if consumed_inputs.len() > 1 {
                            self.log_input_event_with_fields(
                                "interactive_inputs_coalesced",
                                consumed_seq_first,
                                vec![
                                    ("session".to_string(), Value::from(session_name.to_string())),
                                    (
                                        "consumed_input_count".to_string(),
                                        Value::from(consumed_count),
                                    ),
                                    (
                                        "consumed_input_seq_last".to_string(),
                                        Value::from(consumed_seq_last),
                                    ),
                                ],
                            );
                        }
                    }
                    self.event_log.log(output_event);
                }
            }
            Err(message) => {
                self.clear_agent_activity_tracking();
                let capture_error_indicates_missing_session =
                    Self::tmux_capture_error_indicates_missing_session(&message);
                if capture_error_indicates_missing_session {
                    self.lazygit_ready_sessions.remove(session_name);
                    if let Some(workspace) = self.state.selected_workspace_mut()
                        && Self::workspace_session_name(workspace) == session_name
                    {
                        let workspace_path = workspace.path.clone();
                        workspace.status = if workspace.is_main {
                            WorkspaceStatus::Main
                        } else {
                            WorkspaceStatus::Idle
                        };
                        workspace.is_orphaned = !workspace.is_main;
                        self.clear_status_tracking_for_workspace_path(&workspace_path);
                    }
                    if self
                        .interactive
                        .as_ref()
                        .is_some_and(|interactive| interactive.target_session == session_name)
                    {
                        self.interactive = None;
                    }
                }
                self.last_tmux_error = Some(message.clone());
                self.event_log.log(
                    LogEvent::new("preview_poll", "capture_failed")
                        .with_data("session", Value::from(session_name.to_string()))
                        .with_data("capture_ms", Value::from(capture_ms))
                        .with_data(
                            "include_escape_sequences",
                            Value::from(include_escape_sequences),
                        )
                        .with_data("error", Value::from(message.clone())),
                );
                self.log_tmux_error(message.clone());
                self.show_toast("preview capture failed", true);
                self.refresh_preview_summary();
            }
        }
    }

    fn apply_cursor_capture_result(&mut self, cursor_capture: CursorCapture) {
        let parse_started_at = Instant::now();
        let raw_metadata = match cursor_capture.result {
            Ok(raw_metadata) => raw_metadata,
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("preview_poll", "cursor_capture_failed")
                        .with_data("session", Value::from(cursor_capture.session))
                        .with_data("duration_ms", Value::from(cursor_capture.capture_ms))
                        .with_data("error", Value::from(error)),
                );
                return;
            }
        };
        let metadata = match parse_cursor_metadata(&raw_metadata) {
            Some(metadata) => metadata,
            None => {
                self.event_log.log(
                    LogEvent::new("preview_poll", "cursor_parse_failed")
                        .with_data("session", Value::from(cursor_capture.session))
                        .with_data("capture_ms", Value::from(cursor_capture.capture_ms))
                        .with_data(
                            "parse_ms",
                            Value::from(Self::duration_millis(
                                Instant::now().saturating_duration_since(parse_started_at),
                            )),
                        )
                        .with_data("raw_metadata", Value::from(raw_metadata)),
                );
                return;
            }
        };
        let Some(state) = self.interactive.as_mut() else {
            return;
        };
        let session = cursor_capture.session.clone();

        let changed = state.update_cursor(
            metadata.cursor_row,
            metadata.cursor_col,
            metadata.cursor_visible,
            metadata.pane_height,
            metadata.pane_width,
        );
        self.verify_resize_after_cursor_capture(
            &session,
            metadata.pane_width,
            metadata.pane_height,
        );
        let parse_duration_ms =
            Self::duration_millis(Instant::now().saturating_duration_since(parse_started_at));
        self.event_log.log(
            LogEvent::new("preview_poll", "cursor_capture_completed")
                .with_data("session", Value::from(session))
                .with_data("capture_ms", Value::from(cursor_capture.capture_ms))
                .with_data("parse_ms", Value::from(parse_duration_ms))
                .with_data("changed", Value::from(changed))
                .with_data("cursor_visible", Value::from(metadata.cursor_visible))
                .with_data("cursor_row", Value::from(metadata.cursor_row))
                .with_data("cursor_col", Value::from(metadata.cursor_col))
                .with_data("pane_width", Value::from(metadata.pane_width))
                .with_data("pane_height", Value::from(metadata.pane_height)),
        );
    }

    fn verify_resize_after_cursor_capture(
        &mut self,
        session: &str,
        pane_width: u16,
        pane_height: u16,
    ) {
        let Some(pending) = self.pending_resize_verification.clone() else {
            return;
        };
        if pending.session != session {
            return;
        }

        if pending.expected_width == pane_width && pending.expected_height == pane_height {
            self.pending_resize_verification = None;
            return;
        }

        if pending.retried {
            self.event_log.log(
                LogEvent::new("preview_poll", "resize_verify_failed")
                    .with_data("session", Value::from(session.to_string()))
                    .with_data("expected_width", Value::from(pending.expected_width))
                    .with_data("expected_height", Value::from(pending.expected_height))
                    .with_data("actual_width", Value::from(pane_width))
                    .with_data("actual_height", Value::from(pane_height)),
            );
            self.pending_resize_verification = None;
            return;
        }

        self.event_log.log(
            LogEvent::new("preview_poll", "resize_verify_retry")
                .with_data("session", Value::from(session.to_string()))
                .with_data("expected_width", Value::from(pending.expected_width))
                .with_data("expected_height", Value::from(pending.expected_height))
                .with_data("actual_width", Value::from(pane_width))
                .with_data("actual_height", Value::from(pane_height)),
        );
        self.pending_resize_verification = Some(PendingResizeVerification {
            retried: true,
            ..pending.clone()
        });
        if let Err(error) =
            self.tmux_input
                .resize_session(session, pending.expected_width, pending.expected_height)
        {
            let message = error.to_string();
            self.last_tmux_error = Some(message.clone());
            self.log_tmux_error(message);
            self.pending_resize_verification = None;
            return;
        }

        self.poll_preview();
    }

    fn poll_preview_sync(&mut self) {
        let live_preview = self.prepare_live_preview_session();
        let has_live_preview = live_preview.is_some();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = self.workspace_status_poll_targets(
            live_preview.as_ref().map(|(session, _)| session.as_str()),
        );

        if let Some((session_name, include_escape_sequences)) = live_preview {
            let capture_started_at = Instant::now();
            let result = self
                .tmux_input
                .capture_output(&session_name, 600, include_escape_sequences)
                .map_err(|error| error.to_string());
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            self.apply_live_preview_capture(
                &session_name,
                include_escape_sequences,
                capture_ms,
                capture_ms,
                result,
            );
        } else {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
        }

        for target in status_poll_targets {
            let capture_started_at = Instant::now();
            let result = self
                .tmux_input
                .capture_output(&target.session_name, 120, false)
                .map_err(|error| error.to_string());
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            self.apply_workspace_status_capture(WorkspaceStatusCapture {
                workspace_name: target.workspace_name,
                workspace_path: target.workspace_path,
                session_name: target.session_name,
                supported_agent: target.supported_agent,
                capture_ms,
                result,
            });
        }
        if !has_live_preview {
            self.refresh_preview_summary();
        }

        if let Some(target_session) = cursor_session {
            self.poll_interactive_cursor_sync(&target_session);
        }
    }

    fn schedule_async_preview_poll(
        &self,
        generation: u64,
        live_preview: Option<(String, bool)>,
        cursor_session: Option<String>,
        status_poll_targets: Vec<WorkspaceStatusPollTarget>,
    ) -> Cmd<Msg> {
        Cmd::task(move || {
            let live_capture = live_preview.map(|(session, include_escape_sequences)| {
                let capture_started_at = Instant::now();
                let result = CommandTmuxInput::capture_session_output(
                    &session,
                    600,
                    include_escape_sequences,
                )
                .map_err(|error| error.to_string());
                let capture_ms = GroveApp::duration_millis(
                    Instant::now().saturating_duration_since(capture_started_at),
                );
                LivePreviewCapture {
                    session,
                    include_escape_sequences,
                    capture_ms,
                    total_ms: capture_ms,
                    result,
                }
            });

            let cursor_capture = cursor_session.map(|session| {
                let started_at = Instant::now();
                let result = CommandTmuxInput::capture_session_cursor_metadata(&session)
                    .map_err(|error| error.to_string());
                let capture_ms =
                    GroveApp::duration_millis(Instant::now().saturating_duration_since(started_at));
                CursorCapture {
                    session,
                    capture_ms,
                    result,
                }
            });

            let workspace_status_captures = status_poll_targets
                .into_iter()
                .map(|target| {
                    let capture_started_at = Instant::now();
                    let result =
                        CommandTmuxInput::capture_session_output(&target.session_name, 120, false)
                            .map_err(|error| error.to_string());
                    let capture_ms = GroveApp::duration_millis(
                        Instant::now().saturating_duration_since(capture_started_at),
                    );
                    WorkspaceStatusCapture {
                        workspace_name: target.workspace_name,
                        workspace_path: target.workspace_path,
                        session_name: target.session_name,
                        supported_agent: target.supported_agent,
                        capture_ms,
                        result,
                    }
                })
                .collect();

            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation,
                live_capture,
                cursor_capture,
                workspace_status_captures,
            })
        })
    }

    fn poll_preview(&mut self) {
        if !self.tmux_input.supports_background_send() {
            self.poll_preview_sync();
            return;
        }

        let live_preview = self.prepare_live_preview_session();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = self.workspace_status_poll_targets(
            live_preview.as_ref().map(|(session, _)| session.as_str()),
        );

        if live_preview.is_none() && cursor_session.is_none() && status_poll_targets.is_empty() {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
            return;
        }

        self.poll_generation = self.poll_generation.saturating_add(1);
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            cursor_session,
            status_poll_targets,
        ));
    }

    fn handle_preview_poll_completed(&mut self, completion: PreviewPollCompletion) {
        if completion.generation < self.poll_generation {
            self.event_log.log(
                LogEvent::new("preview_poll", "stale_result_dropped")
                    .with_data("generation", Value::from(completion.generation))
                    .with_data("latest_generation", Value::from(self.poll_generation)),
            );
            return;
        }

        if completion.generation > self.poll_generation {
            self.poll_generation = completion.generation;
        }

        let had_live_capture = completion.live_capture.is_some();
        if let Some(live_capture) = completion.live_capture {
            self.apply_live_preview_capture(
                &live_capture.session,
                live_capture.include_escape_sequences,
                live_capture.capture_ms,
                live_capture.total_ms,
                live_capture.result,
            );
        } else {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
        }

        for status_capture in completion.workspace_status_captures {
            self.apply_workspace_status_capture(status_capture);
        }
        if !had_live_capture {
            self.refresh_preview_summary();
        }

        if let Some(cursor_capture) = completion.cursor_capture {
            self.apply_cursor_capture_result(cursor_capture);
        }
    }

    fn scroll_preview(&mut self, delta: i32) {
        let viewport_height = self
            .preview_output_dimensions()
            .map_or(1, |(_, height)| usize::from(height));
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        let changed = self.preview.scroll(delta, Instant::now(), viewport_height);
        if changed {
            let offset = u64::try_from(self.preview.offset).unwrap_or(u64::MAX);
            self.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from(i64::from(delta)))
                    .with_data("offset", Value::from(offset)),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "previous_offset",
                        Value::from(u64::try_from(old_offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
    }

    fn jump_preview_to_bottom(&mut self) {
        let old_offset = self.preview.offset;
        let old_auto_scroll = self.preview.auto_scroll;
        self.preview.jump_to_bottom();
        if old_offset != self.preview.offset {
            self.event_log.log(
                LogEvent::new("preview_update", "scrolled")
                    .with_data("delta", Value::from("jump_bottom"))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        "previous_offset",
                        Value::from(u64::try_from(old_offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
        if old_auto_scroll != self.preview.auto_scroll {
            self.event_log.log(
                LogEvent::new("preview_update", "autoscroll_toggled")
                    .with_data("enabled", Value::from(self.preview.auto_scroll))
                    .with_data(
                        "offset",
                        Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
                    ),
            );
        }
    }

    fn allows_text_input_modifiers(modifiers: Modifiers) -> bool {
        modifiers.is_empty() || modifiers == Modifiers::SHIFT
    }

    fn apply_delete_workspace_completion(&mut self, completion: DeleteWorkspaceCompletion) {
        self.delete_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_deleted")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data(
                            "warning_count",
                            Value::from(
                                u64::try_from(completion.warnings.len()).unwrap_or(u64::MAX),
                            ),
                        ),
                );
                self.last_tmux_error = None;
                self.refresh_workspaces(None);
                if completion.warnings.is_empty() {
                    self.show_toast(
                        format!("workspace '{}' deleted", completion.workspace_name),
                        false,
                    );
                } else if let Some(first_warning) = completion.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' deleted, warning: {}",
                            completion.workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.event_log.log(
                    LogEvent::new("workspace_lifecycle", "workspace_delete_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("error", Value::from(error.clone())),
                );
                self.last_tmux_error = Some(error.clone());
                self.show_toast(format!("workspace delete failed: {error}"), true);
            }
        }
    }

    fn run_delete_workspace(
        dialog: DeleteDialogState,
        multiplexer: MultiplexerKind,
    ) -> (Result<(), String>, Vec<String>) {
        let mut warnings = Vec::new();
        let session_name = session_name_for_workspace_in_project(
            dialog.project_name.as_deref(),
            &dialog.workspace_name,
        );
        let stop_session_command = match multiplexer {
            MultiplexerKind::Tmux => vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                session_name,
            ],
            MultiplexerKind::Zellij => vec![
                "zellij".to_string(),
                "--config".to_string(),
                zellij_config_path().to_string_lossy().to_string(),
                "kill-session".to_string(),
                session_name,
            ],
        };
        let _ = CommandTmuxInput::execute_command(&stop_session_command);

        let repo_root = if let Some(project_path) = dialog.project_path.clone() {
            project_path
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd
        } else {
            return (
                Err("workspace project root unavailable".to_string()),
                warnings,
            );
        };

        if let Err(error) =
            Self::run_delete_worktree_git(&repo_root, &dialog.path, dialog.is_missing)
        {
            return (Err(error), warnings);
        }

        if dialog.delete_local_branch
            && let Err(error) = Self::run_delete_local_branch_git(&repo_root, &dialog.branch)
        {
            warnings.push(format!("local branch: {error}"));
        }

        (Ok(()), warnings)
    }

    fn run_delete_worktree_git(
        repo_root: &Path,
        workspace_path: &Path,
        is_missing: bool,
    ) -> Result<(), String> {
        if is_missing {
            return Self::run_git_command(
                repo_root,
                &["worktree".to_string(), "prune".to_string()],
            )
            .map_err(|error| format!("git worktree prune failed: {error}"));
        }

        let workspace_path_arg = workspace_path.to_string_lossy().to_string();
        let remove_args = vec![
            "worktree".to_string(),
            "remove".to_string(),
            workspace_path_arg.clone(),
        ];
        if Self::run_git_command(repo_root, &remove_args).is_ok() {
            return Ok(());
        }

        Self::run_git_command(
            repo_root,
            &[
                "worktree".to_string(),
                "remove".to_string(),
                "--force".to_string(),
                workspace_path_arg,
            ],
        )
        .map_err(|error| format!("git worktree remove failed: {error}"))
    }

    fn run_delete_local_branch_git(repo_root: &Path, branch: &str) -> Result<(), String> {
        let safe_args = vec!["branch".to_string(), "-d".to_string(), branch.to_string()];
        if Self::run_git_command(repo_root, &safe_args).is_ok() {
            return Ok(());
        }

        Self::run_git_command(
            repo_root,
            &["branch".to_string(), "-D".to_string(), branch.to_string()],
        )
        .map_err(|error| format!("git branch delete failed: {error}"))
    }

    fn run_git_command(repo_root: &Path, args: &[String]) -> Result<(), String> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(args)
            .output()
            .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "git {}: exit status {}",
                args.join(" "),
                output.status
            ));
        }
        Err(format!("git {}: {stderr}", args.join(" ")))
    }

    fn selected_base_branch(&self) -> String {
        let selected = self.state.selected_workspace();
        if let Some(workspace) = selected
            && let Some(base_branch) = workspace.base_branch.as_ref()
            && !base_branch.trim().is_empty()
        {
            return base_branch.clone();
        }

        if let Some(workspace) = selected
            && !workspace.branch.trim().is_empty()
            && workspace.branch != "(detached)"
        {
            return workspace.branch.clone();
        }

        "main".to_string()
    }

    fn selected_project_index(&self) -> usize {
        let Some(workspace) = self.state.selected_workspace() else {
            return 0;
        };
        let Some(workspace_project_path) = workspace.project_path.as_ref() else {
            return 0;
        };
        self.projects
            .iter()
            .position(|project| project_paths_equal(&project.path, workspace_project_path))
            .unwrap_or(0)
    }

    fn create_dialog_selected_project(&self) -> Option<&ProjectConfig> {
        let dialog = self.create_dialog.as_ref()?;
        self.projects.get(dialog.project_index)
    }

    fn refresh_create_dialog_branch_candidates(&mut self, selected_base_branch: String) {
        let branches = self
            .create_dialog_selected_project()
            .map(|project| load_local_branches(&project.path).unwrap_or_default())
            .unwrap_or_default();
        self.create_branch_all = branches;
        if !self
            .create_branch_all
            .iter()
            .any(|branch| branch == &selected_base_branch)
        {
            self.create_branch_all.insert(0, selected_base_branch);
        }
        self.refresh_create_branch_filtered();
    }

    fn open_create_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.projects.is_empty() {
            self.show_toast("no projects configured, press p to add one", true);
            return;
        }

        let selected_base_branch = self.selected_base_branch();
        let default_agent = self
            .state
            .selected_workspace()
            .map_or(AgentType::Claude, |workspace| workspace.agent);
        let project_index = self.selected_project_index();
        self.create_dialog = Some(CreateDialogState {
            workspace_name: String::new(),
            project_index,
            agent: default_agent,
            base_branch: selected_base_branch.clone(),
            focused_field: CreateDialogField::WorkspaceName,
        });
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
        self.log_dialog_event_with_fields(
            "create",
            "dialog_opened",
            [("agent".to_string(), Value::from(default_agent.label()))],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn toggle_agent(agent: AgentType) -> AgentType {
        match agent {
            AgentType::Claude => AgentType::Codex,
            AgentType::Codex => AgentType::Claude,
        }
    }

    fn toggle_create_dialog_agent(dialog: &mut CreateDialogState) {
        dialog.agent = Self::toggle_agent(dialog.agent);
    }

    fn open_edit_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };

        self.edit_dialog = Some(EditDialogState {
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            branch: workspace.branch.clone(),
            agent: workspace.agent,
            was_running: workspace.status.has_session(),
            focused_field: EditDialogField::Agent,
        });
        self.log_dialog_event_with_fields(
            "edit",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("agent".to_string(), Value::from(workspace.agent.label())),
                (
                    "running".to_string(),
                    Value::from(workspace.status.has_session()),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn toggle_edit_dialog_agent(dialog: &mut EditDialogState) {
        dialog.agent = Self::toggle_agent(dialog.agent);
    }

    fn apply_edit_dialog_save(&mut self) {
        let Some(dialog) = self.edit_dialog.as_ref().cloned() else {
            return;
        };

        self.log_dialog_event_with_fields(
            "edit",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("was_running".to_string(), Value::from(dialog.was_running)),
            ],
        );

        if let Err(error) = write_workspace_agent_marker(&dialog.workspace_path, dialog.agent) {
            self.show_toast(
                format!(
                    "workspace edit failed: {}",
                    Self::workspace_lifecycle_error_message(&error)
                ),
                true,
            );
            return;
        }

        if let Some(workspace) = self
            .state
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.path == dialog.workspace_path)
        {
            workspace.agent = dialog.agent;
            workspace.supported_agent = true;
        }

        self.edit_dialog = None;
        self.last_tmux_error = None;
        if dialog.was_running {
            self.show_toast("workspace updated, restart agent to apply change", false);
        } else {
            self.show_toast("workspace updated", false);
        }
    }

    fn shift_create_dialog_project(&mut self, delta: isize) {
        let Some(dialog) = self.create_dialog.as_mut() else {
            return;
        };
        if self.projects.is_empty() {
            return;
        }

        let len = self.projects.len();
        let current = dialog.project_index.min(len.saturating_sub(1));
        let mut next = current;
        if delta < 0 {
            next = current.saturating_sub(1);
        } else if delta > 0 {
            next = (current.saturating_add(1)).min(len.saturating_sub(1));
        }

        if next == dialog.project_index {
            return;
        }

        dialog.project_index = next;
        let selected_base_branch = dialog.base_branch.clone();
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
    }

    fn clear_create_branch_picker(&mut self) {
        self.create_branch_all.clear();
        self.create_branch_filtered.clear();
        self.create_branch_index = 0;
    }

    fn refresh_create_branch_filtered(&mut self) {
        let query = self
            .create_dialog
            .as_ref()
            .map(|dialog| dialog.base_branch.clone())
            .unwrap_or_default();
        self.create_branch_filtered = filter_branches(&query, &self.create_branch_all);
        if self.create_branch_filtered.is_empty() {
            self.create_branch_index = 0;
            return;
        }
        if self.create_branch_index >= self.create_branch_filtered.len() {
            self.create_branch_index = self.create_branch_filtered.len().saturating_sub(1);
        }
    }

    fn create_base_branch_dropdown_visible(&self) -> bool {
        self.create_dialog.as_ref().is_some_and(|dialog| {
            dialog.focused_field == CreateDialogField::BaseBranch
                && !self.create_branch_filtered.is_empty()
        })
    }

    fn select_create_base_branch_from_dropdown(&mut self) -> bool {
        if !self.create_base_branch_dropdown_visible() {
            return false;
        }
        let Some(selected_branch) = self
            .create_branch_filtered
            .get(self.create_branch_index)
            .cloned()
        else {
            return false;
        };
        if let Some(dialog) = self.create_dialog.as_mut() {
            dialog.base_branch = selected_branch;
            return true;
        }
        false
    }

    fn workspace_lifecycle_error_message(error: &WorkspaceLifecycleError) -> String {
        match error {
            WorkspaceLifecycleError::EmptyWorkspaceName => "workspace name is required".to_string(),
            WorkspaceLifecycleError::InvalidWorkspaceName => {
                "workspace name must be [A-Za-z0-9_-]".to_string()
            }
            WorkspaceLifecycleError::EmptyBaseBranch => "base branch is required".to_string(),
            WorkspaceLifecycleError::EmptyExistingBranch => {
                "existing branch is required".to_string()
            }
            WorkspaceLifecycleError::RepoNameUnavailable => "repo name unavailable".to_string(),
            WorkspaceLifecycleError::GitCommandFailed(message) => {
                format!("git command failed: {message}")
            }
            WorkspaceLifecycleError::Io(message) => format!("io error: {message}"),
        }
    }

    fn refresh_workspaces(&mut self, preferred_workspace_path: Option<PathBuf>) {
        if !self.tmux_input.supports_background_send() {
            self.refresh_workspaces_sync(preferred_workspace_path);
            return;
        }

        if self.refresh_in_flight {
            return;
        }

        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let multiplexer = self.multiplexer;
        let projects = self.projects.clone();
        self.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let bootstrap = bootstrap_data_for_projects(&projects, multiplexer);
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_path: target_path,
                bootstrap,
            })
        }));
    }

    fn refresh_workspaces_sync(&mut self, preferred_workspace_path: Option<PathBuf>) {
        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let bootstrap = bootstrap_data_for_projects(&self.projects, self.multiplexer);

        self.repo_name = bootstrap.repo_name;
        self.discovery_state = bootstrap.discovery_state;
        self.state = AppState::new(bootstrap.workspaces);
        if let Some(path) = target_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
    }

    fn apply_refresh_workspaces_completion(&mut self, completion: RefreshWorkspacesCompletion) {
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;

        self.repo_name = completion.bootstrap.repo_name;
        self.discovery_state = completion.bootstrap.discovery_state;
        self.state = AppState::new(completion.bootstrap.workspaces);
        if let Some(path) = completion.preferred_workspace_path
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.path == path)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.refresh_in_flight = false;
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
    }

    fn confirm_create_dialog(&mut self) {
        if self.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog.as_ref().cloned() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "create",
            "dialog_confirmed",
            [
                (
                    "workspace_name".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("branch_mode".to_string(), Value::from("new")),
                (
                    "branch_value".to_string(),
                    Value::from(dialog.base_branch.clone()),
                ),
                (
                    "project_index".to_string(),
                    Value::from(u64::try_from(dialog.project_index).unwrap_or(u64::MAX)),
                ),
            ],
        );
        let Some(project) = self.projects.get(dialog.project_index).cloned() else {
            self.show_toast("project is required", true);
            return;
        };

        let workspace_name = dialog.workspace_name.trim().to_string();
        let branch_mode = BranchMode::NewBranch {
            base_branch: dialog.base_branch.trim().to_string(),
        };
        let request = CreateWorkspaceRequest {
            workspace_name: workspace_name.clone(),
            branch_mode,
            agent: dialog.agent,
        };

        if let Err(error) = request.validate() {
            self.show_toast(Self::workspace_lifecycle_error_message(&error), true);
            return;
        }

        let repo_root = project.path;
        if !self.tmux_input.supports_background_send() {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let result = create_workspace(&repo_root, &request, &git, &setup);
            self.apply_create_workspace_completion(CreateWorkspaceCompletion { request, result });
            return;
        }

        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let result = create_workspace(&repo_root, &request, &git, &setup);
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion { request, result })
        }));
    }

    fn apply_create_workspace_completion(&mut self, completion: CreateWorkspaceCompletion) {
        self.create_in_flight = false;
        let workspace_name = completion.request.workspace_name;
        match completion.result {
            Ok(result) => {
                self.create_dialog = None;
                self.clear_create_branch_picker();
                self.refresh_workspaces(Some(result.workspace_path));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if result.warnings.is_empty() {
                    self.show_toast(format!("workspace '{}' created", workspace_name), false);
                } else if let Some(first_warning) = result.warnings.first() {
                    self.show_toast(
                        format!(
                            "workspace '{}' created, warning: {}",
                            workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.show_toast(
                    format!(
                        "workspace create failed: {}",
                        Self::workspace_lifecycle_error_message(&error)
                    ),
                    true,
                );
            }
        }
    }

    fn start_selected_workspace_agent_with_options(
        &mut self,
        prompt: Option<String>,
        pre_launch_command: Option<String>,
        skip_permissions: bool,
    ) {
        if self.start_in_flight {
            return;
        }

        if !self.can_start_selected_workspace() {
            self.show_toast("workspace cannot be started", true);
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        let capture_cols = self
            .preview_output_dimensions()
            .map_or(self.viewport_width.saturating_sub(4), |(width, _)| width)
            .max(80);
        let capture_rows = self.viewport_height.saturating_sub(4).max(1);

        let request = LaunchRequest {
            project_name: workspace.project_name.clone(),
            capture_cols: Some(capture_cols),
            capture_rows: Some(capture_rows),
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            agent: workspace.agent,
            prompt,
            pre_launch_command,
            skip_permissions,
        };
        let launch_plan = build_launch_plan(&request, self.multiplexer);
        let workspace_name = request.workspace_name.clone();
        let workspace_path = request.workspace_path.clone();
        let session_name = launch_plan.session_name.clone();

        if !self.tmux_input.supports_background_send() {
            if let Some(script) = &launch_plan.launcher_script
                && let Err(error) = fs::write(&script.path, &script.contents)
            {
                self.last_tmux_error = Some(format!("launcher script write failed: {error}"));
                self.show_toast("launcher script write failed", true);
                return;
            }

            for command in &launch_plan.pre_launch_cmds {
                if let Err(error) = self.execute_tmux_command(command) {
                    self.last_tmux_error = Some(error.to_string());
                    self.show_toast("agent start failed", true);
                    return;
                }
            }

            if let Err(error) = self.execute_tmux_command(&launch_plan.launch_cmd) {
                self.last_tmux_error = Some(error.to_string());
                self.show_toast("agent start failed", true);
                return;
            }

            self.apply_start_agent_completion(StartAgentCompletion {
                workspace_name,
                workspace_path,
                session_name,
                result: Ok(()),
            });
            return;
        }

        self.start_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = Self::run_start_agent_plan(launch_plan).map_err(|error| error.to_string());
            Msg::StartAgentCompleted(StartAgentCompletion {
                workspace_name,
                workspace_path,
                session_name,
                result,
            })
        }));
    }

    fn run_start_agent_plan(launch_plan: crate::agent_runtime::LaunchPlan) -> std::io::Result<()> {
        if let Some(script) = &launch_plan.launcher_script {
            fs::write(&script.path, &script.contents)?;
        }

        for command in &launch_plan.pre_launch_cmds {
            CommandTmuxInput::execute_command(command)?;
        }

        CommandTmuxInput::execute_command(&launch_plan.launch_cmd)
    }

    fn apply_start_agent_completion(&mut self, completion: StartAgentCompletion) {
        self.start_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.path == completion.workspace_path)
                {
                    workspace.status = WorkspaceStatus::Active;
                    workspace.is_orphaned = false;
                }
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_started")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_toast("agent started", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_toast("agent start failed", true);
            }
        }
    }

    fn confirm_start_dialog(&mut self) {
        let Some(dialog) = self.launch_dialog.take() else {
            return;
        };
        let workspace_name = self.selected_workspace_name().unwrap_or_default();
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_confirmed",
            [
                ("workspace".to_string(), Value::from(workspace_name)),
                (
                    "prompt_len".to_string(),
                    Value::from(u64::try_from(dialog.prompt.len()).unwrap_or(u64::MAX)),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(dialog.skip_permissions),
                ),
                (
                    "pre_launch_len".to_string(),
                    Value::from(u64::try_from(dialog.pre_launch_command.len()).unwrap_or(u64::MAX)),
                ),
            ],
        );

        self.launch_skip_permissions = dialog.skip_permissions;
        let prompt = if dialog.prompt.trim().is_empty() {
            None
        } else {
            Some(dialog.prompt.trim().to_string())
        };
        let pre_launch_command = if dialog.pre_launch_command.trim().is_empty() {
            None
        } else {
            Some(dialog.pre_launch_command.trim().to_string())
        };
        self.start_selected_workspace_agent_with_options(
            prompt,
            pre_launch_command,
            dialog.skip_permissions,
        );
    }

    fn can_stop_selected_workspace(&self) -> bool {
        if self.stop_in_flight {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        workspace.status.has_session()
    }

    fn stop_selected_workspace_agent(&mut self) {
        if self.stop_in_flight {
            return;
        }

        if !self.can_stop_selected_workspace() {
            self.show_toast("no agent running", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        let workspace_name = workspace.name.clone();
        let workspace_path = workspace.path.clone();
        let session_name = Self::workspace_session_name(workspace);
        let stop_commands = stop_plan(&session_name, self.multiplexer);

        if !self.tmux_input.supports_background_send() {
            for command in &stop_commands {
                if let Err(error) = self.execute_tmux_command(command) {
                    self.last_tmux_error = Some(error.to_string());
                    self.show_toast("agent stop failed", true);
                    return;
                }
            }

            self.apply_stop_agent_completion(StopAgentCompletion {
                workspace_name,
                workspace_path,
                session_name,
                result: Ok(()),
            });
            return;
        }

        self.stop_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = Self::run_stop_commands(&stop_commands).map_err(|error| error.to_string());
            Msg::StopAgentCompleted(StopAgentCompletion {
                workspace_name,
                workspace_path,
                session_name,
                result,
            })
        }));
    }

    fn run_stop_commands(commands: &[Vec<String>]) -> std::io::Result<()> {
        for command in commands {
            CommandTmuxInput::execute_command(command)?;
        }
        Ok(())
    }

    fn apply_stop_agent_completion(&mut self, completion: StopAgentCompletion) {
        self.stop_in_flight = false;
        match completion.result {
            Ok(()) => {
                if self
                    .interactive
                    .as_ref()
                    .is_some_and(|state| state.target_session == completion.session_name)
                {
                    self.interactive = None;
                }

                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.path == completion.workspace_path)
                {
                    workspace.status = if workspace.is_main {
                        WorkspaceStatus::Main
                    } else {
                        WorkspaceStatus::Idle
                    };
                    workspace.is_orphaned = false;
                }
                self.clear_status_tracking_for_workspace_path(&completion.workspace_path);
                self.clear_agent_activity_tracking();
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_stopped")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_toast("agent stopped", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_toast("agent stop failed", true);
            }
        }
    }

    fn next_poll_interval(&self) -> Duration {
        let status = self.selected_workspace_status();

        let since_last_key = self
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            status,
            true,
            self.state.focus == PaneFocus::Preview,
            self.interactive.is_some(),
            since_last_key,
            self.output_changing,
        )
    }

    fn selected_workspace_status(&self) -> WorkspaceStatus {
        self.state
            .selected_workspace()
            .map_or(WorkspaceStatus::Unknown, |workspace| workspace.status)
    }

    fn clear_agent_activity_tracking(&mut self) {
        self.output_changing = false;
        self.agent_output_changing = false;
        self.agent_activity_frames.clear();
    }

    fn workspace_status_tracking_key(workspace_path: &Path) -> String {
        workspace_path.to_string_lossy().to_string()
    }

    fn clear_status_tracking_for_workspace_path(&mut self, workspace_path: &Path) {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_status_digests.remove(&key);
        self.workspace_output_changing.remove(&key);
    }

    fn clear_status_tracking(&mut self) {
        self.workspace_status_digests.clear();
        self.workspace_output_changing.clear();
    }

    fn capture_changed_cleaned_for_workspace(
        &mut self,
        workspace_path: &Path,
        output: &str,
    ) -> bool {
        let key = Self::workspace_status_tracking_key(workspace_path);
        let previous_digest = self.workspace_status_digests.get(&key);
        let change = evaluate_capture_change(previous_digest, output);
        self.workspace_status_digests
            .insert(key.clone(), change.digest);
        self.workspace_output_changing
            .insert(key, change.changed_cleaned);
        change.changed_cleaned
    }

    fn workspace_output_changing(&self, workspace_path: &Path) -> bool {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_output_changing
            .get(&key)
            .copied()
            .unwrap_or(false)
    }

    fn push_agent_activity_frame(&mut self, changed: bool) {
        if self.agent_activity_frames.len() >= AGENT_ACTIVITY_WINDOW_FRAMES {
            self.agent_activity_frames.pop_front();
        }
        self.agent_activity_frames.push_back(changed);
    }

    fn has_recent_agent_activity(&self) -> bool {
        self.agent_activity_frames
            .iter()
            .copied()
            .any(|changed| changed)
    }

    fn visual_tick_interval(&self) -> Option<Duration> {
        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.as_path());
        if self.status_is_visually_working(
            selected_workspace_path,
            self.selected_workspace_status(),
            true,
        ) {
            return Some(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS));
        }
        None
    }

    fn advance_visual_animation(&mut self) {
        self.fast_animation_frame = self.fast_animation_frame.wrapping_add(1);
    }

    fn status_is_visually_working(
        &self,
        workspace_path: Option<&Path>,
        status: WorkspaceStatus,
        is_selected: bool,
    ) -> bool {
        if is_selected
            && self.interactive.as_ref().is_some_and(|interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
                    < Duration::from_millis(LOCAL_TYPING_SUPPRESS_MS)
            })
        {
            return false;
        }
        match status {
            WorkspaceStatus::Thinking => true,
            WorkspaceStatus::Active => {
                if workspace_path.is_some_and(|path| self.workspace_output_changing(path)) {
                    return true;
                }
                if is_selected {
                    return self.agent_output_changing || self.has_recent_agent_activity();
                }
                false
            }
            _ => false,
        }
    }

    fn is_due_with_tolerance(now: Instant, due_at: Instant) -> bool {
        let tolerance = Duration::from_millis(TICK_EARLY_TOLERANCE_MS);
        let now_with_tolerance = now.checked_add(tolerance).unwrap_or(now);
        now_with_tolerance >= due_at
    }

    fn schedule_next_tick(&mut self) -> Cmd<Msg> {
        let scheduled_at = Instant::now();
        let mut poll_due_at = scheduled_at + self.next_poll_interval();
        let mut source = "adaptive_poll";
        if let Some(interactive_due_at) = self.interactive_poll_due_at
            && interactive_due_at < poll_due_at
        {
            poll_due_at = interactive_due_at;
            source = "interactive_debounce";
        }

        if let Some(existing_poll_due_at) = self.next_poll_due_at
            && existing_poll_due_at <= poll_due_at
        {
            if existing_poll_due_at > scheduled_at {
                poll_due_at = existing_poll_due_at;
                source = "retained_poll";
            } else {
                poll_due_at = scheduled_at;
                source = "overdue_poll";
            }
        }
        self.next_poll_due_at = Some(poll_due_at);

        self.next_visual_due_at = if let Some(interval) = self.visual_tick_interval() {
            let candidate = scheduled_at + interval;
            Some(
                if let Some(existing_visual_due_at) = self.next_visual_due_at {
                    if existing_visual_due_at <= candidate && existing_visual_due_at > scheduled_at
                    {
                        existing_visual_due_at
                    } else {
                        candidate
                    }
                } else {
                    candidate
                },
            )
        } else {
            None
        };

        let mut due_at = poll_due_at;
        let mut trigger = "poll";
        if let Some(visual_due_at) = self.next_visual_due_at
            && visual_due_at < due_at
        {
            due_at = visual_due_at;
            trigger = "visual";
        }

        if let Some(existing_due_at) = self.next_tick_due_at
            && existing_due_at <= due_at
            && existing_due_at > scheduled_at
        {
            self.event_log.log(
                LogEvent::new("tick", "retained")
                    .with_data("source", Value::from(source))
                    .with_data("trigger", Value::from(trigger))
                    .with_data(
                        "interval_ms",
                        Value::from(Self::duration_millis(
                            existing_due_at.saturating_duration_since(scheduled_at),
                        )),
                    )
                    .with_data("pending_depth", Value::from(self.pending_input_depth()))
                    .with_data(
                        "oldest_pending_age_ms",
                        Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                    ),
            );
            return Cmd::None;
        }

        let interval = due_at.saturating_duration_since(scheduled_at);
        let interval_ms = Self::duration_millis(interval);
        self.next_tick_due_at = Some(due_at);
        self.next_tick_interval_ms = Some(interval_ms);
        self.event_log.log(
            LogEvent::new("tick", "scheduled")
                .with_data("source", Value::from(source))
                .with_data("trigger", Value::from(trigger))
                .with_data("interval_ms", Value::from(interval_ms))
                .with_data("pending_depth", Value::from(self.pending_input_depth()))
                .with_data(
                    "oldest_pending_age_ms",
                    Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                ),
        );
        Cmd::tick(interval)
    }

    fn tick_is_due(&self, now: Instant) -> bool {
        let Some(due_at) = self.next_tick_due_at else {
            return true;
        };

        Self::is_due_with_tolerance(now, due_at)
    }
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

pub fn run() -> std::io::Result<()> {
    run_with_event_log(None)
}

pub fn run_with_event_log(event_log_path: Option<PathBuf>) -> std::io::Result<()> {
    run_with_logger(event_log_path, None)
}

pub fn run_with_debug_record(event_log_path: PathBuf, app_start_ts: u64) -> std::io::Result<()> {
    run_with_logger(Some(event_log_path), Some(app_start_ts))
}

fn run_with_logger(
    event_log_path: Option<PathBuf>,
    debug_record_start_ts: Option<u64>,
) -> std::io::Result<()> {
    let event_log: Box<dyn EventLogger> = if let Some(path) = event_log_path {
        Box::new(FileEventLogger::open(&path)?)
    } else {
        Box::new(NullEventLogger)
    };

    if let Some(app_start_ts) = debug_record_start_ts {
        event_log.log(
            LogEvent::new("debug_record", "started")
                .with_data("app_start_ts", Value::from(app_start_ts)),
        );
    }

    let app = if let Some(app_start_ts) = debug_record_start_ts {
        GroveApp::new_with_debug_recorder(event_log, app_start_ts)
    } else {
        GroveApp::new_with_event_logger(event_log)
    };

    App::new(app)
        .screen_mode(ScreenMode::AltScreen)
        .with_mouse()
        .with_budget(FrameBudgetConfig::strict(Duration::from_millis(250)))
        .with_widget_refresh(WidgetRefreshConfig {
            enabled: false,
            ..WidgetRefreshConfig::default()
        })
        .run()
}

#[cfg(test)]
mod tests;
