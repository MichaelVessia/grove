use std::cell::RefCell;
use std::collections::{VecDeque, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arboard::Clipboard;
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
use ftui::text::{Line as FtLine, Span as FtSpan, Text as FtText};
use ftui::widgets::Widget;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::status_line::{StatusItem, StatusLine};
use ftui::{App, Cmd, Model, PackedRgba, ScreenMode, Style};
use serde_json::Value;

use crate::adapters::{
    BootstrapData, CommandGitAdapter, CommandSystemAdapter, CommandTmuxAdapter, DiscoveryState,
    bootstrap_data,
};
use crate::agent_runtime::{
    LaunchRequest, build_launch_plan, poll_interval, session_name_for_workspace, stop_plan,
};
use crate::domain::{AgentType, WorkspaceStatus};
use crate::event_log::{Event as LogEvent, EventLogger, FileEventLogger, NullEventLogger};
#[cfg(test)]
use crate::interactive::render_cursor_overlay;
use crate::interactive::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload,
    render_cursor_overlay_ansi, tmux_send_keys_command,
};
use crate::mouse::{
    clamp_sidebar_ratio, parse_sidebar_ratio, ratio_from_drag, serialize_sidebar_ratio,
};
use crate::preview::{FlashMessage, PreviewState, clear_expired_flash_message, new_flash_message};
use crate::state::{Action, AppState, PaneFocus, UiMode, reduce};
use crate::workspace_lifecycle::{
    BranchMode, CommandGitRunner, CommandSetupScriptRunner, CreateWorkspaceRequest,
    CreateWorkspaceResult, WorkspaceLifecycleError, create_workspace,
};

const DEFAULT_SIDEBAR_WIDTH_PCT: u16 = 33;
const SIDEBAR_RATIO_FILENAME: &str = ".grove-sidebar-width";
const WORKSPACE_LAUNCH_PROMPT_FILENAME: &str = ".grove-prompt";
const HEADER_HEIGHT: u16 = 1;
const STATUS_HEIGHT: u16 = 1;
const DIVIDER_WIDTH: u16 = 1;
const WORKSPACE_ITEM_HEIGHT: u16 = 2;
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
const MAX_PENDING_INPUT_TRACES: usize = 256;
const INTERACTIVE_KEYSTROKE_DEBOUNCE_MS: u64 = 20;
const FAST_ANIMATION_INTERVAL_MS: u64 = 100;
const FAST_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
    green: PackedRgba,
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
        green: PackedRgba::rgb(166, 227, 161),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreviewContentViewport {
    output_x: u16,
    output_y: u16,
    visible_start: usize,
    visible_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchDialogState {
    prompt: String,
    skip_permissions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateDialogState {
    workspace_name: String,
    agent: AgentType,
    base_branch: String,
    existing_branch: String,
    branch_mode: CreateBranchMode,
    focused_field: CreateDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateBranchMode {
    NewBranch,
    ExistingBranch,
}

impl CreateBranchMode {
    fn toggle(self) -> Self {
        match self {
            Self::NewBranch => Self::ExistingBranch,
            Self::ExistingBranch => Self::NewBranch,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::NewBranch => "new",
            Self::ExistingBranch => "existing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateDialogField {
    WorkspaceName,
    BranchInput,
    Agent,
    BranchMode,
}

impl CreateDialogField {
    fn next(self) -> Self {
        match self {
            Self::WorkspaceName => Self::BranchInput,
            Self::BranchInput => Self::Agent,
            Self::Agent => Self::BranchMode,
            Self::BranchMode => Self::WorkspaceName,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::WorkspaceName => Self::BranchMode,
            Self::BranchInput => Self::WorkspaceName,
            Self::Agent => Self::BranchInput,
            Self::BranchMode => Self::Agent,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::BranchInput => "branch",
            Self::Agent => "agent",
            Self::BranchMode => "mode",
        }
    }
}

trait TmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()>;
    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String>;
    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String>;
    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()>;
    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()>;

    fn supports_background_send(&self) -> bool {
        false
    }
}

trait ClipboardAccess {
    fn read_text(&mut self) -> Result<String, String>;
    fn write_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Default)]
struct SystemClipboardAccess {
    clipboard: Option<Clipboard>,
}

impl SystemClipboardAccess {
    fn clipboard(&mut self) -> Result<&mut Clipboard, String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new().map_err(|error| error.to_string())?);
        }

        self.clipboard
            .as_mut()
            .ok_or_else(|| "clipboard unavailable".to_string())
    }
}

impl ClipboardAccess for SystemClipboardAccess {
    fn read_text(&mut self) -> Result<String, String> {
        self.clipboard()?
            .get_text()
            .map_err(|error| error.to_string())
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.clipboard()?
            .set_text(text.to_string())
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Msg {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(PasteEvent),
    Tick,
    Resize { width: u16, height: u16 },
    PreviewPollCompleted(PreviewPollCompletion),
    RefreshWorkspacesCompleted(RefreshWorkspacesCompletion),
    CreateWorkspaceCompleted(CreateWorkspaceCompletion),
    StartAgentCompleted(StartAgentCompletion),
    StopAgentCompleted(StopAgentCompletion),
    InteractiveSendCompleted(InteractiveSendCompletion),
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreviewPollCompletion {
    generation: u64,
    live_capture: Option<LivePreviewCapture>,
    cursor_capture: Option<CursorCapture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LivePreviewCapture {
    session: String,
    include_escape_sequences: bool,
    capture_ms: u64,
    total_ms: u64,
    result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CursorCapture {
    session: String,
    capture_ms: u64,
    result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RefreshWorkspacesCompletion {
    preferred_workspace_name: Option<String>,
    bootstrap: BootstrapData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateWorkspaceCompletion {
    request: CreateWorkspaceRequest,
    result: Result<CreateWorkspaceResult, WorkspaceLifecycleError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartAgentCompletion {
    workspace_name: String,
    session_name: String,
    result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StopAgentCompletion {
    workspace_name: String,
    session_name: String,
    result: Result<(), String>,
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

struct CommandTmuxInput;

impl TmuxInput for CommandTmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()> {
        Self::execute_command(command)
    }

    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        Self::capture_session_output(target_session, scrollback_lines, include_escape_sequences)
    }

    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
        Self::capture_session_cursor_metadata(target_session)
    }

    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        Self::resize_target_session(target_session, target_width, target_height)
    }

    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
        Self::paste_target_session_buffer(target_session, text)
    }

    fn supports_background_send(&self) -> bool {
        true
    }
}

impl CommandTmuxInput {
    fn stderr_or_status(output: &std::process::Output) -> String {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return stderr;
        }

        format!("exit status {}", output.status)
    }

    fn execute_command(command: &[String]) -> std::io::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let output = std::process::Command::new(&command[0])
            .args(&command[1..])
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "tmux command failed: {}; {}",
            command.join(" "),
            Self::stderr_or_status(&output),
        )))
    }

    fn capture_session_output(
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        let mut args = vec!["capture-pane".to_string(), "-p".to_string()];
        if include_escape_sequences {
            args.push("-e".to_string());
        }
        args.push("-t".to_string());
        args.push(target_session.to_string());
        args.push("-S".to_string());
        args.push(format!("-{scrollback_lines}"));

        let output = std::process::Command::new("tmux").args(args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(std::io::Error::other(format!(
                "tmux capture-pane failed for '{target_session}': {stderr}"
            )));
        }

        String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("tmux output utf8 decode failed: {error}"))
        })
    }

    fn capture_session_cursor_metadata(target_session: &str) -> std::io::Result<String> {
        let output = std::process::Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                target_session,
                "#{cursor_flag} #{cursor_x} #{cursor_y} #{pane_width} #{pane_height}",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(std::io::Error::other(format!(
                "tmux cursor metadata failed for '{target_session}': {stderr}"
            )));
        }

        String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("tmux cursor metadata utf8 decode failed: {error}"))
        })
    }

    fn resize_target_session(
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        if target_width == 0 || target_height == 0 {
            return Ok(());
        }

        let width = target_width.to_string();
        let height = target_height.to_string();

        let set_manual_output = std::process::Command::new("tmux")
            .args(["set-option", "-t", target_session, "window-size", "manual"])
            .output();
        let set_manual_error = match set_manual_output {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(Self::stderr_or_status(&output)),
            Err(error) => Some(error.to_string()),
        };

        let resize_window = std::process::Command::new("tmux")
            .args([
                "resize-window",
                "-t",
                target_session,
                "-x",
                &width,
                "-y",
                &height,
            ])
            .output()?;
        if resize_window.status.success() {
            return Ok(());
        }

        let resize_pane = std::process::Command::new("tmux")
            .args([
                "resize-pane",
                "-t",
                target_session,
                "-x",
                &width,
                "-y",
                &height,
            ])
            .output()?;
        if resize_pane.status.success() {
            return Ok(());
        }

        let resize_window_error = String::from_utf8_lossy(&resize_window.stderr)
            .trim()
            .to_string();
        let resize_pane_error = String::from_utf8_lossy(&resize_pane.stderr)
            .trim()
            .to_string();
        let set_manual_suffix =
            set_manual_error.map_or_else(String::new, |error| format!("; set-option={error}"));
        Err(std::io::Error::other(format!(
            "tmux resize failed for '{target_session}': resize-window={resize_window_error}; resize-pane={resize_pane_error}{set_manual_suffix}"
        )))
    }

    fn paste_target_session_buffer(target_session: &str, text: &str) -> std::io::Result<()> {
        let mut load_buffer = std::process::Command::new("tmux");
        load_buffer.arg("load-buffer").arg("-");
        load_buffer.stdin(std::process::Stdio::piped());
        let mut load_child = load_buffer.spawn()?;
        if let Some(stdin) = load_child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
        }
        let load_status = load_child.wait()?;
        if !load_status.success() {
            return Err(std::io::Error::other(format!(
                "tmux load-buffer failed for '{target_session}': exit status {load_status}"
            )));
        }

        let paste_output = std::process::Command::new("tmux")
            .args(["paste-buffer", "-t", target_session])
            .output()?;
        if paste_output.status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "tmux paste-buffer failed: {}",
            Self::stderr_or_status(&paste_output),
        )))
    }
}

fn parse_cursor_flag(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "on" | "true" => Some(true),
        "0" | "off" | "false" => Some(false),
        _ => None,
    }
}

fn parse_cursor_metadata(raw: &str) -> Option<CursorMetadata> {
    let mut fields = raw.split_whitespace();
    let cursor_visible = parse_cursor_flag(fields.next()?)?;
    let cursor_col = fields.next()?.parse::<u16>().ok()?;
    let cursor_row = fields.next()?.parse::<u16>().ok()?;
    let pane_width = fields.next()?.parse::<u16>().ok()?;
    let pane_height = fields.next()?.parse::<u16>().ok()?;
    if fields.next().is_some() {
        return None;
    }

    Some(CursorMetadata {
        cursor_visible,
        cursor_col,
        cursor_row,
        pane_width,
        pane_height,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct AnsiStyleState {
    fg: Option<PackedRgba>,
    bg: Option<PackedRgba>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    strikethrough: bool,
}

impl AnsiStyleState {
    fn into_style(self) -> Option<Style> {
        let mut style = Style::new();

        if let Some(fg) = self.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }
        if self.bold {
            style = style.bold();
        }
        if self.dim {
            style = style.dim();
        }
        if self.italic {
            style = style.italic();
        }
        if self.underline {
            style = style.underline();
        }
        if self.blink {
            style = style.blink();
        }
        if self.reverse {
            style = style.reverse();
        }
        if self.strikethrough {
            style = style.strikethrough();
        }

        if style == Style::new() {
            return None;
        }

        Some(style)
    }
}

fn ansi_16_color(index: u8) -> PackedRgba {
    match index {
        0 => PackedRgba::rgb(0, 0, 0),
        1 => PackedRgba::rgb(205, 49, 49),
        2 => PackedRgba::rgb(13, 188, 121),
        3 => PackedRgba::rgb(229, 229, 16),
        4 => PackedRgba::rgb(36, 114, 200),
        5 => PackedRgba::rgb(188, 63, 188),
        6 => PackedRgba::rgb(17, 168, 205),
        7 => PackedRgba::rgb(229, 229, 229),
        8 => PackedRgba::rgb(102, 102, 102),
        9 => PackedRgba::rgb(241, 76, 76),
        10 => PackedRgba::rgb(35, 209, 139),
        11 => PackedRgba::rgb(245, 245, 67),
        12 => PackedRgba::rgb(59, 142, 234),
        13 => PackedRgba::rgb(214, 112, 214),
        14 => PackedRgba::rgb(41, 184, 219),
        _ => PackedRgba::rgb(255, 255, 255),
    }
}

fn ansi_256_color(index: u8) -> PackedRgba {
    if index < 16 {
        return ansi_16_color(index);
    }

    if index <= 231 {
        let value = usize::from(index - 16);
        let r = value / 36;
        let g = (value % 36) / 6;
        let b = value % 6;
        let table = [0u8, 95, 135, 175, 215, 255];
        return PackedRgba::rgb(table[r], table[g], table[b]);
    }

    let gray = 8u8.saturating_add((index - 232).saturating_mul(10));
    PackedRgba::rgb(gray, gray, gray)
}

fn parse_sgr_extended_color(params: &[i32], start: usize) -> Option<(PackedRgba, usize)> {
    let mode = *params.get(start)?;
    match mode {
        5 => {
            let value = *params.get(start.saturating_add(1))?;
            let palette = u8::try_from(value).ok()?;
            Some((ansi_256_color(palette), 2))
        }
        2 => {
            let r = u8::try_from(*params.get(start.saturating_add(1))?).ok()?;
            let g = u8::try_from(*params.get(start.saturating_add(2))?).ok()?;
            let b = u8::try_from(*params.get(start.saturating_add(3))?).ok()?;
            Some((PackedRgba::rgb(r, g, b), 4))
        }
        _ => None,
    }
}

fn apply_sgr_codes(raw_params: &str, state: &mut AnsiStyleState) {
    let params: Vec<i32> = if raw_params.is_empty() {
        vec![0]
    } else {
        raw_params
            .split(';')
            .map(|value| {
                if value.is_empty() {
                    0
                } else {
                    value.parse::<i32>().unwrap_or(-1)
                }
            })
            .collect()
    };

    let mut index = 0usize;
    while index < params.len() {
        match params[index] {
            0 => *state = AnsiStyleState::default(),
            1 => state.bold = true,
            2 => state.dim = true,
            3 => state.italic = true,
            4 => state.underline = true,
            5 => state.blink = true,
            7 => state.reverse = true,
            9 => state.strikethrough = true,
            22 => {
                state.bold = false;
                state.dim = false;
            }
            23 => state.italic = false,
            24 => state.underline = false,
            25 => state.blink = false,
            27 => state.reverse = false,
            29 => state.strikethrough = false,
            30..=37 => {
                if let Ok(code) = u8::try_from(params[index] - 30) {
                    state.fg = Some(ansi_16_color(code));
                }
            }
            90..=97 => {
                if let Ok(code) = u8::try_from(params[index] - 90) {
                    state.fg = Some(ansi_16_color(code.saturating_add(8)));
                }
            }
            40..=47 => {
                if let Ok(code) = u8::try_from(params[index] - 40) {
                    state.bg = Some(ansi_16_color(code));
                }
            }
            100..=107 => {
                if let Ok(code) = u8::try_from(params[index] - 100) {
                    state.bg = Some(ansi_16_color(code.saturating_add(8)));
                }
            }
            38 => {
                if let Some((color, consumed)) =
                    parse_sgr_extended_color(&params, index.saturating_add(1))
                {
                    state.fg = Some(color);
                    index = index.saturating_add(consumed);
                }
            }
            48 => {
                if let Some((color, consumed)) =
                    parse_sgr_extended_color(&params, index.saturating_add(1))
                {
                    state.bg = Some(color);
                    index = index.saturating_add(consumed);
                }
            }
            39 => state.fg = None,
            49 => state.bg = None,
            _ => {}
        }

        index = index.saturating_add(1);
    }
}

fn sgr_params_affect_background(raw_params: &str) -> bool {
    if raw_params.is_empty() {
        return true;
    }

    raw_params.split(';').any(|value| {
        if value.is_empty() {
            return true;
        }

        let Ok(code) = value.parse::<i32>() else {
            return false;
        };

        matches!(code, 0 | 49 | 40..=47 | 100..=107 | 48)
    })
}

fn inject_selection_background_ansi(
    line: &str,
    start_col: usize,
    end_col: Option<usize>,
    selection_bg: PackedRgba,
) -> String {
    let selection_bg_ansi = format!(
        "\u{1b}[48;2;{};{};{}m",
        selection_bg.r(),
        selection_bg.g(),
        selection_bg.b()
    );

    let mut rendered = String::with_capacity(line.len().saturating_add(64));
    let mut chars = line.chars().peekable();
    let mut visible_index = 0usize;
    let mut in_selection = false;

    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            rendered.push(character);
            let Some(next) = chars.next() else {
                break;
            };
            rendered.push(next);

            match next {
                '[' => {
                    let mut params = String::new();
                    let mut final_char = None;
                    while let Some(value) = chars.next() {
                        rendered.push(value);
                        if ('\u{40}'..='\u{7e}').contains(&value) {
                            final_char = Some(value);
                            break;
                        }
                        params.push(value);
                    }

                    if in_selection
                        && final_char == Some('m')
                        && sgr_params_affect_background(&params)
                    {
                        rendered.push_str(&selection_bg_ansi);
                    }
                }
                ']' => {
                    while let Some(value) = chars.next() {
                        rendered.push(value);
                        if value == '\u{7}' {
                            break;
                        }
                        if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                            rendered.push('\\');
                            break;
                        }
                    }
                }
                'P' | 'X' | '^' | '_' => {
                    while let Some(value) = chars.next() {
                        rendered.push(value);
                        if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                            rendered.push('\\');
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }

        let in_range = match end_col {
            Some(end_col) => visible_index >= start_col && visible_index <= end_col,
            None => visible_index >= start_col,
        };

        if in_range && !in_selection {
            rendered.push_str(&selection_bg_ansi);
            in_selection = true;
        } else if !in_range && in_selection {
            rendered.push_str("\u{1b}[49m");
            in_selection = false;
        }

        rendered.push(character);
        visible_index = visible_index.saturating_add(1);
    }

    if in_selection {
        rendered.push_str("\u{1b}[49m");
    }

    rendered
}

fn ansi_line_to_styled_line(line: &str) -> FtLine {
    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    let mut buffer = String::new();
    let mut state = AnsiStyleState::default();
    let mut chars = line.chars().peekable();

    let flush = |buffer: &mut String, spans: &mut Vec<FtSpan<'static>>, state: AnsiStyleState| {
        if buffer.is_empty() {
            return;
        }
        let content = std::mem::take(buffer);
        if let Some(style) = state.into_style() {
            spans.push(FtSpan::styled(content, style));
        } else {
            spans.push(FtSpan::raw(content));
        }
    };

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            buffer.push(character);
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                let mut params = String::new();
                let mut final_char: Option<char> = None;
                while let Some(value) = chars.next() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        final_char = Some(value);
                        break;
                    }
                    params.push(value);
                }
                if final_char == Some('m') {
                    flush(&mut buffer, &mut spans, state);
                    apply_sgr_codes(&params, &mut state);
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

    flush(&mut buffer, &mut spans, state);

    if spans.is_empty() {
        return FtLine::raw("");
    }

    FtLine::from_spans(spans)
}

fn default_sidebar_ratio_path() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(SIDEBAR_RATIO_FILENAME),
        Err(_) => PathBuf::from(SIDEBAR_RATIO_FILENAME),
    }
}

fn load_sidebar_ratio(path: &Path) -> u16 {
    let Ok(raw) = fs::read_to_string(path) else {
        return DEFAULT_SIDEBAR_WIDTH_PCT;
    };

    parse_sidebar_ratio(&raw).unwrap_or(DEFAULT_SIDEBAR_WIDTH_PCT)
}

fn read_workspace_launch_prompt(workspace_path: &Path) -> Option<String> {
    let raw = fs::read_to_string(workspace_path.join(WORKSPACE_LAUNCH_PROMPT_FILENAME)).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key_event) => Self::Key(key_event),
            Event::Mouse(mouse_event) => Self::Mouse(mouse_event),
            Event::Paste(paste_event) => Self::Paste(paste_event),
            Event::Tick => Self::Tick,
            Event::Resize { width, height } => Self::Resize { width, height },
            _ => Self::Noop,
        }
    }
}

struct GroveApp {
    repo_name: String,
    state: AppState,
    discovery_state: DiscoveryState,
    preview: PreviewState,
    flash: Option<FlashMessage>,
    interactive: Option<InteractiveState>,
    action_mapper: ActionMapper,
    launch_dialog: Option<LaunchDialogState>,
    create_dialog: Option<CreateDialogState>,
    tmux_input: Box<dyn TmuxInput>,
    clipboard: Box<dyn ClipboardAccess>,
    last_tmux_error: Option<String>,
    output_changing: bool,
    agent_output_changing: bool,
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
    create_in_flight: bool,
    start_in_flight: bool,
    stop_in_flight: bool,
    deferred_cmds: Vec<Cmd<Msg>>,
}

impl GroveApp {
    fn new_with_event_logger(event_log: Box<dyn EventLogger>) -> Self {
        let bootstrap = bootstrap_data(
            &CommandGitAdapter,
            &CommandTmuxAdapter,
            &CommandSystemAdapter,
        );
        Self::from_parts(
            bootstrap,
            Box::new(CommandTmuxInput),
            default_sidebar_ratio_path(),
            event_log,
            None,
        )
    }

    fn new_with_debug_recorder(event_log: Box<dyn EventLogger>, app_start_ts: u64) -> Self {
        let bootstrap = bootstrap_data(
            &CommandGitAdapter,
            &CommandTmuxAdapter,
            &CommandSystemAdapter,
        );
        Self::from_parts(
            bootstrap,
            Box::new(CommandTmuxInput),
            default_sidebar_ratio_path(),
            event_log,
            Some(app_start_ts),
        )
    }

    fn from_parts(
        bootstrap: BootstrapData,
        tmux_input: Box<dyn TmuxInput>,
        sidebar_ratio_path: PathBuf,
        event_log: Box<dyn EventLogger>,
        debug_record_start_ts: Option<u64>,
    ) -> Self {
        Self::from_parts_with_clipboard(
            bootstrap,
            tmux_input,
            Box::new(SystemClipboardAccess::default()),
            sidebar_ratio_path,
            event_log,
            debug_record_start_ts,
        )
    }

    fn from_parts_with_clipboard(
        bootstrap: BootstrapData,
        tmux_input: Box<dyn TmuxInput>,
        clipboard: Box<dyn ClipboardAccess>,
        sidebar_ratio_path: PathBuf,
        event_log: Box<dyn EventLogger>,
        debug_record_start_ts: Option<u64>,
    ) -> Self {
        let sidebar_width_pct = load_sidebar_ratio(&sidebar_ratio_path);
        let mapper_config = KeybindingConfig::from_env().with_sequence_config(
            KeySequenceConfig::from_env()
                .disable_sequences()
                .validated(),
        );
        let mut app = Self {
            repo_name: bootstrap.repo_name,
            state: AppState::new(bootstrap.workspaces),
            discovery_state: bootstrap.discovery_state,
            preview: PreviewState::new(),
            flash: None,
            interactive: None,
            action_mapper: ActionMapper::new(mapper_config),
            launch_dialog: None,
            create_dialog: None,
            tmux_input,
            clipboard,
            last_tmux_error: None,
            output_changing: false,
            agent_output_changing: false,
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

    fn selected_status_hint(&self) -> &'static str {
        match self
            .state
            .selected_workspace()
            .map(|workspace| workspace.status)
        {
            Some(WorkspaceStatus::Main) => "main worktree",
            Some(WorkspaceStatus::Idle) => "idle",
            Some(WorkspaceStatus::Active) => "active",
            Some(WorkspaceStatus::Thinking) => "thinking",
            Some(WorkspaceStatus::Waiting) => "waiting",
            Some(WorkspaceStatus::Done) => "done",
            Some(WorkspaceStatus::Error) => "error",
            Some(WorkspaceStatus::Unsupported) => "unsupported",
            Some(WorkspaceStatus::Unknown) => "unknown",
            None => "none",
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

    fn selected_workspace_name(&self) -> Option<String> {
        self.state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
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

    fn show_flash(&mut self, text: impl Into<String>, is_error: bool) {
        let message = text.into();
        self.event_log.log(
            LogEvent::new("flash", "flash_shown")
                .with_data("text", Value::from(message.clone()))
                .with_data("is_error", Value::from(is_error)),
        );
        self.flash = Some(new_flash_message(message, is_error, Instant::now()));
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

        self.event_log.log(
            LogEvent::new("frame", "rendered")
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
                .with_data(
                    "frame_lines",
                    Value::Array(lines.into_iter().map(Value::from).collect()),
                ),
        );
    }

    #[cfg(test)]
    fn unsafe_label(&self) -> &'static str {
        if self.launch_skip_permissions {
            "on"
        } else {
            "off"
        }
    }

    #[cfg(test)]
    fn status_bar_line(&self) -> String {
        if let Some(flash) = &self.flash {
            if flash.is_error {
                return format!("Status: error: {}", flash.text);
            }
            return format!("Status: {}", flash.text);
        }

        match &self.discovery_state {
            DiscoveryState::Error(message) => format!("Status: discovery error ({message})"),
            DiscoveryState::Empty => "Status: no worktrees found".to_string(),
            DiscoveryState::Ready => {
                if let Some(dialog) = &self.create_dialog {
                    let branch_value = match dialog.branch_mode {
                        CreateBranchMode::NewBranch => dialog.base_branch.replace('\n', "\\n"),
                        CreateBranchMode::ExistingBranch => {
                            dialog.existing_branch.replace('\n', "\\n")
                        }
                    };
                    return format!(
                        "Status: new workspace, field={}, mode={}, agent={}, branch=\"{}\", name=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.branch_mode.label(),
                        dialog.agent.label(),
                        branch_value,
                        dialog.workspace_name
                    );
                }
                if let Some(dialog) = &self.launch_dialog {
                    return format!(
                        "Status: start agent, unsafe={}, prompt=\"{}\"",
                        if dialog.skip_permissions { "on" } else { "off" },
                        dialog.prompt.replace('\n', "\\n"),
                    );
                }
                if self.interactive.is_some() {
                    if let Some(message) = &self.last_tmux_error {
                        return format!(
                            "Status: INSERT, unsafe={}, tmux error: {message}",
                            self.unsafe_label()
                        );
                    }
                    return format!("Status: INSERT, unsafe={}", self.unsafe_label());
                }

                match self.state.mode {
                    UiMode::List => format!(
                        "Status: list, selected={}, unsafe={}",
                        self.selected_status_hint(),
                        self.unsafe_label()
                    ),
                    UiMode::Preview => format!(
                        "Status: preview, autoscroll={}, offset={}, split={}%, unsafe={}",
                        if self.preview.auto_scroll {
                            "on"
                        } else {
                            "off"
                        },
                        self.preview.offset,
                        self.sidebar_width_pct,
                        self.unsafe_label(),
                    ),
                }
            }
        }
    }

    fn keybind_hints_line(&self) -> &'static str {
        if self.create_dialog.is_some() {
            return "Tab/Shift+Tab field, Left/Right toggle, Enter create, Esc cancel";
        }
        if self.launch_dialog.is_some() {
            return "Type prompt, Tab unsafe, Enter start, Esc cancel";
        }
        if self.interactive.is_some() {
            return "Esc Esc / Ctrl+\\ exit, Alt+C copy, Alt+V paste";
        }
        if self.state.mode == UiMode::Preview {
            return "j/k scroll, PgUp/PgDn, G bottom, Esc list, q quit";
        }

        "j/k move, Enter open, n new, s start, x stop, q quit"
    }

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "Workspace: {}\nBranch: {}\nPath: {}\nAgent: {}\nStatus: {}\nOrphaned session: {}",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display(),
                    workspace.agent.label(),
                    self.selected_status_hint(),
                    if workspace.is_orphaned { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn modal_open(&self) -> bool {
        self.launch_dialog.is_some() || self.create_dialog.is_some()
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

    fn selected_session_for_live_preview(&self) -> Option<(String, bool)> {
        let workspace = self.state.selected_workspace()?;
        if workspace.is_main {
            return None;
        }

        if workspace.status.has_session() {
            return Some((session_name_for_workspace(&workspace.name), true));
        }

        None
    }

    fn interactive_target_session(&self) -> Option<String> {
        self.interactive
            .as_ref()
            .map(|state| state.target_session.clone())
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
                self.output_changing = false;
                self.agent_output_changing = false;
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
                self.show_flash("preview capture failed", true);
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
                .with_data("cursor_col", Value::from(metadata.cursor_col)),
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
        let live_preview = self.selected_session_for_live_preview();
        let cursor_session = self.interactive_target_session();

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
            self.output_changing = false;
            self.agent_output_changing = false;
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

            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation,
                live_capture,
                cursor_capture,
            })
        })
    }

    fn poll_preview(&mut self) {
        if !self.tmux_input.supports_background_send() {
            self.poll_preview_sync();
            return;
        }

        let live_preview = self.selected_session_for_live_preview();
        let cursor_session = self.interactive_target_session();

        if live_preview.is_none() && cursor_session.is_none() {
            self.output_changing = false;
            self.agent_output_changing = false;
            self.refresh_preview_summary();
            return;
        }

        self.poll_generation = self.poll_generation.saturating_add(1);
        self.queue_cmd(self.schedule_async_preview_poll(
            self.poll_generation,
            live_preview,
            cursor_session,
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

        if let Some(live_capture) = completion.live_capture {
            self.apply_live_preview_capture(
                &live_capture.session,
                live_capture.include_escape_sequences,
                live_capture.capture_ms,
                live_capture.total_ms,
                live_capture.result,
            );
        } else {
            self.output_changing = false;
            self.agent_output_changing = false;
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

    fn persist_sidebar_ratio(&mut self) {
        if let Err(error) = fs::write(
            &self.sidebar_ratio_path,
            serialize_sidebar_ratio(self.sidebar_width_pct),
        ) {
            self.last_tmux_error = Some(format!("sidebar ratio persist failed: {error}"));
        }
    }

    fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.preview.jump_to_bottom();
            self.clear_preview_selection();
            self.poll_preview();
        }
    }

    fn is_quit_key(key_event: &KeyEvent) -> bool {
        matches!(
            key_event.code,
            KeyCode::Char('q')
                if key_event.kind == KeyEventKind::Press && key_event.modifiers.is_empty()
        )
    }

    fn keybinding_task_running(&self) -> bool {
        self.refresh_in_flight
            || self.create_in_flight
            || self.start_in_flight
            || self.stop_in_flight
    }

    fn keybinding_input_nonempty(&self) -> bool {
        if let Some(dialog) = self.launch_dialog.as_ref() {
            return !dialog.prompt.is_empty();
        }
        if let Some(dialog) = self.create_dialog.as_ref() {
            return !dialog.workspace_name.is_empty()
                || !dialog.base_branch.is_empty()
                || !dialog.existing_branch.is_empty();
        }

        false
    }

    fn keybinding_state(&self) -> KeybindingAppState {
        KeybindingAppState::new()
            .with_input(self.keybinding_input_nonempty())
            .with_task(self.keybinding_task_running())
            .with_modal(self.modal_open())
    }

    fn apply_keybinding_action(&mut self, action: KeybindingAction) -> bool {
        match action {
            KeybindingAction::DismissModal => {
                if self.create_dialog.is_some() {
                    self.log_dialog_event("create", "dialog_cancelled");
                    self.create_dialog = None;
                } else if self.launch_dialog.is_some() {
                    self.log_dialog_event("launch", "dialog_cancelled");
                    self.launch_dialog = None;
                }
                false
            }
            KeybindingAction::ClearInput => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.prompt.clear();
                    return false;
                }
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => dialog.workspace_name.clear(),
                        CreateDialogField::BranchInput => match dialog.branch_mode {
                            CreateBranchMode::NewBranch => dialog.base_branch.clear(),
                            CreateBranchMode::ExistingBranch => dialog.existing_branch.clear(),
                        },
                        CreateDialogField::Agent | CreateDialogField::BranchMode => {}
                    }
                }
                false
            }
            KeybindingAction::CancelTask => {
                self.show_flash("cannot cancel running lifecycle task", true);
                false
            }
            KeybindingAction::Quit | KeybindingAction::HardQuit => true,
            KeybindingAction::SoftQuit => !self.keybinding_task_running(),
            KeybindingAction::CloseOverlay
            | KeybindingAction::ToggleTreeView
            | KeybindingAction::Bell
            | KeybindingAction::PassThrough => false,
        }
    }

    fn can_enter_interactive(&self) -> bool {
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };

        !workspace.is_main && workspace.status.has_session()
    }

    fn enter_interactive(&mut self, now: Instant) -> bool {
        if !self.can_enter_interactive() {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };

        let session_name = session_name_for_workspace(&workspace.name);
        self.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            session_name,
            now,
            self.viewport_height,
            self.viewport_width,
        ));
        self.interactive_poll_due_at = None;
        self.last_tmux_error = None;
        self.state.mode = UiMode::Preview;
        self.state.focus = PaneFocus::Preview;
        self.clear_preview_selection();
        self.sync_interactive_session_geometry();
        self.poll_preview();
        true
    }

    fn can_start_selected_workspace(&self) -> bool {
        if self.start_in_flight {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        if workspace.is_main || !workspace.supported_agent {
            return false;
        }

        matches!(
            workspace.status,
            WorkspaceStatus::Idle
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
                | WorkspaceStatus::Unknown
        )
    }

    fn open_start_dialog(&mut self) {
        if self.start_in_flight {
            self.show_flash("agent start already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_flash("no workspace selected", true);
            return;
        };
        if workspace.is_main {
            self.show_flash("cannot start agent in main workspace", true);
            return;
        }
        if !workspace.supported_agent {
            self.show_flash("unsupported workspace agent marker", true);
            return;
        }
        if workspace.status.is_running() {
            self.show_flash("agent already running", true);
            return;
        }
        if !self.can_start_selected_workspace() {
            self.show_flash("workspace cannot be started", true);
            return;
        }

        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let skip_permissions = self.launch_skip_permissions;
        self.launch_dialog = Some(LaunchDialogState {
            prompt: prompt.clone(),
            skip_permissions,
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                (
                    "prompt_len".to_string(),
                    Value::from(u64::try_from(prompt.len()).unwrap_or(u64::MAX)),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(skip_permissions),
                ),
            ],
        );
        self.last_tmux_error = None;
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

    fn open_create_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let default_agent = self
            .state
            .selected_workspace()
            .map_or(AgentType::Claude, |workspace| workspace.agent);
        self.create_dialog = Some(CreateDialogState {
            workspace_name: String::new(),
            agent: default_agent,
            base_branch: self.selected_base_branch(),
            existing_branch: String::new(),
            branch_mode: CreateBranchMode::NewBranch,
            focused_field: CreateDialogField::WorkspaceName,
        });
        self.log_dialog_event_with_fields(
            "create",
            "dialog_opened",
            [
                ("agent".to_string(), Value::from(default_agent.label())),
                (
                    "branch_mode".to_string(),
                    Value::from(CreateBranchMode::NewBranch.label()),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn toggle_create_dialog_agent(dialog: &mut CreateDialogState) {
        dialog.agent = match dialog.agent {
            AgentType::Claude => AgentType::Codex,
            AgentType::Codex => AgentType::Claude,
        };
    }

    fn toggle_create_dialog_branch_mode(dialog: &mut CreateDialogState) {
        dialog.branch_mode = dialog.branch_mode.toggle();
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

    fn refresh_workspaces(&mut self, preferred_workspace_name: Option<String>) {
        if !self.tmux_input.supports_background_send() {
            self.refresh_workspaces_sync(preferred_workspace_name);
            return;
        }

        if self.refresh_in_flight {
            return;
        }

        let fallback_name = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.name.clone());
        let target_name = preferred_workspace_name.or(fallback_name);
        self.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let bootstrap = bootstrap_data(
                &CommandGitAdapter,
                &CommandTmuxAdapter,
                &CommandSystemAdapter,
            );
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_name: target_name,
                bootstrap,
            })
        }));
    }

    fn refresh_workspaces_sync(&mut self, preferred_workspace_name: Option<String>) {
        let fallback_name = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.name.clone());
        let target_name = preferred_workspace_name.or(fallback_name);
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;
        let bootstrap = bootstrap_data(
            &CommandGitAdapter,
            &CommandTmuxAdapter,
            &CommandSystemAdapter,
        );

        self.repo_name = bootstrap.repo_name;
        self.discovery_state = bootstrap.discovery_state;
        self.state = AppState::new(bootstrap.workspaces);
        if let Some(name) = target_name
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.name == name)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.poll_preview();
    }

    fn apply_refresh_workspaces_completion(&mut self, completion: RefreshWorkspacesCompletion) {
        let previous_mode = self.state.mode;
        let previous_focus = self.state.focus;

        self.repo_name = completion.bootstrap.repo_name;
        self.discovery_state = completion.bootstrap.discovery_state;
        self.state = AppState::new(completion.bootstrap.workspaces);
        if let Some(name) = completion.preferred_workspace_name
            && let Some(index) = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.name == name)
        {
            self.state.selected_index = index;
        }
        self.state.mode = previous_mode;
        self.state.focus = previous_focus;
        self.refresh_in_flight = false;
        self.poll_preview();
    }

    fn confirm_create_dialog(&mut self) {
        if self.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog.as_ref().cloned() else {
            return;
        };
        let branch_value = match dialog.branch_mode {
            CreateBranchMode::NewBranch => dialog.base_branch.clone(),
            CreateBranchMode::ExistingBranch => dialog.existing_branch.clone(),
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
                (
                    "branch_mode".to_string(),
                    Value::from(dialog.branch_mode.label()),
                ),
                ("branch_value".to_string(), Value::from(branch_value)),
            ],
        );

        let workspace_name = dialog.workspace_name.trim().to_string();
        let branch_mode = match dialog.branch_mode {
            CreateBranchMode::NewBranch => BranchMode::NewBranch {
                base_branch: dialog.base_branch.trim().to_string(),
            },
            CreateBranchMode::ExistingBranch => BranchMode::ExistingBranch {
                existing_branch: dialog.existing_branch.trim().to_string(),
            },
        };
        let request = CreateWorkspaceRequest {
            workspace_name: workspace_name.clone(),
            branch_mode,
            agent: dialog.agent,
        };

        if let Err(error) = request.validate() {
            self.show_flash(Self::workspace_lifecycle_error_message(&error), true);
            return;
        }

        if !self.tmux_input.supports_background_send() {
            let result = match std::env::current_dir() {
                Ok(repo_root) => {
                    let git = CommandGitRunner;
                    let setup = CommandSetupScriptRunner;
                    create_workspace(&repo_root, &request, &git, &setup)
                }
                Err(_) => Err(WorkspaceLifecycleError::Io(
                    "cannot resolve current directory".to_string(),
                )),
            };
            self.apply_create_workspace_completion(CreateWorkspaceCompletion { request, result });
            return;
        }

        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = match std::env::current_dir() {
                Ok(repo_root) => {
                    let git = CommandGitRunner;
                    let setup = CommandSetupScriptRunner;
                    create_workspace(&repo_root, &request, &git, &setup)
                }
                Err(_) => Err(WorkspaceLifecycleError::Io(
                    "cannot resolve current directory".to_string(),
                )),
            };
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion { request, result })
        }));
    }

    fn apply_create_workspace_completion(&mut self, completion: CreateWorkspaceCompletion) {
        self.create_in_flight = false;
        let workspace_name = completion.request.workspace_name;
        match completion.result {
            Ok(result) => {
                self.create_dialog = None;
                self.refresh_workspaces(Some(workspace_name.clone()));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if result.warnings.is_empty() {
                    self.show_flash(format!("workspace '{}' created", workspace_name), false);
                } else if let Some(first_warning) = result.warnings.first() {
                    self.show_flash(
                        format!(
                            "workspace '{}' created, warning: {}",
                            workspace_name, first_warning
                        ),
                        true,
                    );
                }
            }
            Err(error) => {
                self.show_flash(
                    format!(
                        "workspace create failed: {}",
                        Self::workspace_lifecycle_error_message(&error)
                    ),
                    true,
                );
            }
        }
    }

    fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.create_in_flight {
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.create_dialog = None;
            }
            KeyCode::Enter => {
                self.confirm_create_dialog();
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Left | KeyCode::Right => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::Agent => Self::toggle_create_dialog_agent(dialog),
                        CreateDialogField::BranchMode => {
                            Self::toggle_create_dialog_branch_mode(dialog);
                        }
                        CreateDialogField::WorkspaceName | CreateDialogField::BranchInput => {}
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.workspace_name.pop();
                        }
                        CreateDialogField::BranchInput => match dialog.branch_mode {
                            CreateBranchMode::NewBranch => {
                                dialog.base_branch.pop();
                            }
                            CreateBranchMode::ExistingBranch => {
                                dialog.existing_branch.pop();
                            }
                        },
                        CreateDialogField::Agent | CreateDialogField::BranchMode => {}
                    }
                }
            }
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            if character.is_ascii_alphanumeric()
                                || character == '-'
                                || character == '_'
                            {
                                dialog.workspace_name.push(character);
                            }
                        }
                        CreateDialogField::BranchInput => {
                            if !character.is_control() {
                                match dialog.branch_mode {
                                    CreateBranchMode::NewBranch => {
                                        dialog.base_branch.push(character)
                                    }
                                    CreateBranchMode::ExistingBranch => {
                                        dialog.existing_branch.push(character);
                                    }
                                }
                            }
                        }
                        CreateDialogField::Agent => {
                            if character == ' ' {
                                Self::toggle_create_dialog_agent(dialog);
                            }
                        }
                        CreateDialogField::BranchMode => {
                            if character == ' ' {
                                Self::toggle_create_dialog_branch_mode(dialog);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn start_selected_workspace_agent_with_options(
        &mut self,
        prompt: Option<String>,
        skip_permissions: bool,
    ) {
        if self.start_in_flight {
            return;
        }

        if !self.can_start_selected_workspace() {
            self.show_flash("workspace cannot be started", true);
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            self.show_flash("no workspace selected", true);
            return;
        };

        let request = LaunchRequest {
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            agent: workspace.agent,
            prompt,
            skip_permissions,
        };
        let launch_plan = build_launch_plan(&request);
        let workspace_name = request.workspace_name.clone();
        let session_name = session_name_for_workspace(&request.workspace_name);

        if !self.tmux_input.supports_background_send() {
            if let Some(script) = &launch_plan.launcher_script
                && let Err(error) = fs::write(&script.path, &script.contents)
            {
                self.last_tmux_error = Some(format!("launcher script write failed: {error}"));
                self.show_flash("launcher script write failed", true);
                return;
            }

            for command in &launch_plan.pre_launch_cmds {
                if let Err(error) = self.execute_tmux_command(command) {
                    self.last_tmux_error = Some(error.to_string());
                    self.show_flash("agent start failed", true);
                    return;
                }
            }

            if let Err(error) = self.execute_tmux_command(&launch_plan.launch_cmd) {
                self.last_tmux_error = Some(error.to_string());
                self.show_flash("agent start failed", true);
                return;
            }

            self.apply_start_agent_completion(StartAgentCompletion {
                workspace_name,
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
                if let Some(workspace) = self
                    .state
                    .workspaces
                    .iter_mut()
                    .find(|workspace| workspace.name == completion.workspace_name)
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
                self.show_flash("agent started", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_flash("agent start failed", true);
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
            ],
        );

        self.launch_skip_permissions = dialog.skip_permissions;
        let prompt = if dialog.prompt.trim().is_empty() {
            None
        } else {
            Some(dialog.prompt.trim().to_string())
        };
        self.start_selected_workspace_agent_with_options(prompt, dialog.skip_permissions);
    }

    fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("launch", "dialog_cancelled");
                self.launch_dialog = None;
            }
            KeyCode::Enter => {
                self.confirm_start_dialog();
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.skip_permissions = !dialog.skip_permissions;
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.prompt.pop();
                }
            }
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.prompt.push(character);
                }
            }
            _ => {}
        }
    }

    fn can_stop_selected_workspace(&self) -> bool {
        if self.stop_in_flight {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        !workspace.is_main && workspace.status.has_session()
    }

    fn stop_selected_workspace_agent(&mut self) {
        if self.stop_in_flight {
            return;
        }

        if !self.can_stop_selected_workspace() {
            self.show_flash("no agent running", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_flash("no workspace selected", true);
            return;
        };
        let workspace_name = workspace.name.clone();
        let session_name = session_name_for_workspace(&workspace_name);
        let stop_commands = stop_plan(&session_name);

        if !self.tmux_input.supports_background_send() {
            for command in &stop_commands {
                if let Err(error) = self.execute_tmux_command(command) {
                    self.last_tmux_error = Some(error.to_string());
                    self.show_flash("agent stop failed", true);
                    return;
                }
            }

            self.apply_stop_agent_completion(StopAgentCompletion {
                workspace_name,
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
                    .find(|workspace| workspace.name == completion.workspace_name)
                {
                    workspace.status = WorkspaceStatus::Idle;
                    workspace.is_orphaned = false;
                }
                self.event_log.log(
                    LogEvent::new("agent_lifecycle", "agent_stopped")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data("session", Value::from(completion.session_name)),
                );
                self.last_tmux_error = None;
                self.show_flash("agent stopped", false);
                self.poll_preview();
            }
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                self.log_tmux_error(error);
                self.show_flash("agent stop failed", true);
            }
        }
    }

    fn map_interactive_key(key_event: KeyEvent) -> Option<InteractiveKey> {
        let ctrl = key_event.modifiers.contains(Modifiers::CTRL);
        let alt = key_event.modifiers.contains(Modifiers::ALT);

        match key_event.code {
            KeyCode::Enter => Some(InteractiveKey::Enter),
            KeyCode::Tab => Some(InteractiveKey::Tab),
            KeyCode::Backspace => Some(InteractiveKey::Backspace),
            KeyCode::Delete => Some(InteractiveKey::Delete),
            KeyCode::Up => Some(InteractiveKey::Up),
            KeyCode::Down => Some(InteractiveKey::Down),
            KeyCode::Left => Some(InteractiveKey::Left),
            KeyCode::Right => Some(InteractiveKey::Right),
            KeyCode::Home => Some(InteractiveKey::Home),
            KeyCode::End => Some(InteractiveKey::End),
            KeyCode::PageUp => Some(InteractiveKey::PageUp),
            KeyCode::PageDown => Some(InteractiveKey::PageDown),
            KeyCode::Escape => Some(InteractiveKey::Escape),
            KeyCode::F(index) => Some(InteractiveKey::Function(index)),
            KeyCode::Char(character) => {
                if (ctrl && matches!(character, '\\' | '|' | '4')) || character == '\u{1c}' {
                    return Some(InteractiveKey::CtrlBackslash);
                }
                if alt && matches!(character, 'c' | 'C') {
                    return Some(InteractiveKey::AltC);
                }
                if alt && matches!(character, 'v' | 'V') {
                    return Some(InteractiveKey::AltV);
                }
                if ctrl {
                    return Some(InteractiveKey::Ctrl(character));
                }
                Some(InteractiveKey::Char(character))
            }
            _ => None,
        }
    }

    fn queue_interactive_send(&mut self, send: QueuedInteractiveSend) -> Cmd<Msg> {
        self.pending_interactive_sends.push_back(send);
        self.dispatch_next_interactive_send()
    }

    fn dispatch_next_interactive_send(&mut self) -> Cmd<Msg> {
        if self.interactive_send_in_flight {
            return Cmd::None;
        }
        let Some(send) = self.pending_interactive_sends.pop_front() else {
            return Cmd::None;
        };
        self.interactive_send_in_flight = true;
        let command = send.command.clone();
        Cmd::task(move || {
            let started_at = Instant::now();
            let execution = CommandTmuxInput::execute_command(&command);
            let completed_at = Instant::now();
            let tmux_send_ms = u64::try_from(
                completed_at
                    .saturating_duration_since(started_at)
                    .as_millis(),
            )
            .unwrap_or(u64::MAX);
            Msg::InteractiveSendCompleted(InteractiveSendCompletion {
                send,
                tmux_send_ms,
                error: execution.err().map(|error| error.to_string()),
            })
        })
    }

    fn handle_interactive_send_completed(
        &mut self,
        completion: InteractiveSendCompletion,
    ) -> Cmd<Msg> {
        let InteractiveSendCompletion {
            send:
                QueuedInteractiveSend {
                    target_session,
                    action_kind,
                    trace_context,
                    literal_chars,
                    ..
                },
            tmux_send_ms,
            error,
        } = completion;
        self.interactive_send_in_flight = false;
        if let Some(error) = error {
            self.last_tmux_error = Some(error.clone());
            self.log_tmux_error(error.clone());
            if let Some(trace_context) = trace_context {
                self.log_input_event_with_fields(
                    "interactive_forward_failed",
                    trace_context.seq,
                    vec![
                        ("session".to_string(), Value::from(target_session)),
                        ("action".to_string(), Value::from(action_kind)),
                        ("error".to_string(), Value::from(error)),
                    ],
                );
            }
            return self.dispatch_next_interactive_send();
        }

        self.last_tmux_error = None;
        if let Some(trace_context) = trace_context {
            let forwarded_at = Instant::now();
            self.track_pending_interactive_input(trace_context, &target_session, forwarded_at);
            let mut fields = vec![
                ("session".to_string(), Value::from(target_session)),
                ("action".to_string(), Value::from(action_kind)),
                ("tmux_send_ms".to_string(), Value::from(tmux_send_ms)),
                (
                    "queue_depth".to_string(),
                    Value::from(
                        u64::try_from(self.pending_interactive_inputs.len()).unwrap_or(u64::MAX),
                    ),
                ),
            ];
            if let Some(literal_chars) = literal_chars {
                fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
            }
            self.log_input_event_with_fields("interactive_forwarded", trace_context.seq, fields);
        }
        self.dispatch_next_interactive_send()
    }

    fn send_interactive_action(
        &mut self,
        action: &InteractiveAction,
        target_session: &str,
        trace_context: Option<InputTraceContext>,
    ) -> Cmd<Msg> {
        let Some(command) = tmux_send_keys_command(target_session, action) else {
            if let Some(trace_context) = trace_context {
                self.log_input_event_with_fields(
                    "interactive_action_unmapped",
                    trace_context.seq,
                    vec![
                        (
                            "action".to_string(),
                            Value::from(Self::interactive_action_kind(action)),
                        ),
                        (
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        ),
                    ],
                );
            }
            return Cmd::None;
        };

        let literal_chars = if let InteractiveAction::SendLiteral(text) = action {
            Some(u64::try_from(text.chars().count()).unwrap_or(u64::MAX))
        } else {
            None
        };

        if self.tmux_input.supports_background_send() {
            return self.queue_interactive_send(QueuedInteractiveSend {
                command,
                target_session: target_session.to_string(),
                action_kind: Self::interactive_action_kind(action).to_string(),
                trace_context,
                literal_chars,
            });
        }

        let send_started_at = Instant::now();
        match self.execute_tmux_command(&command) {
            Ok(()) => {
                self.last_tmux_error = None;
                if let Some(trace_context) = trace_context {
                    let forwarded_at = Instant::now();
                    let send_duration_ms = Self::duration_millis(
                        forwarded_at.saturating_duration_since(send_started_at),
                    );
                    self.track_pending_interactive_input(
                        trace_context,
                        target_session,
                        forwarded_at,
                    );

                    let mut fields = vec![
                        (
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        ),
                        (
                            "action".to_string(),
                            Value::from(Self::interactive_action_kind(action)),
                        ),
                        ("tmux_send_ms".to_string(), Value::from(send_duration_ms)),
                        (
                            "queue_depth".to_string(),
                            Value::from(
                                u64::try_from(self.pending_interactive_inputs.len())
                                    .unwrap_or(u64::MAX),
                            ),
                        ),
                    ];
                    if let Some(literal_chars) = literal_chars {
                        fields.push(("literal_chars".to_string(), Value::from(literal_chars)));
                    }
                    self.log_input_event_with_fields(
                        "interactive_forwarded",
                        trace_context.seq,
                        fields,
                    );
                }
            }
            Err(error) => {
                let message = error.to_string();
                self.last_tmux_error = Some(message.clone());
                self.log_tmux_error(message);
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "interactive_forward_failed",
                        trace_context.seq,
                        vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            (
                                "action".to_string(),
                                Value::from(Self::interactive_action_kind(action)),
                            ),
                            ("error".to_string(), Value::from(error.to_string())),
                        ],
                    );
                }
            }
        }
        Cmd::None
    }

    fn copy_interactive_capture(&mut self) {
        self.copy_interactive_selection_or_visible();
    }

    fn read_clipboard_or_cached_text(&mut self) -> Result<String, String> {
        let clipboard_text = self.clipboard.read_text();
        if let Ok(text) = clipboard_text
            && !text.is_empty()
        {
            return Ok(text);
        }

        if let Some(text) = self.copied_text.clone()
            && !text.is_empty()
        {
            return Ok(text);
        }

        Err("clipboard empty".to_string())
    }

    fn paste_clipboard_text(
        &mut self,
        target_session: &str,
        bracketed_paste: bool,
        trace_context: Option<InputTraceContext>,
    ) -> Cmd<Msg> {
        let text = match self.read_clipboard_or_cached_text() {
            Ok(text) => text,
            Err(error) => {
                self.last_tmux_error = Some(error.clone());
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "paste_clipboard_missing",
                        trace_context.seq,
                        vec![(
                            "session".to_string(),
                            Value::from(target_session.to_string()),
                        )],
                    );
                }
                return Cmd::None;
            }
        };

        if bracketed_paste {
            let payload = format!("\u{1b}[200~{text}\u{1b}[201~");
            return self.send_interactive_action(
                &InteractiveAction::SendLiteral(payload),
                target_session,
                trace_context,
            );
        }

        match self.tmux_input.paste_buffer(target_session, &text) {
            Ok(()) => {
                self.last_tmux_error = None;
            }
            Err(error) => {
                let message = error.to_string();
                self.last_tmux_error = Some(message.clone());
                self.log_tmux_error(message.clone());
                if let Some(trace_context) = trace_context {
                    self.log_input_event_with_fields(
                        "interactive_paste_buffer_failed",
                        trace_context.seq,
                        vec![
                            (
                                "session".to_string(),
                                Value::from(target_session.to_string()),
                            ),
                            ("error".to_string(), Value::from(message)),
                        ],
                    );
                }
            }
        }

        Cmd::None
    }

    fn handle_interactive_key(&mut self, key_event: KeyEvent) -> Cmd<Msg> {
        let now = Instant::now();
        let input_seq = self.next_input_seq();
        if let KeyCode::Char(character) = key_event.code
            && key_event.modifiers.is_empty()
            && let Some(state) = self.interactive.as_mut()
            && state.should_drop_split_mouse_fragment(character, now)
        {
            self.log_input_event_with_fields(
                "interactive_key_dropped_mouse_fragment",
                input_seq,
                vec![
                    ("code".to_string(), Value::from("char")),
                    ("modifiers".to_string(), Value::from("none")),
                ],
            );
            return Cmd::None;
        }

        let Some(interactive_key) = Self::map_interactive_key(key_event) else {
            self.log_input_event_with_fields(
                "interactive_key_unmapped",
                input_seq,
                vec![(
                    "code".to_string(),
                    Value::from(format!("{:?}", key_event.code)),
                )],
            );
            return Cmd::None;
        };
        self.log_input_event_with_fields(
            "interactive_key_received",
            input_seq,
            vec![
                (
                    "key".to_string(),
                    Value::from(Self::interactive_key_kind(&interactive_key)),
                ),
                (
                    "repeat".to_string(),
                    Value::from(matches!(key_event.kind, KeyEventKind::Repeat)),
                ),
            ],
        );

        let (action, target_session, bracketed_paste) = {
            let Some(state) = self.interactive.as_mut() else {
                return Cmd::None;
            };
            let action = state.handle_key(interactive_key, now);
            let session = state.target_session.clone();
            let bracketed_paste = state.bracketed_paste;
            (action, session, bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_action_selected",
            input_seq,
            vec![
                (
                    "action".to_string(),
                    Value::from(Self::interactive_action_kind(&action)),
                ),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );
        let trace_context = InputTraceContext {
            seq: input_seq,
            received_at: now,
        };

        match action {
            InteractiveAction::ExitInteractive => {
                self.interactive = None;
                self.state.mode = UiMode::Preview;
                self.state.focus = PaneFocus::Preview;
                self.clear_preview_selection();
                Cmd::None
            }
            InteractiveAction::CopySelection => {
                self.copy_interactive_capture();
                Cmd::None
            }
            InteractiveAction::PasteClipboard => {
                if self.preview.offset > 0 {
                    self.preview.jump_to_bottom();
                }
                let send_cmd = self.paste_clipboard_text(
                    &target_session,
                    bracketed_paste,
                    Some(trace_context),
                );
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
            InteractiveAction::Noop
            | InteractiveAction::SendNamed(_)
            | InteractiveAction::SendLiteral(_) => {
                let send_cmd =
                    self.send_interactive_action(&action, &target_session, Some(trace_context));
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
        }
    }

    fn handle_paste_event(&mut self, paste_event: PasteEvent) -> Cmd<Msg> {
        let input_seq = self.next_input_seq();
        let received_at = Instant::now();
        let (target_session, bracketed) = {
            let Some(state) = self.interactive.as_mut() else {
                return Cmd::None;
            };
            state.bracketed_paste = paste_event.bracketed;
            (state.target_session.clone(), state.bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_paste_received",
            input_seq,
            vec![
                (
                    "chars".to_string(),
                    Value::from(
                        u64::try_from(paste_event.text.chars().count()).unwrap_or(u64::MAX),
                    ),
                ),
                ("bracketed".to_string(), Value::from(paste_event.bracketed)),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );

        let payload = encode_paste_payload(&paste_event.text, bracketed || paste_event.bracketed);
        let send_cmd = self.send_interactive_action(
            &InteractiveAction::SendLiteral(payload),
            &target_session,
            Some(InputTraceContext {
                seq: input_seq,
                received_at,
            }),
        );
        self.schedule_interactive_debounced_poll(received_at);
        send_cmd
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => reduce(&mut self.state, Action::ToggleFocus),
            KeyCode::Enter => {
                if !self.enter_interactive(Instant::now()) {
                    reduce(&mut self.state, Action::EnterPreviewMode);
                    self.poll_preview();
                }
            }
            KeyCode::Escape => reduce(&mut self.state, Action::EnterListMode),
            KeyCode::Char('!') => {
                self.launch_skip_permissions = !self.launch_skip_permissions;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => self.open_create_dialog(),
            KeyCode::Char('s') => self.open_start_dialog(),
            KeyCode::Char('x') => self.stop_selected_workspace_agent(),
            KeyCode::PageUp => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-5);
                }
            }
            KeyCode::PageDown => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(5);
                }
            }
            KeyCode::Char('G') => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.jump_preview_to_bottom();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(1);
                } else {
                    self.move_selection(Action::MoveSelectionDown);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-1);
                } else {
                    self.move_selection(Action::MoveSelectionUp);
                }
            }
            _ => {}
        }
    }

    fn view_layout_for_size(width: u16, height: u16, sidebar_width_pct: u16) -> ViewLayout {
        let area = Rect::from_size(width, height);
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(HEADER_HEIGHT),
                Constraint::Fill,
                Constraint::Fixed(STATUS_HEIGHT),
            ])
            .split(area);

        let sidebar_width = ((u32::from(rows[1].width) * u32::from(sidebar_width_pct)) / 100)
            .try_into()
            .unwrap_or(rows[1].width);
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Fixed(sidebar_width),
                Constraint::Fixed(DIVIDER_WIDTH),
                Constraint::Fill,
            ])
            .split(rows[1]);

        ViewLayout {
            header: rows[0],
            sidebar: cols[0],
            divider: cols[1],
            preview: cols[2],
            status: rows[2],
        }
    }

    fn view_layout(&self) -> ViewLayout {
        Self::view_layout_for_size(
            self.viewport_width,
            self.viewport_height,
            self.sidebar_width_pct,
        )
    }

    fn divider_hit_area(divider: Rect, viewport_width: u16) -> Rect {
        let left = divider.x.saturating_sub(1);
        let right = divider.right().saturating_add(1).min(viewport_width);
        Rect::new(left, divider.y, right.saturating_sub(left), divider.height)
    }

    fn hit_region_for_point(&self, x: u16, y: u16) -> (HitRegion, Option<u64>) {
        if let Some((id, _region, data)) = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
        {
            let mapped = match id.id() {
                HIT_ID_HEADER => HitRegion::Header,
                HIT_ID_STATUS => HitRegion::StatusLine,
                HIT_ID_DIVIDER => HitRegion::Divider,
                HIT_ID_PREVIEW => HitRegion::Preview,
                HIT_ID_WORKSPACE_LIST | HIT_ID_WORKSPACE_ROW => HitRegion::WorkspaceList,
                HIT_ID_CREATE_DIALOG | HIT_ID_LAUNCH_DIALOG => HitRegion::Outside,
                _ => HitRegion::Outside,
            };
            let row_data = if id.id() == HIT_ID_WORKSPACE_ROW {
                Some(data)
            } else {
                None
            };
            return (mapped, row_data);
        }

        let layout = self.view_layout();

        if x >= self.viewport_width || y >= self.viewport_height {
            return (HitRegion::Outside, None);
        }
        if y < layout.header.bottom() {
            return (HitRegion::Header, None);
        }
        if y >= layout.status.y {
            return (HitRegion::StatusLine, None);
        }

        let divider_area = Self::divider_hit_area(layout.divider, self.viewport_width);
        if x >= divider_area.x && x < divider_area.right() {
            return (HitRegion::Divider, None);
        }
        if x >= layout.sidebar.x && x < layout.sidebar.right() {
            return (HitRegion::WorkspaceList, None);
        }
        if x >= layout.preview.x && x < layout.preview.right() {
            return (HitRegion::Preview, None);
        }

        (HitRegion::Outside, None)
    }

    fn interactive_cursor_target(&self, preview_height: usize) -> Option<(usize, usize, bool)> {
        let Some(interactive) = self.interactive.as_ref() else {
            return None;
        };
        if self.preview.lines.is_empty() {
            return None;
        }

        let pane_height = usize::from(interactive.pane_height.max(1));
        let cursor_row = usize::from(interactive.cursor_row);
        if cursor_row >= pane_height {
            return None;
        }

        let preview_len = self.preview.lines.len();
        let pane_start = preview_len.saturating_sub(pane_height);
        let cursor_line = pane_start.saturating_add(cursor_row);
        if cursor_line >= preview_len {
            return None;
        }

        let end = preview_len.saturating_sub(self.preview.offset);
        let start = end.saturating_sub(preview_height);
        if cursor_line < start || cursor_line >= end {
            return None;
        }

        let visible_index = cursor_line - start;
        Some((
            visible_index,
            usize::from(interactive.cursor_col),
            interactive.cursor_visible,
        ))
    }

    #[cfg(test)]
    fn apply_interactive_cursor_overlay(
        &self,
        visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(line) = visible_lines.get_mut(visible_index) else {
            return;
        };

        *line = render_cursor_overlay(line, cursor_col, cursor_visible);
    }

    fn apply_interactive_cursor_overlay_render(
        &self,
        plain_visible_lines: &[String],
        render_visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(plain_line) = plain_visible_lines.get(visible_index) else {
            return;
        };
        let Some(render_line) = render_visible_lines.get_mut(visible_index) else {
            return;
        };

        *render_line =
            render_cursor_overlay_ansi(render_line, plain_line, cursor_col, cursor_visible);
    }

    fn clear_preview_selection(&mut self) {
        self.preview_selection.clear();
    }

    fn preview_visible_range_for_height(&self, preview_height: usize) -> (usize, usize) {
        if preview_height == 0 {
            return (0, 0);
        }

        let max_offset = self.preview.max_scroll_offset(preview_height);
        let clamped_offset = self.preview.offset.min(max_offset);
        let end = self.preview.lines.len().saturating_sub(clamped_offset);
        let start = end.saturating_sub(preview_height);
        (start, end)
    }

    fn preview_content_viewport(&self) -> Option<PreviewContentViewport> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }
        let inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if inner.is_empty() {
            return None;
        }

        let preview_height = usize::from(inner.height)
            .saturating_sub(usize::from(PREVIEW_METADATA_ROWS))
            .max(1);
        let (visible_start, visible_end) = self.preview_visible_range_for_height(preview_height);

        Some(PreviewContentViewport {
            output_x: inner.x,
            output_y: inner.y.saturating_add(PREVIEW_METADATA_ROWS),
            visible_start,
            visible_end,
        })
    }

    fn preview_text_point_at(&self, x: u16, y: u16) -> Option<TextSelectionPoint> {
        let viewport = self.preview_content_viewport()?;
        if y < viewport.output_y {
            return None;
        }

        let visible_row = usize::from(y - viewport.output_y);
        let visible_count = viewport.visible_end.saturating_sub(viewport.visible_start);
        if visible_row >= visible_count {
            return None;
        }

        let line_idx = viewport.visible_start.saturating_add(visible_row);
        let line = self.preview.lines.get(line_idx)?;
        let line_len = line.chars().count();
        if x < viewport.output_x {
            return Some(TextSelectionPoint {
                line: line_idx,
                col: 0,
            });
        }

        let relative_x = usize::from(x - viewport.output_x);
        let col = if line_len == 0 {
            0
        } else {
            relative_x.min(line_len.saturating_sub(1))
        };

        Some(TextSelectionPoint {
            line: line_idx,
            col,
        })
    }

    fn prepare_preview_selection_drag(&mut self, x: u16, y: u16) {
        if let Some(point) = self.preview_text_point_at(x, y) {
            self.preview_selection.prepare_drag(point);
            return;
        }

        self.clear_preview_selection();
    }

    fn update_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        let Some(point) = self.preview_text_point_at(x, y) else {
            return;
        };
        self.preview_selection.handle_drag(point);
    }

    fn finish_preview_selection_drag(&mut self, x: u16, y: u16) {
        if self.preview_selection.anchor.is_none() {
            return;
        }
        if !self.preview_selection.has_selection()
            && let Some(point) = self.preview_text_point_at(x, y)
        {
            self.preview_selection.handle_drag(point);
        }
        self.preview_selection.finish_drag();
    }

    fn apply_preview_selection_highlight(
        &self,
        render_visible_lines: &mut [String],
        visible_start: usize,
    ) {
        if !self.preview_selection.has_selection() {
            return;
        }

        let selection_bg = ui_theme().surface1;
        for (offset, line) in render_visible_lines.iter_mut().enumerate() {
            let line_idx = visible_start.saturating_add(offset);
            let Some((start_col, end_col)) = self.preview_selection.line_selection_cols(line_idx)
            else {
                continue;
            };
            *line = inject_selection_background_ansi(line, start_col, end_col, selection_bg);
        }
    }

    fn selected_preview_text_lines(&self) -> Option<Vec<String>> {
        let (start, end) = self.preview_selection.bounds()?;
        if self.preview.lines.is_empty() {
            return None;
        }

        let start_line = start.line.min(self.preview.lines.len().saturating_sub(1));
        let end_line = end.line.min(self.preview.lines.len().saturating_sub(1));
        if end_line < start_line {
            return None;
        }

        let mut lines = self.preview.lines[start_line..=end_line].to_vec();
        if lines.is_empty() {
            return None;
        }

        if lines.len() == 1 {
            let end_exclusive = end.col.saturating_add(1);
            lines[0] = lines[0]
                .chars()
                .skip(start.col)
                .take(end_exclusive.saturating_sub(start.col))
                .collect();
            return Some(lines);
        }

        lines[0] = lines[0].chars().skip(start.col).collect();
        let last_idx = lines.len().saturating_sub(1);
        lines[last_idx] = lines[last_idx]
            .chars()
            .take(end.col.saturating_add(1))
            .collect();

        Some(lines)
    }

    fn visible_preview_output_lines(&self) -> Vec<String> {
        let Some((_, output_height)) = self.preview_output_dimensions() else {
            return Vec::new();
        };
        self.preview.visible_lines(usize::from(output_height))
    }

    fn copy_interactive_selection_or_visible(&mut self) {
        let mut lines = self
            .selected_preview_text_lines()
            .unwrap_or_else(|| self.visible_preview_output_lines());
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            return;
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            return;
        }
        let text = lines.join("\n");
        self.copied_text = Some(text.clone());
        match self.clipboard.write_text(&text) {
            Ok(()) => {
                self.last_tmux_error = None;
            }
            Err(error) => {
                self.last_tmux_error = Some(format!("clipboard write failed: {error}"));
            }
        }
        self.clear_preview_selection();
    }

    fn select_workspace_by_mouse(&mut self, y: u16) {
        if !matches!(self.discovery_state, DiscoveryState::Ready) {
            return;
        }

        let layout = self.view_layout();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return;
        }

        let row = usize::from((y - sidebar_inner.y) / WORKSPACE_ITEM_HEIGHT);
        if row >= self.state.workspaces.len() {
            return;
        }

        if row != self.state.selected_index {
            self.state.selected_index = row;
            self.preview.jump_to_bottom();
            self.clear_preview_selection();
            self.poll_preview();
        }
    }

    fn select_workspace_by_index(&mut self, index: usize) {
        if index >= self.state.workspaces.len() {
            return;
        }
        if index == self.state.selected_index {
            return;
        }

        self.state.selected_index = index;
        self.preview.jump_to_bottom();
        self.clear_preview_selection();
        self.poll_preview();
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(state) = self.interactive.as_mut() {
            state.note_mouse_event(Instant::now());
        }

        if self.modal_open() {
            return;
        }

        let (region, row_data) = self.hit_region_for_point(mouse_event.x, mouse_event.y);

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => match region {
                HitRegion::Divider => {
                    self.divider_drag_active = true;
                }
                HitRegion::WorkspaceList => {
                    self.state.focus = PaneFocus::WorkspaceList;
                    self.state.mode = UiMode::List;
                    if let Some(row_data) = row_data {
                        if let Ok(index) = usize::try_from(row_data) {
                            self.select_workspace_by_index(index);
                        }
                    } else {
                        self.select_workspace_by_mouse(mouse_event.y);
                    }
                }
                HitRegion::Preview => {
                    self.state.focus = PaneFocus::Preview;
                    self.state.mode = UiMode::Preview;
                    if self.interactive.is_some() {
                        self.prepare_preview_selection_drag(mouse_event.x, mouse_event.y);
                    } else {
                        self.clear_preview_selection();
                    }
                }
                HitRegion::StatusLine | HitRegion::Header | HitRegion::Outside => {}
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.divider_drag_active {
                    let ratio =
                        clamp_sidebar_ratio(ratio_from_drag(self.viewport_width, mouse_event.x));
                    if ratio != self.sidebar_width_pct {
                        self.sidebar_width_pct = ratio;
                        self.persist_sidebar_ratio();
                        self.sync_interactive_session_geometry();
                    }
                } else if self.interactive.is_some() {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Moved => {
                if self.interactive.is_some() && !self.divider_drag_active {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.divider_drag_active = false;
                self.finish_preview_selection_drag(mouse_event.x, mouse_event.y);
            }
            MouseEventKind::ScrollUp => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    self.scroll_preview(-1);
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    self.scroll_preview(1);
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key_event: KeyEvent) -> (bool, Cmd<Msg>) {
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return (false, Cmd::None);
        }

        if self.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        let keybinding_state = self.keybinding_state();
        if let Some(action) = self
            .action_mapper
            .map(&key_event, &keybinding_state, Instant::now())
        {
            if !matches!(action, KeybindingAction::PassThrough) {
                return (self.apply_keybinding_action(action), Cmd::None);
            }
        } else {
            return (false, Cmd::None);
        }

        if self.create_dialog.is_some() {
            self.handle_create_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.launch_dialog.is_some() {
            self.handle_launch_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if Self::is_quit_key(&key_event) {
            return (true, Cmd::None);
        }

        self.handle_non_interactive_key(key_event);
        (false, Cmd::None)
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

    fn visual_tick_interval(&self) -> Option<Duration> {
        if self.status_is_visually_working(self.selected_workspace_status(), true) {
            return Some(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS));
        }
        None
    }

    fn advance_visual_animation(&mut self) {
        self.fast_animation_frame = self.fast_animation_frame.wrapping_add(1);
    }

    fn status_is_visually_working(&self, status: WorkspaceStatus, is_selected: bool) -> bool {
        match status {
            WorkspaceStatus::Thinking => true,
            WorkspaceStatus::Active => is_selected && self.agent_output_changing,
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

    fn pane_border_style(&self, focused: bool) -> Style {
        let theme = ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    fn workspace_status_color(&self, status: WorkspaceStatus) -> PackedRgba {
        let theme = ui_theme();
        match status {
            WorkspaceStatus::Main => theme.lavender,
            WorkspaceStatus::Idle => theme.overlay0,
            WorkspaceStatus::Active | WorkspaceStatus::Thinking => theme.green,
            WorkspaceStatus::Waiting => theme.yellow,
            WorkspaceStatus::Done => theme.teal,
            WorkspaceStatus::Error => theme.red,
            WorkspaceStatus::Unknown | WorkspaceStatus::Unsupported => theme.peach,
        }
    }

    fn status_icon(&self, status: WorkspaceStatus, is_selected: bool) -> &'static str {
        if self.status_is_visually_working(status, is_selected) {
            return FAST_SPINNER_FRAMES[self.fast_animation_frame % FAST_SPINNER_FRAMES.len()];
        }

        status.icon()
    }

    fn activity_spinner_slot(&self, status: WorkspaceStatus, is_selected: bool) -> &'static str {
        if self.status_is_visually_working(status, is_selected) {
            return FAST_SPINNER_FRAMES[self.fast_animation_frame % FAST_SPINNER_FRAMES.len()];
        }

        " "
    }

    fn relative_age_label(&self, unix_secs: Option<i64>) -> String {
        let Some(unix_secs) = unix_secs else {
            return String::new();
        };
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        let Some(now_secs) = now_secs else {
            return String::new();
        };
        let age_secs = now_secs.saturating_sub(unix_secs).max(0);
        if age_secs < 60 {
            return "now".to_string();
        }
        if age_secs < 3_600 {
            return format!("{}m", age_secs / 60);
        }
        if age_secs < 86_400 {
            return format!("{}h", age_secs / 3_600);
        }
        format!("{}d", age_secs / 86_400)
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let mode_chip = format!("[{}]", self.mode_label());
        let focus_chip = format!("[{}]", self.focus_label());
        let selected_status = self.selected_workspace_status();
        let activity_chip = format!(
            "{} {}",
            self.status_icon(selected_status, true),
            self.selected_status_hint()
        );

        let header = StatusLine::new()
            .separator("  ")
            .style(Style::new().bg(theme.crust).fg(theme.text))
            .left(StatusItem::text("Grove"))
            .left(StatusItem::text(self.repo_name.as_str()))
            .center(StatusItem::text(mode_chip.as_str()))
            .center(StatusItem::text(focus_chip.as_str()))
            .right(StatusItem::text(activity_chip.as_str()))
            .right(StatusItem::text(
                self.activity_spinner_slot(selected_status, true),
            ));

        header.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_HEADER));
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Workspaces")
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(
                self.state.focus == PaneFocus::WorkspaceList && !self.modal_open(),
            ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_WORKSPACE_LIST));

        if inner.is_empty() {
            return;
        }

        let theme = ui_theme();
        let mut lines: Vec<FtLine> = Vec::new();
        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "Discovery error",
                    Style::new().fg(theme.red).bold(),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    message.as_str(),
                    Style::new().fg(theme.peach),
                )]));
            }
            DiscoveryState::Empty => {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "No workspaces",
                    Style::new().fg(theme.subtext0),
                )]));
            }
            DiscoveryState::Ready => {
                let max_items = usize::from(inner.height / WORKSPACE_ITEM_HEIGHT);
                for (idx, workspace) in self.state.workspaces.iter().take(max_items).enumerate() {
                    let is_selected = idx == self.state.selected_index;
                    let selected = if idx == self.state.selected_index {
                        ">"
                    } else {
                        " "
                    };
                    let icon = self.status_icon(workspace.status, is_selected);
                    let age = self.relative_age_label(workspace.last_activity_unix_secs);

                    let secondary = format!(
                        "  {} | {}{}",
                        workspace.branch,
                        workspace.agent.label(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    );

                    let row_background = if idx == self.state.selected_index {
                        if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                            Some(theme.surface1)
                        } else {
                            Some(theme.surface0)
                        }
                    } else if workspace.status == WorkspaceStatus::Waiting {
                        Some(theme.surface0)
                    } else {
                        None
                    };

                    let mut primary_style = Style::new().fg(theme.text);
                    let mut secondary_style = Style::new().fg(theme.subtext0);
                    if let Some(bg) = row_background {
                        primary_style = primary_style.bg(bg);
                        secondary_style = secondary_style.bg(bg);
                    }
                    if idx == self.state.selected_index {
                        primary_style = primary_style.bold();
                    }

                    let mut primary_spans = vec![
                        FtSpan::styled(format!("{selected} "), primary_style),
                        FtSpan::styled(
                            format!("{icon} "),
                            primary_style
                                .fg(self.workspace_status_color(workspace.status))
                                .bold(),
                        ),
                        FtSpan::styled(workspace.name.clone(), primary_style),
                    ];
                    if !age.is_empty() {
                        primary_spans.push(FtSpan::styled("  ", primary_style));
                        primary_spans.push(FtSpan::styled(age, primary_style.fg(theme.overlay0)));
                    }

                    lines.push(FtLine::from_spans(primary_spans));
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        secondary,
                        secondary_style,
                    )]));

                    if let Ok(data) = u64::try_from(idx) {
                        let row_y = inner.y.saturating_add(
                            u16::try_from(idx)
                                .unwrap_or(u16::MAX)
                                .saturating_mul(WORKSPACE_ITEM_HEIGHT),
                        );
                        let row_height =
                            WORKSPACE_ITEM_HEIGHT.min(inner.bottom().saturating_sub(row_y));
                        let row_rect = Rect::new(inner.x, row_y, inner.width, row_height);
                        let _ = frame.register_hit(
                            row_rect,
                            HitId::new(HIT_ID_WORKSPACE_ROW),
                            FrameHitRegion::Content,
                            data,
                        );
                    }
                }
            }
        }

        Paragraph::new(FtText::from_lines(lines)).render(inner, frame);
    }

    fn render_divider(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let glyph = if self.divider_drag_active {
            "█"
        } else {
            "│"
        };
        let divider = std::iter::repeat(glyph)
            .take(usize::from(area.height))
            .collect::<Vec<&str>>()
            .join("\n");
        let theme = ui_theme();
        Paragraph::new(divider)
            .style(Style::new().fg(if self.divider_drag_active {
                theme.blue
            } else {
                theme.overlay0
            }))
            .render(area, frame);
        let _ = frame.register_hit_region(
            Self::divider_hit_area(area, frame.width()),
            HitId::new(HIT_ID_DIVIDER),
        );
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let title = if self.interactive.is_some() {
            "Preview (Interactive)"
        } else {
            "Preview"
        };
        let block =
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_style(self.pane_border_style(
                    self.state.focus == PaneFocus::Preview && !self.modal_open(),
                ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_PREVIEW));

        if inner.is_empty() {
            return;
        }

        let selected_workspace = self.state.selected_workspace();
        let selected_agent = selected_workspace.map(|workspace| workspace.agent);
        let allow_cursor_overlay = selected_agent != Some(AgentType::Codex);
        let theme = ui_theme();
        let selected_workspace_label = selected_workspace
            .map(|workspace| {
                let name_label = if workspace.name == workspace.branch {
                    workspace.name.clone()
                } else {
                    format!("{} ({})", workspace.name, workspace.branch)
                };
                let mode_label = if self.interactive.is_some() {
                    "INTERACTIVE"
                } else {
                    "PREVIEW"
                };
                let detail = format!(
                    "{} | {} | {}",
                    workspace.agent.label(),
                    workspace.status.icon(),
                    workspace.path.display()
                );
                (format!("{mode_label} | {name_label}"), detail)
            })
            .unwrap_or_else(|| ("PREVIEW | none".to_string(), "no workspace".to_string()));

        let metadata_rows = usize::from(PREVIEW_METADATA_ROWS);
        let preview_height = usize::from(inner.height)
            .saturating_sub(metadata_rows)
            .max(1);

        let mut text_lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                selected_workspace_label.0,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                selected_workspace_label.1,
                Style::new().fg(theme.overlay0),
            )]),
        ];

        let visible_plain_lines = self.preview.visible_lines(preview_height);
        let mut visible_render_lines = self.preview.visible_render_lines(preview_height);
        let (visible_start, _visible_end) = self.preview_visible_range_for_height(preview_height);
        if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
            visible_render_lines = visible_plain_lines.clone();
        }
        self.apply_preview_selection_highlight(&mut visible_render_lines, visible_start);
        if allow_cursor_overlay {
            self.apply_interactive_cursor_overlay_render(
                &visible_plain_lines,
                &mut visible_render_lines,
                preview_height,
            );
        }

        if visible_render_lines.is_empty() {
            text_lines.push(FtLine::raw("(no preview output)"));
        } else {
            text_lines.extend(
                visible_render_lines
                    .iter()
                    .map(|line| ansi_line_to_styled_line(line)),
            );
        }

        Paragraph::new(FtText::from_lines(text_lines)).render(inner, frame);
    }

    fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let hints = self.keybind_hints_line();

        let status = StatusLine::new()
            .separator("  ")
            .style(Style::new().bg(theme.mantle).fg(theme.text))
            .left(StatusItem::text(hints));
        status.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }

    fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 8 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 8u16;
        let dialog_x = area.x + area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.y + area.height.saturating_sub(dialog_height) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
        let theme = ui_theme();

        let block = Block::new()
            .title("Start Agent")
            .borders(Borders::ALL)
            .style(Style::new().bg(theme.base).fg(theme.text))
            .border_style(Style::new().fg(theme.mauve).bold());
        let inner = block.inner(dialog_area);
        block.render(dialog_area, frame);
        let _ = frame.register_hit(
            dialog_area,
            HitId::new(HIT_ID_LAUNCH_DIALOG),
            FrameHitRegion::Content,
            0,
        );

        if inner.is_empty() {
            return;
        }

        let body = [
            "Edit prompt, [Tab] toggles unsafe, [Enter] starts, [Esc] cancels".to_string(),
            String::new(),
            format!(
                "Unsafe launch: {}",
                if dialog.skip_permissions { "on" } else { "off" }
            ),
            format!(
                "Prompt: {}",
                if dialog.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.prompt.clone()
                }
            ),
        ]
        .join("\n");

        Paragraph::new(body)
            .style(Style::new().fg(theme.text).bg(theme.base))
            .render(inner, frame);
    }

    fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 10u16;
        let dialog_x = area.x + area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.y + area.height.saturating_sub(dialog_height) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
        let theme = ui_theme();

        let block = Block::new()
            .title("New Workspace")
            .borders(Borders::ALL)
            .style(Style::new().bg(theme.base).fg(theme.text))
            .border_style(Style::new().fg(theme.mauve).bold());
        let inner = block.inner(dialog_area);
        block.render(dialog_area, frame);
        let _ = frame.register_hit(
            dialog_area,
            HitId::new(HIT_ID_CREATE_DIALOG),
            FrameHitRegion::Content,
            0,
        );

        if inner.is_empty() {
            return;
        }

        let active_branch_value = match dialog.branch_mode {
            CreateBranchMode::NewBranch => dialog.base_branch.as_str(),
            CreateBranchMode::ExistingBranch => dialog.existing_branch.as_str(),
        };
        let body = [
            "Create workspace: [Tab/Shift+Tab] field, [Left/Right] toggle mode/agent, [Enter] create".to_string(),
            format!("Focus: {}", dialog.focused_field.label()),
            format!("Name: {}", dialog.workspace_name),
            format!("Branch mode: {}", dialog.branch_mode.label()),
            format!("Branch value: {}", active_branch_value),
            format!("Base branch: {}", dialog.base_branch),
            format!("Existing branch: {}", dialog.existing_branch),
            format!("Agent: {}", dialog.agent.label()),
        ]
        .join("\n");

        Paragraph::new(body)
            .style(Style::new().fg(theme.text).bg(theme.base))
            .render(inner, frame);
    }

    #[cfg(test)]
    fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab focus, Enter preview, s start, x stop, ! unsafe toggle, Esc list, mouse enabled)"
                .to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        ">"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{} {} {} | {} | {}{}",
                        selected,
                        workspace.status.icon(),
                        workspace.name,
                        workspace.branch,
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        if let Some(dialog) = &self.launch_dialog {
            lines.push(String::new());
            lines.push("Start Agent Dialog".to_string());
            lines.push(format!(
                "Prompt: {}",
                if dialog.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.prompt.clone()
                }
            ));
            lines.push(format!(
                "Unsafe launch: {}",
                if dialog.skip_permissions { "on" } else { "off" }
            ));
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let mut visible_lines = self.preview.visible_lines(preview_height);
        self.apply_interactive_cursor_overlay(&mut visible_lines, preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }
}

impl Model for GroveApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.poll_preview();
        let next_tick_cmd = self.schedule_next_tick();
        let init_cmd = Cmd::batch(vec![next_tick_cmd, Cmd::set_mouse_capture(true)]);
        self.merge_deferred_cmds(init_cmd)
    }

    fn update(&mut self, msg: Msg) -> Cmd<Self::Message> {
        let update_started_at = Instant::now();
        let msg_kind = Self::msg_kind(&msg);
        let before = self.capture_transition_snapshot();
        let cmd = match msg {
            Msg::Tick => {
                let now = Instant::now();
                let pending_before = self.pending_input_depth();
                let oldest_pending_before_ms = self.oldest_pending_input_age_ms(now);
                let late_by_ms = self
                    .next_tick_due_at
                    .map(|due_at| Self::duration_millis(now.saturating_duration_since(due_at)))
                    .unwrap_or(0);
                let early_by_ms = self
                    .next_tick_due_at
                    .map(|due_at| Self::duration_millis(due_at.saturating_duration_since(now)))
                    .unwrap_or(0);
                let _ = clear_expired_flash_message(&mut self.flash, Instant::now());
                if !self.tick_is_due(now) {
                    self.event_log.log(
                        LogEvent::new("tick", "skipped")
                            .with_data("reason", Value::from("not_due"))
                            .with_data(
                                "interval_ms",
                                Value::from(self.next_tick_interval_ms.unwrap_or(0)),
                            )
                            .with_data("late_by_ms", Value::from(late_by_ms))
                            .with_data("early_by_ms", Value::from(early_by_ms))
                            .with_data("pending_depth", Value::from(pending_before))
                            .with_data(
                                "oldest_pending_age_ms",
                                Value::from(oldest_pending_before_ms),
                            ),
                    );
                    Cmd::None
                } else {
                    let poll_due = self
                        .next_poll_due_at
                        .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));
                    let visual_due = self
                        .next_visual_due_at
                        .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at));

                    self.next_tick_due_at = None;
                    self.next_tick_interval_ms = None;
                    if visual_due {
                        self.next_visual_due_at = None;
                        self.advance_visual_animation();
                    }
                    if poll_due {
                        self.next_poll_due_at = None;
                        if self
                            .interactive_poll_due_at
                            .is_some_and(|due_at| Self::is_due_with_tolerance(now, due_at))
                        {
                            self.interactive_poll_due_at = None;
                        }
                        self.poll_preview();
                    }

                    let pending_after = self.pending_input_depth();
                    self.event_log.log(
                        LogEvent::new("tick", "processed")
                            .with_data("late_by_ms", Value::from(late_by_ms))
                            .with_data("early_by_ms", Value::from(early_by_ms))
                            .with_data("poll_due", Value::from(poll_due))
                            .with_data("visual_due", Value::from(visual_due))
                            .with_data("pending_before", Value::from(pending_before))
                            .with_data("pending_after", Value::from(pending_after))
                            .with_data(
                                "drained_count",
                                Value::from(pending_before.saturating_sub(pending_after)),
                            ),
                    );
                    self.schedule_next_tick()
                }
            }
            Msg::Key(key_event) => {
                let (quit, key_cmd) = self.handle_key(key_event);
                if quit {
                    Cmd::Quit
                } else {
                    let tick_cmd = self.schedule_next_tick();
                    if matches!(key_cmd, Cmd::None) {
                        tick_cmd
                    } else {
                        Cmd::batch(vec![key_cmd, tick_cmd])
                    }
                }
            }
            Msg::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
                self.schedule_next_tick()
            }
            Msg::Paste(paste_event) => {
                let paste_cmd = self.handle_paste_event(paste_event);
                let tick_cmd = self.schedule_next_tick();
                if matches!(paste_cmd, Cmd::None) {
                    tick_cmd
                } else {
                    Cmd::batch(vec![paste_cmd, tick_cmd])
                }
            }
            Msg::Resize { width, height } => {
                self.viewport_width = width;
                self.viewport_height = height;
                let interactive_active = self.interactive.is_some();
                if let Some(state) = self.interactive.as_mut() {
                    state.update_cursor(
                        state.cursor_row,
                        state.cursor_col,
                        state.cursor_visible,
                        height,
                        width,
                    );
                }
                self.sync_interactive_session_geometry();
                if interactive_active {
                    self.poll_preview();
                }
                Cmd::None
            }
            Msg::PreviewPollCompleted(completion) => {
                self.handle_preview_poll_completed(completion);
                Cmd::None
            }
            Msg::RefreshWorkspacesCompleted(completion) => {
                self.apply_refresh_workspaces_completion(completion);
                Cmd::None
            }
            Msg::CreateWorkspaceCompleted(completion) => {
                self.apply_create_workspace_completion(completion);
                Cmd::None
            }
            Msg::StartAgentCompleted(completion) => {
                self.apply_start_agent_completion(completion);
                Cmd::None
            }
            Msg::StopAgentCompleted(completion) => {
                self.apply_stop_agent_completion(completion);
                Cmd::None
            }
            Msg::InteractiveSendCompleted(completion) => {
                self.handle_interactive_send_completed(completion)
            }
            Msg::Noop => Cmd::None,
        };
        self.emit_transition_events(&before);
        self.event_log.log(
            LogEvent::new("update_timing", "message_handled")
                .with_data("msg_kind", Value::from(msg_kind))
                .with_data(
                    "update_ms",
                    Value::from(Self::duration_millis(
                        Instant::now().saturating_duration_since(update_started_at),
                    )),
                )
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
        self.merge_deferred_cmds(cmd)
    }

    fn view(&self, frame: &mut Frame) {
        let view_started_at = Instant::now();
        frame.enable_hit_testing();
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let layout = Self::view_layout_for_size(
            frame.buffer.width(),
            frame.buffer.height(),
            self.sidebar_width_pct,
        );

        self.render_header(frame, layout.header);
        self.render_sidebar(frame, layout.sidebar);
        self.render_divider(frame, layout.divider);
        self.render_preview_pane(frame, layout.preview);
        self.render_status_line(frame, layout.status);
        self.render_create_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        let draw_completed_at = Instant::now();
        self.last_hit_grid.replace(frame.hit_grid.clone());
        let frame_log_started_at = Instant::now();
        self.log_frame_render(frame);
        let view_completed_at = Instant::now();
        self.event_log.log(
            LogEvent::new("frame", "timing")
                .with_data(
                    "draw_ms",
                    Value::from(Self::duration_millis(
                        draw_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data(
                    "frame_log_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(frame_log_started_at),
                    )),
                )
                .with_data(
                    "view_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data("degradation", Value::from(frame.degradation.as_str()))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
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
mod tests {
    mod render_support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/render.rs"
        ));
    }

    use self::render_support::{
        assert_row_bg, assert_row_fg, find_cell_with_char, find_row_containing, row_text,
    };
    use super::{
        ClipboardAccess, CreateBranchMode, CreateDialogField, CreateWorkspaceCompletion,
        CursorCapture, FAST_SPINNER_FRAMES, GroveApp, HIT_ID_HEADER, HIT_ID_PREVIEW, HIT_ID_STATUS,
        HIT_ID_WORKSPACE_ROW, LaunchDialogState, LivePreviewCapture, Msg, PREVIEW_METADATA_ROWS,
        PendingResizeVerification, PreviewPollCompletion, StartAgentCompletion,
        StopAgentCompletion, TmuxInput, WORKSPACE_ITEM_HEIGHT, ansi_16_color,
        ansi_line_to_styled_line, parse_cursor_metadata, ui_theme,
    };
    use crate::adapters::{BootstrapData, DiscoveryState};
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use crate::event_log::{Event as LoggedEvent, EventLogger, NullEventLogger};
    use crate::interactive::InteractiveState;
    use crate::workspace_lifecycle::{BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult};
    use ftui::core::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        PasteEvent,
    };
    use ftui::render::frame::HitId;
    use ftui::widgets::block::Block;
    use ftui::widgets::borders::Borders;
    use ftui::{Cmd, Frame, GraphemePool};
    use proptest::prelude::*;
    use serde_json::Value;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    type RecordedCommands = Rc<RefCell<Vec<Vec<String>>>>;
    type RecordedCaptures = Rc<RefCell<Vec<Result<String, String>>>>;
    type RecordedCalls = Rc<RefCell<Vec<String>>>;
    type RecordedEvents = Arc<Mutex<Vec<LoggedEvent>>>;
    type FixtureApp = (
        GroveApp,
        RecordedCommands,
        RecordedCaptures,
        RecordedCaptures,
    );
    type FixtureAppWithCalls = (
        GroveApp,
        RecordedCommands,
        RecordedCaptures,
        RecordedCaptures,
        RecordedCalls,
    );
    type FixtureAppWithEvents = (
        GroveApp,
        RecordedCommands,
        RecordedCaptures,
        RecordedCaptures,
        RecordedEvents,
    );

    struct RecordingEventLogger {
        events: RecordedEvents,
    }

    impl EventLogger for RecordingEventLogger {
        fn log(&self, event: LoggedEvent) {
            let Ok(mut events) = self.events.lock() else {
                return;
            };
            events.push(event);
        }
    }

    #[derive(Clone)]
    struct RecordingTmuxInput {
        commands: RecordedCommands,
        captures: RecordedCaptures,
        cursor_captures: RecordedCaptures,
        calls: RecordedCalls,
    }

    #[derive(Clone, Default)]
    struct RecordingClipboard {
        text: Rc<RefCell<String>>,
    }

    impl ClipboardAccess for RecordingClipboard {
        fn read_text(&mut self) -> Result<String, String> {
            Ok(self.text.borrow().clone())
        }

        fn write_text(&mut self, text: &str) -> Result<(), String> {
            self.text.replace(text.to_string());
            Ok(())
        }
    }

    fn test_clipboard() -> Box<dyn ClipboardAccess> {
        Box::new(RecordingClipboard::default())
    }

    impl TmuxInput for RecordingTmuxInput {
        fn execute(&self, command: &[String]) -> std::io::Result<()> {
            self.commands.borrow_mut().push(command.to_vec());
            self.calls
                .borrow_mut()
                .push(format!("exec:{}", command.join(" ")));
            Ok(())
        }

        fn capture_output(
            &self,
            target_session: &str,
            scrollback_lines: usize,
            include_escape_sequences: bool,
        ) -> std::io::Result<String> {
            self.calls.borrow_mut().push(format!(
                "capture:{target_session}:{scrollback_lines}:{include_escape_sequences}"
            ));
            let mut captures = self.captures.borrow_mut();
            if captures.is_empty() {
                return Ok(String::new());
            }

            let next = captures.remove(0);
            match next {
                Ok(output) => Ok(output),
                Err(error) => Err(std::io::Error::other(error)),
            }
        }

        fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
            self.calls
                .borrow_mut()
                .push(format!("cursor:{target_session}"));
            let mut captures = self.cursor_captures.borrow_mut();
            if captures.is_empty() {
                return Ok("1 0 0 120 40".to_string());
            }

            let next = captures.remove(0);
            match next {
                Ok(output) => Ok(output),
                Err(error) => Err(std::io::Error::other(error)),
            }
        }

        fn resize_session(
            &self,
            target_session: &str,
            target_width: u16,
            target_height: u16,
        ) -> std::io::Result<()> {
            self.calls.borrow_mut().push(format!(
                "resize:{target_session}:{target_width}:{target_height}"
            ));
            Ok(())
        }

        fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
            self.calls.borrow_mut().push(format!(
                "paste-buffer:{target_session}:{}",
                text.chars().count()
            ));
            self.commands.borrow_mut().push(vec![
                "tmux".to_string(),
                "paste-buffer".to_string(),
                "-t".to_string(),
                target_session.to_string(),
                text.to_string(),
            ]);
            Ok(())
        }
    }

    #[derive(Clone)]
    struct BackgroundOnlyTmuxInput;

    impl TmuxInput for BackgroundOnlyTmuxInput {
        fn execute(&self, _command: &[String]) -> std::io::Result<()> {
            Ok(())
        }

        fn capture_output(
            &self,
            _target_session: &str,
            _scrollback_lines: usize,
            _include_escape_sequences: bool,
        ) -> std::io::Result<String> {
            panic!("sync preview capture should not run when background mode is enabled")
        }

        fn capture_cursor_metadata(&self, _target_session: &str) -> std::io::Result<String> {
            panic!("sync cursor capture should not run when background mode is enabled")
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
    }

    fn fixture_bootstrap(status: WorkspaceStatus) -> BootstrapData {
        BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: vec![
                Workspace::try_new(
                    "grove".to_string(),
                    PathBuf::from("/repos/grove"),
                    "main".to_string(),
                    Some(1_700_000_200),
                    AgentType::Claude,
                    WorkspaceStatus::Main,
                    true,
                )
                .expect("workspace should be valid"),
                Workspace::try_new(
                    "feature-a".to_string(),
                    PathBuf::from("/repos/grove-feature-a"),
                    "feature-a".to_string(),
                    Some(1_700_000_100),
                    AgentType::Codex,
                    status,
                    false,
                )
                .expect("workspace should be valid"),
            ],
            discovery_state: DiscoveryState::Ready,
            orphaned_sessions: Vec::new(),
        }
    }

    fn fixture_app() -> GroveApp {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture");
        GroveApp::from_parts_with_clipboard(
            fixture_bootstrap(WorkspaceStatus::Idle),
            Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            test_clipboard(),
            sidebar_ratio_path,
            Box::new(NullEventLogger),
            None,
        )
    }

    fn event_kinds(events: &RecordedEvents) -> Vec<String> {
        let Ok(events) = events.lock() else {
            return Vec::new();
        };
        events.iter().map(|event| event.kind.clone()).collect()
    }

    fn recorded_events(events: &RecordedEvents) -> Vec<LoggedEvent> {
        let Ok(events) = events.lock() else {
            return Vec::new();
        };
        events.clone()
    }

    fn clear_recorded_events(events: &RecordedEvents) {
        let Ok(mut events) = events.lock() else {
            return;
        };
        events.clear();
    }

    fn assert_kind_subsequence(actual: &[String], expected: &[&str]) {
        let mut expected_index = 0usize;
        for kind in actual {
            if expected_index < expected.len() && kind == expected[expected_index] {
                expected_index = expected_index.saturating_add(1);
            }
        }
        assert_eq!(
            expected_index,
            expected.len(),
            "expected subsequence {:?} in {:?}",
            expected,
            actual
        );
    }

    fn key_press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code).with_kind(KeyEventKind::Press)
    }

    fn force_tick_due(app: &mut GroveApp) {
        let now = Instant::now();
        app.next_tick_due_at = Some(now);
        app.next_poll_due_at = Some(now);
    }

    fn cmd_contains_task(cmd: &Cmd<Msg>) -> bool {
        match cmd {
            Cmd::Task(_, _) => true,
            Cmd::Batch(commands) | Cmd::Sequence(commands) => {
                commands.iter().any(cmd_contains_task)
            }
            _ => false,
        }
    }

    fn contains_spinner_frame(text: &str) -> bool {
        FAST_SPINNER_FRAMES.iter().any(|frame| text.contains(frame))
    }

    fn arb_key_event() -> impl Strategy<Value = KeyEvent> {
        proptest::prop_oneof![
            Just(key_press(KeyCode::Char('j'))),
            Just(key_press(KeyCode::Char('k'))),
            Just(key_press(KeyCode::Char('s'))),
            Just(key_press(KeyCode::Char('x'))),
            Just(key_press(KeyCode::Char('n'))),
            Just(key_press(KeyCode::Char('!'))),
            Just(key_press(KeyCode::Char('q'))),
            Just(key_press(KeyCode::Char('G'))),
            Just(key_press(KeyCode::Tab)),
            Just(key_press(KeyCode::Enter)),
            Just(key_press(KeyCode::Escape)),
            Just(key_press(KeyCode::Up)),
            Just(key_press(KeyCode::Down)),
            Just(key_press(KeyCode::PageUp)),
            Just(key_press(KeyCode::PageDown)),
            proptest::char::range('a', 'z').prop_map(|ch| key_press(KeyCode::Char(ch))),
        ]
    }

    fn arb_msg() -> impl Strategy<Value = Msg> {
        proptest::prop_oneof![
            arb_key_event().prop_map(Msg::Key),
            Just(Msg::Tick),
            Just(Msg::Noop),
            (1u16..200, 1u16..60).prop_map(|(width, height)| Msg::Resize { width, height }),
        ]
    }

    fn unique_sidebar_ratio_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        std::env::temp_dir().join(format!(
            "grove-sidebar-width-{label}-{}-{timestamp}.txt",
            std::process::id()
        ))
    }

    fn unique_temp_workspace_dir(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "grove-test-workspace-{label}-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp workspace directory should exist");
        path
    }

    fn fixture_app_with_tmux(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
    ) -> FixtureApp {
        fixture_app_with_tmux_and_sidebar_path(
            status,
            captures,
            Vec::new(),
            unique_sidebar_ratio_path("fixture-with-tmux"),
        )
    }

    fn fixture_app_with_tmux_and_sidebar_path(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
        cursor_captures: Vec<Result<String, String>>,
        sidebar_ratio_path: PathBuf,
    ) -> FixtureApp {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captures = Rc::new(RefCell::new(captures));
        let cursor_captures = Rc::new(RefCell::new(cursor_captures));
        let tmux = RecordingTmuxInput {
            commands: commands.clone(),
            captures: captures.clone(),
            cursor_captures: cursor_captures.clone(),
            calls: Rc::new(RefCell::new(Vec::new())),
        };
        (
            GroveApp::from_parts_with_clipboard(
                fixture_bootstrap(status),
                Box::new(tmux),
                test_clipboard(),
                sidebar_ratio_path,
                Box::new(NullEventLogger),
                None,
            ),
            commands,
            captures,
            cursor_captures,
        )
    }

    fn fixture_app_with_tmux_and_calls(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
        cursor_captures: Vec<Result<String, String>>,
    ) -> FixtureAppWithCalls {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture-with-calls");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captures = Rc::new(RefCell::new(captures));
        let cursor_captures = Rc::new(RefCell::new(cursor_captures));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let tmux = RecordingTmuxInput {
            commands: commands.clone(),
            captures: captures.clone(),
            cursor_captures: cursor_captures.clone(),
            calls: calls.clone(),
        };

        (
            GroveApp::from_parts_with_clipboard(
                fixture_bootstrap(status),
                Box::new(tmux),
                test_clipboard(),
                sidebar_ratio_path,
                Box::new(NullEventLogger),
                None,
            ),
            commands,
            captures,
            cursor_captures,
            calls,
        )
    }

    fn fixture_app_with_tmux_and_events(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
        cursor_captures: Vec<Result<String, String>>,
    ) -> FixtureAppWithEvents {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture-with-events");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captures = Rc::new(RefCell::new(captures));
        let cursor_captures = Rc::new(RefCell::new(cursor_captures));
        let events = Arc::new(Mutex::new(Vec::new()));
        let tmux = RecordingTmuxInput {
            commands: commands.clone(),
            captures: captures.clone(),
            cursor_captures: cursor_captures.clone(),
            calls: Rc::new(RefCell::new(Vec::new())),
        };
        let event_log = RecordingEventLogger {
            events: events.clone(),
        };

        (
            GroveApp::from_parts_with_clipboard(
                fixture_bootstrap(status),
                Box::new(tmux),
                test_clipboard(),
                sidebar_ratio_path,
                Box::new(event_log),
                None,
            ),
            commands,
            captures,
            cursor_captures,
            events,
        )
    }

    fn fixture_background_app(status: WorkspaceStatus) -> GroveApp {
        GroveApp::from_parts_with_clipboard(
            fixture_bootstrap(status),
            Box::new(BackgroundOnlyTmuxInput),
            test_clipboard(),
            unique_sidebar_ratio_path("background"),
            Box::new(NullEventLogger),
            None,
        )
    }

    fn with_rendered_frame(
        app: &GroveApp,
        width: u16,
        height: u16,
        assert_frame: impl FnOnce(&Frame),
    ) {
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(width, height, &mut pool);
        ftui::Model::view(app, &mut frame);
        assert_frame(&frame);
    }

    proptest::proptest! {
        #[test]
        fn no_panic_on_random_messages(msgs in prop::collection::vec(arb_msg(), 1..200)) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
            }
        }

        #[test]
        fn selection_always_in_bounds(msgs in prop::collection::vec(arb_msg(), 1..200)) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
                if !app.state.workspaces.is_empty() {
                    prop_assert!(app.state.selected_index < app.state.workspaces.len());
                }
            }
        }

        #[test]
        fn modal_exclusivity(msgs in prop::collection::vec(arb_msg(), 1..200)) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
                let active_modals = [app.launch_dialog.is_some(), app.create_dialog.is_some(), app.interactive.is_some()]
                    .iter()
                    .filter(|is_active| **is_active)
                    .count();
                prop_assert!(active_modals <= 1);
            }
        }

        #[test]
        fn scroll_offset_in_bounds(msgs in prop::collection::vec(arb_msg(), 1..200)) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
                prop_assert!(app.preview.offset <= app.preview.lines.len());
            }
        }

        #[test]
        fn view_never_panics(
            msgs in prop::collection::vec(arb_msg(), 0..100),
            width in 20u16..200,
            height in 5u16..60,
        ) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
            }

            let mut pool = GraphemePool::new();
            let mut frame = Frame::new(width, height, &mut pool);
            ftui::Model::view(&app, &mut frame);
        }

        #[test]
        fn view_fills_status_bar_row(msgs in prop::collection::vec(arb_msg(), 0..50)) {
            let mut app = fixture_app();
            for msg in msgs {
                let _ = ftui::Model::update(&mut app, msg);
            }

            let mut pool = GraphemePool::new();
            let mut frame = Frame::new(80, 24, &mut pool);
            ftui::Model::view(&app, &mut frame);

            let status_row = frame.height().saturating_sub(1);
            let status = row_text(&frame, status_row, 0, frame.width());
            prop_assert!(!status.is_empty(), "status bar should not be blank");
        }
    }

    #[test]
    fn sidebar_shows_workspace_names() {
        let app = fixture_app();
        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            assert!(find_row_containing(frame, "grove", x_start, x_end).is_some());
            assert!(find_row_containing(frame, "feature-a", x_start, x_end).is_some());
        });
    }

    #[test]
    fn selected_workspace_row_has_selection_marker() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let rendered_row = row_text(frame, selected_row, x_start, x_end);
            assert!(
                rendered_row.starts_with("> "),
                "selected row should start with selection marker, got: {rendered_row}"
            );
        });
    }

    #[test]
    fn active_workspace_without_recent_activity_uses_static_indicators() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = false;
        app.agent_output_changing = false;

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(
                sidebar_row_text.contains("●"),
                "active workspace should show static active icon, got: {sidebar_row_text}"
            );
            assert!(
                !contains_spinner_frame(&sidebar_row_text),
                "active workspace should not animate without recent output, got: {sidebar_row_text}"
            );

            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(
                !contains_spinner_frame(&status_text),
                "status bar should not animate without recent output, got: {status_text}"
            );
        });
    }

    #[test]
    fn active_workspace_with_recent_activity_animates_indicators() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = true;
        app.agent_output_changing = true;

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(
                contains_spinner_frame(&sidebar_row_text),
                "active workspace should animate when output is changing, got: {sidebar_row_text}"
            );
        });
    }

    #[test]
    fn activity_spinner_does_not_shift_header_or_status_layout() {
        let (mut idle_app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        idle_app.state.selected_index = 1;
        idle_app.output_changing = false;
        idle_app.agent_output_changing = false;

        let (mut active_app, _commands2, _captures2, _cursor_captures2) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        active_app.state.selected_index = 1;
        active_app.output_changing = true;
        active_app.agent_output_changing = true;

        with_rendered_frame(&idle_app, 80, 24, |idle_frame| {
            with_rendered_frame(&active_app, 80, 24, |active_frame| {
                let idle_header = row_text(idle_frame, 0, 0, idle_frame.width());
                let active_header = row_text(active_frame, 0, 0, active_frame.width());
                assert_eq!(
                    idle_header.find("[List]"),
                    active_header.find("[List]"),
                    "header mode chip column should remain stable when spinner state changes"
                );
                assert_eq!(
                    idle_header.find("[WorkspaceList]"),
                    active_header.find("[WorkspaceList]"),
                    "header focus chip column should remain stable when spinner state changes"
                );

                let idle_status_row = idle_frame.height().saturating_sub(1);
                let active_status_row = active_frame.height().saturating_sub(1);
                let idle_status = row_text(idle_frame, idle_status_row, 0, idle_frame.width());
                let active_status =
                    row_text(active_frame, active_status_row, 0, active_frame.width());
                assert_eq!(
                    idle_status, active_status,
                    "status keybind hints should remain stable when spinner state changes"
                );
            });
        });
    }

    #[test]
    fn interactive_input_echo_does_not_trigger_activity_spinner() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = true;
        app.agent_output_changing = false;

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(
                !contains_spinner_frame(&sidebar_row_text),
                "spinner should not animate for user-driven echo output, got: {sidebar_row_text}"
            );

            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(
                status_text.contains("j/k move, Enter open"),
                "status row should show keybind hints, got: {status_text}"
            );
        });
    }

    #[test]
    fn modal_dialog_renders_over_sidebar() {
        let mut app = fixture_app();
        app.launch_dialog = Some(LaunchDialogState {
            prompt: String::new(),
            skip_permissions: false,
        });

        with_rendered_frame(&app, 80, 24, |frame| {
            assert!(find_row_containing(frame, "Start Agent", 0, frame.width()).is_some());
        });
    }

    #[test]
    fn launch_dialog_uses_opaque_background_fill() {
        let mut app = fixture_app();
        app.launch_dialog = Some(LaunchDialogState {
            prompt: String::new(),
            skip_permissions: false,
        });

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(100);
            let dialog_height = 8u16;
            let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
            let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
            let probe_x = dialog_x.saturating_add(dialog_width.saturating_sub(3));
            let probe_y = dialog_y.saturating_add(4);
            let Some(cell) = frame.buffer.get(probe_x, probe_y) else {
                panic!("expected dialog probe cell at ({probe_x},{probe_y})");
            };
            assert_eq!(cell.bg, ui_theme().base);
        });
    }

    #[test]
    fn create_dialog_uses_opaque_background_fill() {
        let mut app = fixture_app();
        app.open_create_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(90);
            let dialog_height = 10u16;
            let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
            let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
            let probe_x = dialog_x.saturating_add(dialog_width.saturating_sub(3));
            let probe_y = dialog_y.saturating_add(4);
            let Some(cell) = frame.buffer.get(probe_x, probe_y) else {
                panic!("expected dialog probe cell at ({probe_x},{probe_y})");
            };
            assert_eq!(cell.bg, ui_theme().base);
        });
    }

    #[test]
    fn status_row_shows_keybind_hints_not_flash_state() {
        let mut app = fixture_app();
        app.show_flash("Agent started", false);

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(!status_text.contains("Agent started"));
            assert!(status_text.contains("j/k move, Enter open"));
        });
    }

    #[test]
    fn status_row_shows_create_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.open_create_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Tab/Shift+Tab field"));
            assert!(status_text.contains("Enter create"));
        });
    }

    #[test]
    fn status_row_shows_launch_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.launch_dialog = Some(LaunchDialogState {
            prompt: String::new(),
            skip_permissions: false,
        });

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Type prompt"));
            assert!(status_text.contains("Enter start"));
        });
    }

    #[test]
    fn preview_pane_renders_ansi_colors() {
        let mut app = fixture_app();
        app.preview.lines = vec!["Success: all tests passed".to_string()];
        app.preview.render_lines = vec!["\u{1b}[32mSuccess\u{1b}[0m: all tests passed".to_string()];

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(row) = find_row_containing(frame, "Success", x_start, x_end) else {
                panic!("success row should be present in preview pane");
            };
            let Some(s_col) = find_cell_with_char(frame, row, x_start, x_end, 'S') else {
                panic!("success row should include first character column");
            };

            assert_row_fg(frame, row, s_col, s_col.saturating_add(7), ansi_16_color(2));
        });
    }

    #[test]
    fn codex_interactive_preview_keeps_ansi_colors() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        app.interactive = Some(InteractiveState::new(
            "%1".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            34,
            78,
        ));
        app.preview.lines = vec!["Success: all tests passed".to_string()];
        app.preview.render_lines = vec!["\u{1b}[32mSuccess\u{1b}[0m: all tests passed".to_string()];

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(row) = find_row_containing(frame, "Success", x_start, x_end) else {
                panic!("success row should be present in preview pane");
            };
            let Some(s_col) = find_cell_with_char(frame, row, x_start, x_end, 'S') else {
                panic!("success row should include first character column");
            };

            assert_row_fg(frame, row, s_col, s_col.saturating_add(7), ansi_16_color(2));
        });
    }

    #[test]
    fn view_registers_hit_regions_for_panes_and_workspace_rows() {
        let app = fixture_app();
        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);

        with_rendered_frame(&app, 80, 24, |frame| {
            assert_eq!(
                frame
                    .hit_test(layout.header.x, layout.header.y)
                    .map(|hit| hit.0),
                Some(HitId::new(HIT_ID_HEADER))
            );
            assert_eq!(
                frame
                    .hit_test(layout.preview.x, layout.preview.y)
                    .map(|hit| hit.0),
                Some(HitId::new(HIT_ID_PREVIEW))
            );
            assert_eq!(
                frame
                    .hit_test(layout.status.x, layout.status.y)
                    .map(|hit| hit.0),
                Some(HitId::new(HIT_ID_STATUS))
            );
            assert_eq!(
                frame
                    .hit_test(sidebar_inner.x, sidebar_inner.y)
                    .map(|hit| hit.0),
                Some(HitId::new(HIT_ID_WORKSPACE_ROW))
            );
            assert_eq!(
                frame
                    .hit_test(sidebar_inner.x, sidebar_inner.y)
                    .map(|hit| hit.2),
                Some(0)
            );
        });
    }

    #[test]
    fn mouse_workspace_selection_uses_row_hit_data_after_render() {
        let mut app = fixture_app();
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        let second_row_y = sidebar_inner.y.saturating_add(WORKSPACE_ITEM_HEIGHT);

        with_rendered_frame(&app, 100, 40, |_frame| {});

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                sidebar_inner.x,
                second_row_y,
            )),
        );

        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn start_agent_emits_dialog_and_lifecycle_events() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let kinds = event_kinds(&events);
        assert_kind_subsequence(
            &kinds,
            &[
                "selection_changed",
                "dialog_opened",
                "dialog_confirmed",
                "agent_started",
            ],
        );
        assert!(kinds.iter().any(|kind| kind == "flash_shown"));
    }

    #[test]
    fn preview_poll_change_emits_output_changed_event() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(
                WorkspaceStatus::Active,
                vec![Ok("line one\nline two\n".to_string())],
                Vec::new(),
            );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        let kinds = event_kinds(&events);
        assert!(kinds.iter().any(|kind| kind == "output_changed"));
    }

    #[test]
    fn tick_queues_async_preview_poll_with_background_io() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("background-poll");
        let mut app = GroveApp::from_parts(
            fixture_bootstrap(WorkspaceStatus::Active),
            Box::new(BackgroundOnlyTmuxInput),
            sidebar_ratio_path,
            Box::new(NullEventLogger),
            None,
        );
        app.state.selected_index = 1;
        force_tick_due(&mut app);

        let cmd = ftui::Model::update(&mut app, Msg::Tick);
        assert!(cmd_contains_task(&cmd));
    }

    #[test]
    fn async_preview_capture_failure_sets_flash_message() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: false,
                    capture_ms: 2,
                    total_ms: 2,
                    result: Err("capture failed".to_string()),
                }),
                cursor_capture: None,
            }),
        );

        assert!(app.status_bar_line().contains("preview capture failed"));
    }

    #[test]
    fn stale_preview_poll_result_is_dropped_by_generation() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
        app.state.selected_index = 1;
        app.preview.lines = vec!["initial".to_string()];
        app.preview.render_lines = vec!["initial".to_string()];
        app.poll_generation = 2;

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: false,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Ok("stale-output\n".to_string()),
                }),
                cursor_capture: None,
            }),
        );
        assert_eq!(app.preview.lines, vec!["initial".to_string()]);
        assert!(
            event_kinds(&events)
                .iter()
                .any(|kind| kind == "stale_result_dropped")
        );

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 2,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: false,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Ok("fresh-output\n".to_string()),
                }),
                cursor_capture: None,
            }),
        );
        assert_eq!(app.preview.lines, vec!["fresh-output".to_string()]);
    }

    #[test]
    fn preview_poll_uses_cleaned_change_for_status_lane() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: true,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Ok("hello\u{1b}[?1000h\u{1b}[<35;192;47M".to_string()),
                }),
                cursor_capture: None,
            }),
        );
        assert!(app.output_changing);

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 2,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: true,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Ok("hello\u{1b}[?1000l".to_string()),
                }),
                cursor_capture: None,
            }),
        );

        assert!(!app.output_changing);
        let capture = app
            .preview
            .recent_captures
            .back()
            .expect("capture record should exist");
        assert!(capture.changed_raw);
        assert!(!capture.changed_cleaned);
    }

    #[test]
    fn preview_scroll_emits_scrolled_and_autoscroll_events() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());
        app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
        app.preview.offset = 0;
        app.preview.auto_scroll = true;

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, 90, 10)),
        );

        let kinds = event_kinds(&events);
        assert!(kinds.iter().any(|kind| kind == "scrolled"));
        assert!(kinds.iter().any(|kind| kind == "autoscroll_toggled"));
    }

    #[test]
    fn create_dialog_confirmed_event_includes_branch_payload() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        for character in ['f', 'o', 'o'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }
        for _ in 0..3 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let dialog_confirmed = recorded_events(&events)
            .into_iter()
            .find(|event| event.kind == "dialog_confirmed" && event.event == "dialog")
            .expect("dialog_confirmed event should be logged");
        assert_eq!(
            dialog_confirmed
                .data
                .get("branch_mode")
                .and_then(Value::as_str),
            Some("existing")
        );
        assert_eq!(
            dialog_confirmed
                .data
                .get("workspace_name")
                .and_then(Value::as_str),
            Some("foo")
        );
    }

    #[test]
    fn create_workspace_completed_success_queues_refresh_task_in_background_mode() {
        let mut app = fixture_background_app(WorkspaceStatus::Idle);
        let request = CreateWorkspaceRequest {
            workspace_name: "feature-x".to_string(),
            branch_mode: BranchMode::NewBranch {
                base_branch: "main".to_string(),
            },
            agent: AgentType::Claude,
        };
        let result = CreateWorkspaceResult {
            workspace_path: PathBuf::from("/repos/grove-feature-x"),
            branch: "feature-x".to_string(),
            warnings: Vec::new(),
        };

        let cmd = ftui::Model::update(
            &mut app,
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion {
                request,
                result: Ok(result),
            }),
        );

        assert!(cmd_contains_task(&cmd));
        assert!(app.refresh_in_flight);
    }

    #[test]
    fn interactive_enter_and_exit_emit_mode_events() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );

        let kinds = event_kinds(&events);
        assert_kind_subsequence(&kinds, &["interactive_entered", "interactive_exited"]);
    }

    #[test]
    fn key_q_maps_to_key_message() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press));
        assert_eq!(
            Msg::from(event),
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press))
        );
    }

    #[test]
    fn ctrl_c_maps_to_key_message() {
        let event = Event::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert_eq!(
            Msg::from(event),
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press)
            )
        );
    }

    #[test]
    fn tmux_runtime_paths_avoid_status_calls_in_tui_module() {
        let source = include_str!("tui.rs");
        let status_call_pattern = ['.', 's', 't', 'a', 't', 'u', 's', '(']
            .into_iter()
            .collect::<String>();
        assert!(
            !source.contains(&status_call_pattern),
            "runtime tmux paths should avoid status command calls to preserve one-writer discipline"
        );
    }

    #[test]
    fn tick_maps_to_tick_message() {
        assert_eq!(Msg::from(Event::Tick), Msg::Tick);
    }

    #[test]
    fn key_message_updates_model_state() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(cmd, Cmd::Tick(_)));
        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn q_quits_when_not_interactive() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn ctrl_q_quits_via_action_mapper() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('q'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn ctrl_d_quits_when_idle_via_action_mapper() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('d'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn ctrl_c_dismisses_modal_via_action_mapper() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        assert!(app.launch_dialog.is_some());

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(app.launch_dialog.is_none());
    }

    #[test]
    fn ctrl_c_with_task_running_does_not_quit() {
        let mut app = fixture_app();
        app.start_in_flight = true;

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(!matches!(cmd, Cmd::Quit));
        assert!(
            app.status_bar_line()
                .contains("cannot cancel running lifecycle task")
        );
    }

    #[test]
    fn ctrl_d_with_task_running_does_not_quit() {
        let mut app = fixture_app();
        app.start_in_flight = true;

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('d'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(!matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn start_key_launches_selected_workspace_agent() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        assert!(app.launch_dialog.is_some());
        assert!(commands.borrow().is_empty());
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().as_slice(),
            &[
                vec![
                    "tmux".to_string(),
                    "new-session".to_string(),
                    "-d".to_string(),
                    "-s".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "-c".to_string(),
                    "/repos/grove-feature-a".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "set-option".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "history-limit".to_string(),
                    "10000".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "codex".to_string(),
                    "Enter".to_string(),
                ],
            ]
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Active)
        );
    }

    #[test]
    fn background_start_confirm_queues_lifecycle_task() {
        let mut app = fixture_background_app(WorkspaceStatus::Idle);
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(cmd_contains_task(&cmd));
    }

    #[test]
    fn start_agent_completed_updates_workspace_status() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::StartAgentCompleted(StartAgentCompletion {
                workspace_name: "feature-a".to_string(),
                session_name: "grove-ws-feature-a".to_string(),
                result: Ok(()),
            }),
        );

        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Active)
        );
    }

    #[test]
    fn unsafe_toggle_changes_launch_command_flags() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('!')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
                "Enter".to_string(),
            ])
        );
        assert!(app.launch_skip_permissions);
    }

    #[test]
    fn start_key_uses_workspace_prompt_file_launcher_script() {
        let workspace_dir = unique_temp_workspace_dir("prompt");
        let prompt_path = workspace_dir.join(".grove-prompt");
        fs::write(&prompt_path, "fix bug\nand add tests").expect("prompt file should be writable");

        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
        app.state.workspaces[1].path = workspace_dir.clone();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                format!("bash {}/.grove-start.sh", workspace_dir.display()),
                "Enter".to_string(),
            ])
        );

        let launcher_path = workspace_dir.join(".grove-start.sh");
        let launcher_script =
            fs::read_to_string(&launcher_path).expect("launcher script should be written");
        assert!(launcher_script.contains("fix bug"));
        assert!(launcher_script.contains("and add tests"));
        assert!(launcher_script.contains("codex"));

        let _ = fs::remove_dir_all(workspace_dir);
    }

    #[test]
    fn start_dialog_tab_toggles_unsafe_for_launch() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
                "Enter".to_string(),
            ])
        );
    }

    #[test]
    fn start_dialog_blocks_background_navigation_and_escape_cancels() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.selected_index, 1);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(app.state.selected_index, 1);
        assert_eq!(
            app.launch_dialog
                .as_ref()
                .map(|dialog| dialog.prompt.clone()),
            Some("k".to_string())
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );

        assert!(app.launch_dialog.is_none());
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn new_workspace_key_opens_create_dialog() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Claude)
        );
        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.base_branch.clone()),
            Some("main".to_string())
        );
    }

    #[test]
    fn create_dialog_tab_cycles_focus_field() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.focused_field),
            Some(CreateDialogField::BranchInput)
        );
    }

    #[test]
    fn create_dialog_right_on_agent_field_toggles_agent() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Codex)
        );
    }

    #[test]
    fn create_dialog_branch_field_edits_base_branch_in_new_mode() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );

        for _ in 0..4 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
            );
        }
        for character in ['d', 'e', 'v'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }

        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.base_branch.clone()),
            Some("dev".to_string())
        );
    }

    #[test]
    fn create_dialog_can_toggle_existing_mode_and_edit_existing_branch() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        for _ in 0..3 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        for character in ['f', 'e', 'a', 't', '/', 'x'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }

        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.branch_mode),
            Some(CreateBranchMode::ExistingBranch)
        );
        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.existing_branch.clone()),
            Some("feat/x".to_string())
        );
    }

    #[test]
    fn create_dialog_blocks_navigation_and_escape_cancels() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(app.state.selected_index, 0);
        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.workspace_name.clone()),
            Some("j".to_string())
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        assert!(app.create_dialog.is_none());
    }

    #[test]
    fn create_dialog_enter_without_name_shows_validation_flash() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.create_dialog.is_some());
        assert!(app.status_bar_line().contains("workspace name is required"));
    }

    #[test]
    fn create_dialog_existing_mode_without_branch_shows_validation_flash() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        for character in ['f', 'o', 'o'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }
        for _ in 0..3 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.create_dialog.is_some());
        assert!(
            app.status_bar_line()
                .contains("existing branch is required")
        );
    }

    #[test]
    fn stop_key_stops_selected_workspace_agent() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().as_slice(),
            &[
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "C-c".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                ],
            ]
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Idle)
        );
    }

    #[test]
    fn background_stop_key_queues_lifecycle_task() {
        let mut app = fixture_background_app(WorkspaceStatus::Active);
        app.state.selected_index = 1;

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        assert!(cmd_contains_task(&cmd));
    }

    #[test]
    fn stop_agent_completed_updates_workspace_status_and_exits_interactive() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        app.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            34,
            78,
        ));

        ftui::Model::update(
            &mut app,
            Msg::StopAgentCompleted(StopAgentCompletion {
                workspace_name: "feature-a".to_string(),
                session_name: "grove-ws-feature-a".to_string(),
                result: Ok(()),
            }),
        );

        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Idle)
        );
        assert!(app.interactive.is_none());
    }

    #[test]
    fn start_key_ignores_main_workspace() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
        assert!(app.launch_dialog.is_none());
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Main)
        );
        assert!(
            app.status_bar_line()
                .contains("cannot start agent in main workspace")
        );
    }

    #[test]
    fn start_key_on_running_workspace_shows_flash_and_no_dialog() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );

        assert!(app.launch_dialog.is_none());
        assert!(commands.borrow().is_empty());
        assert!(app.status_bar_line().contains("agent already running"));
    }

    #[test]
    fn stop_key_without_running_agent_shows_flash() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
        assert!(app.status_bar_line().contains("no agent running"));
    }

    #[test]
    fn enter_on_active_workspace_starts_interactive_mode() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_some());
        assert_eq!(app.mode_label(), "Interactive");
    }

    #[test]
    fn enter_on_active_workspace_resizes_tmux_session_to_preview_dimensions() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(WorkspaceStatus::Active, Vec::new(), Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "resize:grove-ws-feature-a:78:34")
        );
    }

    #[test]
    fn enter_interactive_immediately_polls_preview_and_cursor() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![Ok("entered\n".to_string())],
                vec![Ok("1 0 0 78 34".to_string())],
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "capture:grove-ws-feature-a:600:true")
        );
        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "cursor:grove-ws-feature-a")
        );
    }

    #[test]
    fn resize_in_interactive_mode_immediately_resizes_and_polls() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![Ok("entered\n".to_string()), Ok("resized\n".to_string())],
                vec![Ok("1 0 0 78 34".to_string()), Ok("1 0 0 58 34".to_string())],
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        calls.borrow_mut().clear();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 80,
                height: 40,
            },
        );

        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call.starts_with("resize:grove-ws-feature-a:"))
        );
        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "capture:grove-ws-feature-a:600:true")
        );
    }

    #[test]
    fn resize_verify_retries_once_then_stops() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![Ok("after-retry\n".to_string())],
                vec![Ok("1 0 0 70 20".to_string())],
            );
        app.state.selected_index = 1;
        app.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            34,
            78,
        ));
        app.pending_resize_verification = Some(PendingResizeVerification {
            session: "grove-ws-feature-a".to_string(),
            expected_width: 78,
            expected_height: 34,
            retried: false,
        });

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: None,
                cursor_capture: Some(CursorCapture {
                    session: "grove-ws-feature-a".to_string(),
                    capture_ms: 1,
                    result: Ok("1 0 0 70 20".to_string()),
                }),
            }),
        );

        let resize_retries = calls
            .borrow()
            .iter()
            .filter(|call| *call == "resize:grove-ws-feature-a:78:34")
            .count();
        assert_eq!(resize_retries, 1);
        assert!(app.pending_resize_verification.is_none());
    }

    #[test]
    fn interactive_keys_forward_to_tmux_session() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
        );

        assert!(matches!(cmd, Cmd::Tick(_)));
        assert_eq!(
            commands.borrow().as_slice(),
            &[vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "q".to_string(),
            ]]
        );
    }

    #[test]
    fn interactive_filters_split_mouse_bracket_fragment() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let Some(state) = app.interactive.as_mut() else {
            panic!("interactive state should be active");
        };
        state.note_mouse_event(Instant::now());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('[')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn interactive_filters_split_mouse_fragment_without_opening_bracket() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let Some(state) = app.interactive.as_mut() else {
            panic!("interactive state should be active");
        };
        state.note_mouse_event(Instant::now());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('<')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn interactive_filters_boundary_marker_before_split_mouse_fragment() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let Some(state) = app.interactive.as_mut() else {
            panic!("interactive state should be active");
        };
        state.note_mouse_event(Instant::now());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('M')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn interactive_still_forwards_bracket_when_not_mouse_fragment() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('[')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().as_slice(),
            &[vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "[".to_string(),
            ]]
        );
    }

    #[test]
    fn double_escape_exits_interactive_mode() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_none());
        assert_eq!(
            commands.borrow().as_slice(),
            &[vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "Escape".to_string(),
            ]]
        );
    }

    #[test]
    fn ctrl_backslash_exits_interactive_mode() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('\\'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(app.interactive.is_none());
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn ctrl_backslash_control_character_exits_interactive_mode() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('\u{1c}')).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_none());
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn ctrl_four_exits_interactive_mode() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('4'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(app.interactive.is_none());
        assert!(commands.borrow().is_empty());
    }

    #[test]
    fn interactive_key_schedules_debounced_poll_interval() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        match cmd {
            Cmd::Tick(interval) => {
                assert!(
                    interval <= Duration::from_millis(20) && interval >= Duration::from_millis(15),
                    "expected debounced interactive interval near 20ms, got {interval:?}"
                );
            }
            _ => panic!("expected Cmd::Tick from interactive key update"),
        }
    }

    #[test]
    fn interactive_key_does_not_postpone_existing_due_tick() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let first_cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(first_cmd, Cmd::Tick(_)));
        let first_due = app
            .next_tick_due_at
            .expect("first key should schedule a due tick");

        std::thread::sleep(Duration::from_millis(1));

        let second_cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('y')).with_kind(KeyEventKind::Press)),
        );
        let second_due = app
            .next_tick_due_at
            .expect("second key should retain an existing due tick");

        assert!(
            second_due <= first_due,
            "second key should not postpone existing due tick"
        );
        assert!(
            matches!(second_cmd, Cmd::None),
            "when a sooner tick is already pending, no new timer should be scheduled"
        );
    }

    #[test]
    fn interactive_update_flow_sequences_tick_copy_paste_and_exit() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![
                    Ok("initial-preview".to_string()),
                    Ok("preview-output".to_string()),
                    Ok("copied-text".to_string()),
                ],
                vec![Ok("1 0 0 78 34".to_string())],
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        calls.borrow_mut().clear();

        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('v'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            calls.borrow().as_slice(),
            &[
                "capture:grove-ws-feature-a:600:true".to_string(),
                "cursor:grove-ws-feature-a".to_string(),
                "exec:tmux send-keys -l -t grove-ws-feature-a x".to_string(),
                "capture:grove-ws-feature-a:200:true".to_string(),
                "exec:tmux send-keys -l -t grove-ws-feature-a copied-text".to_string(),
                "exec:tmux send-keys -t grove-ws-feature-a Escape".to_string(),
            ]
        );
        assert!(app.interactive.is_none());
    }

    #[test]
    fn interactive_input_latency_correlates_forwarded_key_with_preview_update() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(
                WorkspaceStatus::Active,
                vec![
                    Ok("initial-preview".to_string()),
                    Ok("initial-preview\nx".to_string()),
                ],
                vec![Ok("1 0 0 120 40".to_string())],
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        clear_recorded_events(&events);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        let recorded = recorded_events(&events);
        let forwarded = recorded
            .iter()
            .find(|event| event.event == "input" && event.kind == "interactive_forwarded")
            .expect("forwarded input event should be logged");
        let seq = forwarded
            .data
            .get("seq")
            .and_then(Value::as_u64)
            .expect("forwarded input should include seq");

        let latency = recorded
            .iter()
            .find(|event| event.event == "input" && event.kind == "interactive_input_to_preview")
            .expect("input latency event should be logged");
        assert_eq!(latency.data.get("seq").and_then(Value::as_u64), Some(seq));
        assert!(
            latency
                .data
                .get("input_to_preview_ms")
                .and_then(Value::as_u64)
                .is_some()
        );
        assert!(
            latency
                .data
                .get("tmux_to_preview_ms")
                .and_then(Value::as_u64)
                .is_some()
        );

        let output_changed = recorded
            .iter()
            .find(|event| event.event == "preview_update" && event.kind == "output_changed")
            .expect("preview update event should be logged");
        assert_eq!(
            output_changed.data.get("input_seq").and_then(Value::as_u64),
            Some(seq)
        );
    }

    #[test]
    fn preview_update_logs_coalesced_input_range() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(
                WorkspaceStatus::Active,
                vec![
                    Ok("initial-preview".to_string()),
                    Ok("initial-preview\nab".to_string()),
                ],
                vec![Ok("1 0 0 120 40".to_string())],
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        clear_recorded_events(&events);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('a')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('b')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        let recorded = recorded_events(&events);
        let output_changed = recorded
            .iter()
            .find(|event| event.event == "preview_update" && event.kind == "output_changed")
            .expect("preview update event should be logged");
        assert_eq!(
            output_changed
                .data
                .get("consumed_input_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            output_changed
                .data
                .get("consumed_input_seq_first")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            output_changed
                .data
                .get("consumed_input_seq_last")
                .and_then(Value::as_u64),
            Some(2)
        );

        let coalesced = recorded
            .iter()
            .find(|event| event.event == "input" && event.kind == "interactive_inputs_coalesced")
            .expect("coalesced input event should be logged");
        assert_eq!(
            coalesced
                .data
                .get("consumed_input_count")
                .and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn tick_logs_skip_reason_when_not_due() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
        clear_recorded_events(&events);

        app.next_tick_due_at = Some(Instant::now() + Duration::from_secs(10));
        app.next_tick_interval_ms = Some(10_000);
        ftui::Model::update(&mut app, Msg::Tick);

        let recorded = recorded_events(&events);
        let skipped = recorded
            .iter()
            .find(|event| event.event == "tick" && event.kind == "skipped")
            .expect("tick skip event should be logged");
        assert_eq!(
            skipped.data.get("reason").and_then(Value::as_str),
            Some("not_due")
        );
        assert_eq!(
            skipped.data.get("interval_ms").and_then(Value::as_u64),
            Some(10_000)
        );
    }

    #[test]
    fn interactive_exit_clears_pending_input_traces() {
        let (mut app, _commands, _captures, _cursor_captures, events) =
            fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        clear_recorded_events(&events);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('\\'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        let recorded = recorded_events(&events);
        let cleared = recorded
            .iter()
            .find(|event| event.event == "input" && event.kind == "pending_inputs_cleared")
            .expect("pending traces should be cleared when interactive exits");
        assert_eq!(
            cleared.data.get("session").and_then(Value::as_str),
            Some("grove-ws-feature-a")
        );
        assert!(
            cleared
                .data
                .get("cleared")
                .and_then(Value::as_u64)
                .is_some_and(|value| value > 0)
        );
    }

    #[test]
    fn codex_live_preview_capture_keeps_tmux_escape_output() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![
                    Ok("line one\nline two\n".to_string()),
                    Ok("line one\nline two\n".to_string()),
                ],
                Vec::new(),
            );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "capture:grove-ws-feature-a:600:true")
        );
    }

    #[test]
    fn claude_live_preview_capture_keeps_tmux_escape_output() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![
                    Ok("line one\nline two\n".to_string()),
                    Ok("line one\nline two\n".to_string()),
                ],
                Vec::new(),
            );
        app.state.workspaces[1].agent = AgentType::Claude;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        assert!(
            calls
                .borrow()
                .iter()
                .any(|call| call == "capture:grove-ws-feature-a:600:true")
        );
    }

    #[test]
    fn tick_polls_live_tmux_output_into_preview() {
        let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux(
            WorkspaceStatus::Active,
            vec![
                Ok("line one\nline two\n".to_string()),
                Ok("line one\nline two\n".to_string()),
            ],
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        assert_eq!(
            app.preview.lines,
            vec!["line one".to_string(), "line two".to_string()]
        );
    }

    #[test]
    fn stale_tick_before_due_is_ignored() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![Ok("line".to_string())],
                Vec::new(),
            );

        app.state.selected_index = 1;
        app.next_tick_due_at = Some(Instant::now() + Duration::from_secs(5));

        let cmd = ftui::Model::update(&mut app, Msg::Tick);

        assert!(matches!(cmd, Cmd::None));
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn parse_cursor_metadata_requires_five_fields() {
        assert_eq!(
            parse_cursor_metadata("1 4 2 120 40"),
            Some(super::CursorMetadata {
                cursor_visible: true,
                cursor_col: 4,
                cursor_row: 2,
                pane_width: 120,
                pane_height: 40,
            })
        );
        assert!(parse_cursor_metadata("1 4 2 120").is_none());
        assert!(parse_cursor_metadata("invalid").is_none());
    }

    #[test]
    fn ansi_line_parser_preserves_text_and_styles() {
        let line = ansi_line_to_styled_line("a\u{1b}[31mb\u{1b}[0mc");
        assert_eq!(line.to_plain_text(), "abc");
        assert_eq!(line.spans().len(), 3);
        assert_eq!(line.spans()[1].as_str(), "b");
        assert_eq!(
            line.spans()[1].style.and_then(|style| style.fg),
            Some(ansi_16_color(1))
        );
    }

    #[test]
    fn tick_polls_cursor_metadata_and_renders_overlay() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("cursor-overlay");
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Active,
                vec![
                    Ok("first\nsecond\nthird\n".to_string()),
                    Ok("first\nsecond\nthird\n".to_string()),
                ],
                vec![Ok("1 1 1 78 34".to_string()), Ok("1 1 1 78 34".to_string())],
                sidebar_ratio_path,
            );
        app.state.workspaces[1].agent = AgentType::Claude;
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        force_tick_due(&mut app);
        ftui::Model::update(&mut app, Msg::Tick);

        let rendered = app.shell_lines(8).join("\n");
        assert_eq!(
            app.interactive.as_ref().map(|state| (
                state.cursor_row,
                state.cursor_col,
                state.pane_height
            )),
            Some((1, 1, 34))
        );
        assert!(rendered.contains("s|econd"), "{rendered}");
    }

    #[test]
    fn divider_ratio_persists_across_app_instances() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("persist");
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Idle,
                Vec::new(),
                Vec::new(),
                sidebar_ratio_path.clone(),
            );

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                33,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                52,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 52);
        assert_eq!(
            fs::read_to_string(&sidebar_ratio_path).expect("ratio file should be written"),
            "52"
        );

        let (app_reloaded, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Idle,
                Vec::new(),
                Vec::new(),
                sidebar_ratio_path.clone(),
            );

        assert_eq!(app_reloaded.sidebar_width_pct, 52);
        let _ = fs::remove_file(sidebar_ratio_path);
    }

    #[test]
    fn mouse_click_on_list_selects_workspace() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                5,
                4,
            )),
        );

        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn mouse_drag_on_divider_updates_sidebar_ratio() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                33,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                55,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 55);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                55,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                20,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 55);
    }

    #[test]
    fn mouse_drag_near_divider_still_updates_sidebar_ratio() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                32,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                50,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 50);
    }

    #[test]
    fn mouse_scroll_in_preview_scrolls_output() {
        let mut app = fixture_app();
        app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
        app.preview.offset = 0;
        app.preview.auto_scroll = true;

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, 90, 10)),
        );

        assert!(app.preview.offset > 0);
        assert!(!app.preview.auto_scroll);
    }

    #[test]
    fn mouse_drag_in_interactive_preview_highlights_selected_text() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        app.preview.lines = vec!["alpha beta".to_string()];
        app.preview.render_lines = vec!["\u{1b}[32malpha beta\u{1b}[0m".to_string()];

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                preview_inner.x,
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );

        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);
        with_rendered_frame(&app, 100, 40, |frame| {
            let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end) else {
                panic!("output row should be rendered");
            };
            let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a')
            else {
                panic!("selected output row should include first char");
            };

            assert_row_bg(
                frame,
                output_row,
                first_col,
                first_col.saturating_add(5),
                ui_theme().surface1,
            );
            assert_row_fg(
                frame,
                output_row,
                first_col,
                first_col.saturating_add(5),
                ansi_16_color(2),
            );
        });
    }

    #[test]
    fn mouse_move_then_release_highlights_selected_text_without_drag_event() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        app.preview.lines = vec!["alpha beta".to_string()];
        app.preview.render_lines = vec!["\u{1b}[32malpha beta\u{1b}[0m".to_string()];

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                preview_inner.x,
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Moved,
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );

        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);
        with_rendered_frame(&app, 100, 40, |frame| {
            let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end) else {
                panic!("output row should be rendered");
            };
            let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a')
            else {
                panic!("selected output row should include first char");
            };

            assert_row_bg(
                frame,
                output_row,
                first_col,
                first_col.saturating_add(5),
                ui_theme().surface1,
            );
            assert_row_fg(
                frame,
                output_row,
                first_col,
                first_col.saturating_add(5),
                ansi_16_color(2),
            );
        });
    }

    #[test]
    fn mouse_drag_selection_overrides_existing_ansi_background_sequences() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        app.preview.lines = vec!["abc".to_string()];
        app.preview.render_lines = vec![
            "\u{1b}[48;2;30;35;50ma\u{1b}[48;2;30;35;50mb\u{1b}[48;2;30;35;50mc\u{1b}[0m"
                .to_string(),
        ];

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                preview_inner.x,
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                preview_inner.x.saturating_add(2),
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                preview_inner.x.saturating_add(2),
                select_y,
            )),
        );

        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);
        with_rendered_frame(&app, 100, 40, |frame| {
            let Some(output_row) = find_row_containing(frame, "abc", x_start, x_end) else {
                panic!("output row should be rendered");
            };
            let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a')
            else {
                panic!("selected output row should include first char");
            };

            assert_row_bg(
                frame,
                output_row,
                first_col,
                first_col.saturating_add(3),
                ui_theme().surface1,
            );
        });
    }

    #[test]
    fn alt_copy_then_alt_paste_uses_mouse_selected_preview_text() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, vec![Ok(String::new())]);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        app.preview.lines = vec!["alpha beta".to_string()];
        app.preview.render_lines = app.preview.lines.clone();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                preview_inner.x,
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                preview_inner.x.saturating_add(4),
                select_y,
            )),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('v'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(!app.preview_selection.has_selection());
        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "paste-buffer".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "alpha".to_string(),
            ])
        );
    }

    #[test]
    fn bracketed_paste_event_forwards_wrapped_literal() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(&mut app, Msg::Paste(PasteEvent::bracketed("hello\nworld")));

        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "\u{1b}[200~hello\nworld\u{1b}[201~".to_string(),
            ])
        );
    }

    #[test]
    fn alt_copy_then_alt_paste_uses_visible_preview_text_when_no_selection() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("alt-copy-paste");
        let (mut app, commands, captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Active,
                vec![Ok(String::new())],
                vec![Ok("1 0 0 78 34".to_string())],
                sidebar_ratio_path,
            );
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        app.preview.lines = vec!["copy me".to_string()];
        app.preview.render_lines = app.preview.lines.clone();

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('v'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(captures.borrow().is_empty());
        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "paste-buffer".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "copy me".to_string(),
            ])
        );
    }

    #[test]
    fn shell_contains_list_preview_and_status_placeholders() {
        let app = fixture_app();
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("Workspaces"));
        assert!(content.contains("Preview Pane"));
        assert!(content.contains("Status:"));
        assert!(content.contains("feature-a | feature-a | /repos/grove-feature-a"));
        assert!(content.contains("Workspace: grove"));
    }

    #[test]
    fn shell_renders_discovery_error_state() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("error-state");
        let app = GroveApp::from_parts(
            BootstrapData {
                repo_name: "grove".to_string(),
                workspaces: Vec::new(),
                discovery_state: DiscoveryState::Error("fatal: not a git repository".to_string()),
                orphaned_sessions: Vec::new(),
            },
            Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            sidebar_ratio_path,
            Box::new(NullEventLogger),
            None,
        );
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("discovery failed"));
        assert!(content.contains("discovery error"));
    }

    #[test]
    fn preview_mode_keys_scroll_and_jump_to_bottom() {
        let mut app = fixture_app();
        app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
        app.preview.render_lines = app.preview.lines.clone();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.mode, crate::state::UiMode::Preview);

        let was_auto_scroll = app.preview.auto_scroll;
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );
        assert!(was_auto_scroll);
        assert!(!app.preview.auto_scroll);
        assert!(app.preview.offset > 0);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('G')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.preview.offset, 0);
        assert!(app.preview.auto_scroll);
    }

    #[test]
    fn preview_mode_scroll_keys_noop_when_content_fits_viewport() {
        let mut app = fixture_app();
        app.preview.lines = (1..=4).map(|value| value.to_string()).collect();
        app.preview.render_lines = app.preview.lines.clone();
        app.preview.offset = 0;
        app.preview.auto_scroll = true;

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(app.preview.offset, 0);
        assert!(app.preview.auto_scroll);
    }

    #[test]
    fn frame_debug_record_logs_every_view() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("frame-log");
        let events = Arc::new(Mutex::new(Vec::new()));
        let event_log = RecordingEventLogger {
            events: events.clone(),
        };
        let app = GroveApp::from_parts(
            fixture_bootstrap(WorkspaceStatus::Idle),
            Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            sidebar_ratio_path,
            Box::new(event_log),
            Some(1_771_023_000_000),
        );

        with_rendered_frame(&app, 100, 40, |_frame| {});
        with_rendered_frame(&app, 100, 40, |_frame| {});

        let recorded = recorded_events(&events);
        let frame_events: Vec<LoggedEvent> = recorded
            .into_iter()
            .filter(|event| event.event == "frame" && event.kind == "rendered")
            .collect();
        assert_eq!(frame_events.len(), 2);
        assert_eq!(
            frame_events[0].data.get("seq").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            frame_events[1].data.get("seq").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            frame_events[0]
                .data
                .get("app_start_ts")
                .and_then(Value::as_u64),
            Some(1_771_023_000_000)
        );
    }

    #[test]
    fn frame_debug_record_includes_frame_lines() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("frame-lines");
        let events = Arc::new(Mutex::new(Vec::new()));
        let event_log = RecordingEventLogger {
            events: events.clone(),
        };
        let mut app = GroveApp::from_parts(
            fixture_bootstrap(WorkspaceStatus::Idle),
            Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            sidebar_ratio_path,
            Box::new(event_log),
            Some(1_771_023_000_123),
        );
        app.preview.lines = vec!["render-check 🧪".to_string()];
        app.preview.render_lines = app.preview.lines.clone();

        with_rendered_frame(&app, 80, 24, |_frame| {});

        let frame_event = recorded_events(&events)
            .into_iter()
            .find(|event| event.event == "frame" && event.kind == "rendered")
            .expect("frame event should be present");

        let lines = frame_event
            .data
            .get("frame_lines")
            .and_then(Value::as_array)
            .expect("frame_lines should be array");
        assert!(lines.iter().any(|line| {
            line.as_str()
                .is_some_and(|text| text.contains("render-check 🧪"))
        }));
        assert!(frame_event.data.get("frame_hash").is_some());
        assert_eq!(
            frame_event.data.get("degradation").and_then(Value::as_str),
            Some("Full")
        );
        assert!(
            frame_event
                .data
                .get("non_empty_line_count")
                .and_then(Value::as_u64)
                .is_some_and(|count| count > 0)
        );
    }
}
