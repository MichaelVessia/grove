use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
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

use crate::adapters::{
    BootstrapData, CommandGitAdapter, CommandMultiplexerAdapter, CommandSystemAdapter,
    DiscoveryState, MultiplexerAdapter, bootstrap_data,
};
use crate::agent_runtime::{
    LaunchRequest, OutputDigest, SessionActivity, build_launch_plan,
    detect_status_with_session_override, evaluate_capture_change, poll_interval,
    session_name_for_workspace_in_project, stop_plan, zellij_capture_log_path, zellij_config_path,
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
use crate::mouse::{
    clamp_sidebar_ratio, parse_sidebar_ratio, ratio_from_drag, serialize_sidebar_ratio,
};
use crate::preview::PreviewState;
use crate::state::{Action, AppState, PaneFocus, UiMode, reduce};
use crate::workspace_lifecycle::{
    BranchMode, CommandGitRunner, CommandSetupScriptRunner, CreateWorkspaceRequest,
    CreateWorkspaceResult, WorkspaceLifecycleError, create_workspace, write_workspace_agent_marker,
};
use crate::zellij_emulator::ZellijPreviewEmulator;

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

fn modal_labeled_input_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
) -> FtLine {
    let row_bg = if focused { theme.surface1 } else { theme.base };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let value_raw = if value.is_empty() { placeholder } else { value };
    let rendered = truncate_to_display_width(value_raw, content_width.saturating_sub(prefix_width));
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    theme.yellow
                } else {
                    theme.overlay0
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(badge, Style::new().fg(theme.blue).bg(row_bg).bold()),
        FtSpan::styled(
            rendered,
            Style::new()
                .fg(if value.is_empty() {
                    theme.overlay0
                } else {
                    theme.text
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

fn modal_static_badged_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine {
    let badge = format!("[{label}] ");
    let prefix = format!("  {badge}");
    let available = content_width.saturating_sub(text_display_width(prefix.as_str()));
    let rendered = truncate_to_display_width(value, available);
    let used =
        text_display_width(prefix.as_str()).saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled("  ", Style::new().bg(theme.base)),
        FtSpan::styled(badge, Style::new().fg(badge_fg).bg(theme.base).bold()),
        FtSpan::styled(rendered, Style::new().fg(value_fg).bg(theme.base)),
        FtSpan::styled(pad, Style::new().bg(theme.base)),
    ])
}

fn modal_focus_badged_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    focused: bool,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine {
    let row_bg = if focused { theme.surface1 } else { theme.base };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let rendered = truncate_to_display_width(value, content_width.saturating_sub(prefix_width));
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    theme.yellow
                } else {
                    theme.overlay0
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(badge, Style::new().fg(badge_fg).bg(row_bg).bold()),
        FtSpan::styled(rendered, Style::new().fg(value_fg).bg(row_bg).bold()),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

fn modal_actions_row(
    content_width: usize,
    theme: UiTheme,
    primary_label: &str,
    secondary_label: &str,
    primary_focused: bool,
    secondary_focused: bool,
) -> FtLine {
    let actions_bg = if primary_focused || secondary_focused {
        theme.surface1
    } else {
        theme.base
    };
    let actions_prefix = if primary_focused || secondary_focused {
        "> "
    } else {
        "  "
    };
    let primary = if primary_focused {
        format!("[{primary_label}]")
    } else {
        format!(" {primary_label} ")
    };
    let secondary = if secondary_focused {
        format!("[{secondary_label}]")
    } else {
        format!(" {secondary_label} ")
    };
    let row = pad_or_truncate_to_display_width(
        format!("{actions_prefix}{primary}   {secondary}").as_str(),
        content_width,
    );

    FtLine::from_spans(vec![FtSpan::styled(
        row,
        Style::new().fg(theme.text).bg(actions_bg).bold(),
    )])
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
struct LaunchDialogState {
    prompt: String,
    pre_launch_command: String,
    skip_permissions: bool,
    focused_field: LaunchDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeleteDialogState {
    project_name: Option<String>,
    project_path: Option<PathBuf>,
    workspace_name: String,
    branch: String,
    path: PathBuf,
    is_missing: bool,
    delete_local_branch: bool,
    focused_field: DeleteDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeleteDialogField {
    DeleteLocalBranch,
    DeleteButton,
    CancelButton,
}

impl DeleteDialogField {
    fn next(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::DeleteButton,
            Self::DeleteButton => Self::CancelButton,
            Self::CancelButton => Self::DeleteLocalBranch,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::CancelButton,
            Self::DeleteButton => Self::DeleteLocalBranch,
            Self::CancelButton => Self::DeleteButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchDialogField {
    Prompt,
    PreLaunchCommand,
    Unsafe,
    StartButton,
    CancelButton,
}

impl LaunchDialogField {
    fn next(self) -> Self {
        match self {
            Self::Prompt => Self::PreLaunchCommand,
            Self::PreLaunchCommand => Self::Unsafe,
            Self::Unsafe => Self::StartButton,
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::Prompt,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Prompt => Self::CancelButton,
            Self::PreLaunchCommand => Self::Prompt,
            Self::Unsafe => Self::PreLaunchCommand,
            Self::StartButton => Self::Unsafe,
            Self::CancelButton => Self::StartButton,
        }
    }

    #[cfg(test)]
    fn label(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::PreLaunchCommand => "pre_launch_command",
            Self::Unsafe => "unsafe",
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateDialogState {
    workspace_name: String,
    project_index: usize,
    agent: AgentType,
    base_branch: String,
    focused_field: CreateDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditDialogState {
    workspace_name: String,
    workspace_path: PathBuf,
    branch: String,
    agent: AgentType,
    was_running: bool,
    focused_field: EditDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectDialogState {
    filter: String,
    filtered_project_indices: Vec<usize>,
    selected_filtered_index: usize,
    add_dialog: Option<ProjectAddDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectAddDialogState {
    name: String,
    path: String,
    focused_field: ProjectAddDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectAddDialogField {
    Name,
    Path,
    AddButton,
    CancelButton,
}

impl ProjectAddDialogField {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Path,
            Self::Path => Self::AddButton,
            Self::AddButton => Self::CancelButton,
            Self::CancelButton => Self::Name,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Name => Self::CancelButton,
            Self::Path => Self::Name,
            Self::AddButton => Self::Path,
            Self::CancelButton => Self::AddButton,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SettingsDialogState {
    multiplexer: MultiplexerKind,
    focused_field: SettingsDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsDialogField {
    Multiplexer,
    SaveButton,
    CancelButton,
}

impl SettingsDialogField {
    fn next(self) -> Self {
        match self {
            Self::Multiplexer => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Multiplexer,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Multiplexer => Self::CancelButton,
            Self::SaveButton => Self::Multiplexer,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateDialogField {
    WorkspaceName,
    Project,
    BaseBranch,
    Agent,
    CreateButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditDialogField {
    Agent,
    SaveButton,
    CancelButton,
}

impl EditDialogField {
    fn next(self) -> Self {
        match self {
            Self::Agent => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Agent,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Agent => Self::CancelButton,
            Self::SaveButton => Self::Agent,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

impl CreateDialogField {
    fn next(self) -> Self {
        match self {
            Self::WorkspaceName => Self::Project,
            Self::Project => Self::BaseBranch,
            Self::BaseBranch => Self::Agent,
            Self::Agent => Self::CreateButton,
            Self::CreateButton => Self::CancelButton,
            Self::CancelButton => Self::WorkspaceName,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::WorkspaceName => Self::CancelButton,
            Self::Project => Self::WorkspaceName,
            Self::BaseBranch => Self::Project,
            Self::Agent => Self::BaseBranch,
            Self::CreateButton => Self::Agent,
            Self::CancelButton => Self::CreateButton,
        }
    }

    #[cfg(test)]
    fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::Project => "project",
            Self::BaseBranch => "base_branch",
            Self::Agent => "agent",
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone)]
struct OverlayModalContent<'a> {
    title: &'a str,
    body: FtText,
    theme: UiTheme,
    border_color: PackedRgba,
}

impl Widget for OverlayModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new().bg(self.theme.base).fg(self.theme.text);

        // set_style_area preserves glyphs, so clear with spaces first.
        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title(self.title)
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(self.border_color).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        Paragraph::new(self.body.clone())
            .style(content_style)
            .render(inner, frame);
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

    fn run_write_command(program: &str, args: &[&str], text: &str) -> Result<(), String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("{program}: {error}"))?;

        let Some(mut stdin) = child.stdin.take() else {
            return Err(format!("{program}: failed to open stdin"));
        };
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("{program}: {error}"))?;
        drop(stdin);

        let status = child
            .wait()
            .map_err(|error| format!("{program}: {error}"))?;
        if status.success() {
            return Ok(());
        }

        Err(format!("{program}: exited with status {status}"))
    }

    fn run_read_command(program: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|error| format!("{program}: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                return Err(format!("{program}: exited with status {}", output.status));
            }
            return Err(format!("{program}: {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn write_text_with_platform_command(text: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            return Self::run_write_command("pbcopy", &[], text);
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
            let candidates: &[(&str, &[&str])] = if wayland {
                &[
                    ("wl-copy", &[]),
                    ("xclip", &["-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--input"]),
                ]
            } else {
                &[
                    ("xclip", &["-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--input"]),
                    ("wl-copy", &[]),
                ]
            };

            let mut errors: Vec<String> = Vec::new();
            for (program, args) in candidates {
                match Self::run_write_command(program, args, text) {
                    Ok(()) => return Ok(()),
                    Err(error) => errors.push(error),
                }
            }

            Err(errors.join("; "))
        }

        #[cfg(not(any(target_os = "macos", unix)))]
        {
            Err("platform clipboard command unavailable".to_string())
        }
    }

    fn read_text_with_platform_command() -> Result<String, String> {
        #[cfg(target_os = "macos")]
        {
            return Self::run_read_command("pbpaste", &[]);
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
            let candidates: &[(&str, &[&str])] = if wayland {
                &[
                    ("wl-paste", &["--no-newline"]),
                    ("xclip", &["-o", "-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--output"]),
                ]
            } else {
                &[
                    ("xclip", &["-o", "-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--output"]),
                    ("wl-paste", &["--no-newline"]),
                ]
            };

            let mut errors: Vec<String> = Vec::new();
            for (program, args) in candidates {
                match Self::run_read_command(program, args) {
                    Ok(text) => return Ok(text),
                    Err(error) => errors.push(error),
                }
            }

            Err(errors.join("; "))
        }

        #[cfg(not(any(target_os = "macos", unix)))]
        {
            Err("platform clipboard command unavailable".to_string())
        }
    }
}

impl ClipboardAccess for SystemClipboardAccess {
    fn read_text(&mut self) -> Result<String, String> {
        match Self::read_text_with_platform_command() {
            Ok(text) => Ok(text),
            Err(command_error) => self
                .clipboard()?
                .get_text()
                .map_err(|error| format!("{command_error}; arboard: {error}")),
        }
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        match Self::write_text_with_platform_command(text) {
            Ok(()) => Ok(()),
            Err(command_error) => self
                .clipboard()?
                .set_text(text.to_string())
                .map_err(|error| format!("{command_error}; arboard: {error}")),
        }
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
    DeleteWorkspaceCompleted(DeleteWorkspaceCompletion),
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
    workspace_status_captures: Vec<WorkspaceStatusCapture>,
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
struct WorkspaceStatusPollTarget {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    supported_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceStatusCapture {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    supported_agent: bool,
    capture_ms: u64,
    result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RefreshWorkspacesCompletion {
    preferred_workspace_path: Option<PathBuf>,
    bootstrap: BootstrapData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeleteWorkspaceCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    result: Result<(), String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateWorkspaceCompletion {
    request: CreateWorkspaceRequest,
    result: Result<CreateWorkspaceResult, WorkspaceLifecycleError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartAgentCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StopAgentCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
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
        false
    }
}

#[derive(Default)]
struct CommandZellijInput {
    pane_sizes: Mutex<HashMap<String, (u16, u16)>>,
    emulator: Mutex<ZellijPreviewEmulator>,
}

impl TmuxInput for CommandZellijInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()> {
        CommandTmuxInput::execute_command(command)
    }

    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        _include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        self.capture_session_output(target_session, scrollback_lines)
    }

    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
        let (pane_width, pane_height) = self
            .pane_sizes
            .lock()
            .ok()
            .and_then(|sizes| sizes.get(target_session).copied())
            .unwrap_or((120, 40));
        Ok(format!("0 0 0 {pane_width} {pane_height}"))
    }

    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        let mut sizes = self
            .pane_sizes
            .lock()
            .map_err(|_| std::io::Error::other("zellij pane size lock poisoned"))?;
        sizes.insert(target_session.to_string(), (target_width, target_height));
        Ok(())
    }

    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
        let config_path_text = zellij_config_path().to_string_lossy().to_string();
        let output = Command::new("zellij")
            .args([
                "--config",
                config_path_text.as_str(),
                "--session",
                target_session,
                "action",
                "write-chars",
                text,
            ])
            .output()?;
        if output.status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "zellij paste failed: {}",
            CommandTmuxInput::stderr_or_status(&output),
        )))
    }

    fn supports_background_send(&self) -> bool {
        false
    }
}

impl CommandZellijInput {
    fn capture_session_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
    ) -> std::io::Result<String> {
        #[cfg(not(test))]
        {
            if !Self::session_exists(target_session)? {
                return Err(std::io::Error::other(format!(
                    "zellij session not found: {target_session}"
                )));
            }
        }

        let log_path = zellij_capture_log_path(target_session);
        self.emulator
            .lock()
            .map_err(|_| std::io::Error::other("zellij emulator lock poisoned"))?
            .capture_from_log(target_session, &log_path, None, scrollback_lines)
    }

    #[cfg(not(test))]
    fn session_exists(target_session: &str) -> std::io::Result<bool> {
        let output = Command::new("zellij")
            .args(["list-sessions", "--short"])
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "zellij list-sessions failed: {}",
                CommandTmuxInput::stderr_or_status(&output),
            )));
        }
        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("zellij list-sessions utf8 decode failed: {error}"))
        })?;

        Ok(stdout.lines().any(|line| line.trim() == target_session))
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
            "command failed: {}; {}",
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
                for value in chars.by_ref() {
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

fn default_config_path() -> PathBuf {
    crate::config::config_path().unwrap_or_else(|| PathBuf::from(".config/grove/config.toml"))
}

fn current_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?;
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.canonicalize().ok().or(Some(path))
}

fn project_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn project_paths_equal(left: &Path, right: &Path) -> bool {
    let left_canonical = left.canonicalize().ok();
    let right_canonical = right.canonicalize().ok();
    match (left_canonical, right_canonical) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

fn ensure_current_repo_project(config: &mut GroveConfig, config_path: &Path) -> Option<String> {
    let repo_root = current_repo_root()?;

    let already_present = config
        .projects
        .iter()
        .any(|project| project_paths_equal(&project.path, &repo_root));
    if already_present {
        return None;
    }

    config.projects.push(ProjectConfig {
        name: project_display_name(&repo_root),
        path: repo_root,
    });
    crate::config::save_to_path(config_path, config).err()
}

fn load_runtime_config() -> (GroveConfig, PathBuf, Option<String>) {
    let (mut config, config_path, load_error) = match crate::config::load() {
        Ok(loaded) => (loaded.config, loaded.path, None),
        Err(error) => (GroveConfig::default(), default_config_path(), Some(error)),
    };
    let startup_error = ensure_current_repo_project(&mut config, &config_path);
    let error = match (load_error, startup_error) {
        (Some(load_error), Some(startup_error)) => Some(format!(
            "{load_error}; startup project add failed: {startup_error}"
        )),
        (Some(load_error), None) => Some(load_error),
        (None, Some(startup_error)) => Some(format!("startup project add failed: {startup_error}")),
        (None, None) => None,
    };

    (config, config_path, error)
}

#[derive(Debug, Clone)]
struct StaticMultiplexerAdapter {
    running_sessions: HashSet<String>,
}

impl crate::adapters::MultiplexerAdapter for StaticMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        self.running_sessions.clone()
    }
}

fn bootstrap_data_for_projects(
    projects: &[ProjectConfig],
    multiplexer: MultiplexerKind,
) -> BootstrapData {
    if projects.is_empty() {
        return BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Empty,
            orphaned_sessions: Vec::new(),
        };
    }

    let live_multiplexer = CommandMultiplexerAdapter { multiplexer };
    let static_multiplexer = StaticMultiplexerAdapter {
        running_sessions: live_multiplexer.running_sessions(),
    };
    let mut workspaces = Vec::new();
    let mut orphaned_sessions = Vec::new();
    let mut errors = Vec::new();
    for project in projects {
        let git = CommandGitAdapter::for_repo(project.path.clone());
        let system = CommandSystemAdapter::for_repo(project.path.clone());
        let bootstrap = bootstrap_data(&git, &static_multiplexer, &system);
        if let DiscoveryState::Error(message) = &bootstrap.discovery_state {
            errors.push(format!("{}: {message}", project.name));
        }

        workspaces.extend(bootstrap.workspaces);
        orphaned_sessions.extend(bootstrap.orphaned_sessions);
    }

    let discovery_state = if !workspaces.is_empty() {
        DiscoveryState::Ready
    } else if !errors.is_empty() {
        DiscoveryState::Error(errors.join("; "))
    } else {
        DiscoveryState::Empty
    };
    let repo_name = if projects.len() == 1 {
        projects[0].name.clone()
    } else {
        format!("{} projects", projects.len())
    };

    BootstrapData {
        repo_name,
        workspaces,
        discovery_state,
        orphaned_sessions,
    }
}

fn input_for_multiplexer(multiplexer: MultiplexerKind) -> Box<dyn TmuxInput> {
    match multiplexer {
        MultiplexerKind::Tmux => Box::new(CommandTmuxInput),
        MultiplexerKind::Zellij => Box::new(CommandZellijInput::default()),
    }
}

fn read_workspace_launch_prompt(workspace_path: &Path) -> Option<String> {
    let raw = fs::read_to_string(workspace_path.join(WORKSPACE_LAUNCH_PROMPT_FILENAME)).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn load_local_branches(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|error| format!("git branch failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git branch failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("git branch output decode failed: {error}"))?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn filter_branches(query: &str, all_branches: &[String]) -> Vec<String> {
    if query.is_empty() {
        return all_branches.to_vec();
    }

    let query_lower = query.to_lowercase();
    all_branches
        .iter()
        .filter(|branch| branch.to_lowercase().contains(&query_lower))
        .cloned()
        .collect()
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
        if let Some(toast) = self.notifications.visible().last() {
            if matches!(toast.config.style_variant, ToastStyle::Error) {
                return format!("Status: error: {}", toast.content.message);
            }
            return format!("Status: {}", toast.content.message);
        }

        match &self.discovery_state {
            DiscoveryState::Error(message) => format!("Status: discovery error ({message})"),
            DiscoveryState::Empty => "Status: no worktrees found".to_string(),
            DiscoveryState::Ready => {
                if let Some(dialog) = &self.create_dialog {
                    return format!(
                        "Status: new workspace, field={}, agent={}, base_branch=\"{}\", name=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.agent.label(),
                        dialog.base_branch.replace('\n', "\\n"),
                        dialog.workspace_name
                    );
                }
                if let Some(dialog) = &self.launch_dialog {
                    return format!(
                        "Status: start agent, field={}, unsafe={}, prompt=\"{}\", pre=\"{}\"",
                        dialog.focused_field.label(),
                        if dialog.skip_permissions { "on" } else { "off" },
                        dialog.prompt.replace('\n', "\\n"),
                        dialog.pre_launch_command.replace('\n', "\\n"),
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
                    UiMode::List => format!("Status: list, unsafe={}", self.unsafe_label()),
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
        if self.command_palette.is_visible() {
            return "Type to search, Up/Down choose, Enter run, Esc close";
        }
        if self.keybind_help_open {
            return "Esc/? close help";
        }
        if self.create_dialog.is_some() {
            return "Tab/S-Tab field, j/k or C-n/C-p move, h/l buttons, Enter select/create, Esc cancel";
        }
        if self.edit_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle agent, Enter save/select, Esc cancel";
        }
        if self.launch_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle unsafe, Enter select/start, Esc cancel";
        }
        if self.delete_dialog.is_some() {
            return "Tab/S-Tab field, j/k move, Space toggle branch delete, Enter select/delete, D confirm, Esc cancel";
        }
        if self.settings_dialog.is_some() {
            return "Tab/S-Tab field, j/k or h/l change, Enter save/select, Esc cancel";
        }
        if self.project_dialog.is_some() {
            return "Type filter, Up/Down or Tab/S-Tab navigate, Enter focus project, Ctrl+A add, Esc close";
        }
        if self.interactive.is_some() {
            return "Esc Esc / Ctrl+\\ exit, Alt+C copy, Alt+V paste";
        }
        if self.state.mode == UiMode::Preview {
            return "[/] tabs, j/k scroll, PgUp/PgDn, G bottom, h/l pane, Enter open, n new, e edit, p projects, s start, x stop, D delete, S settings, Ctrl+K palette, ? help, q quit";
        }

        "j/k move, h/l pane, Enter open, n new, e edit, p projects, s start, x stop, D delete, S settings, Ctrl+K palette, ? help, q quit"
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
        if self.preview_tab == PreviewTab::Agent {
            self.poll_preview();
        }
    }

    const fn workspace_status_label(status: WorkspaceStatus) -> &'static str {
        match status {
            WorkspaceStatus::Main => "main",
            WorkspaceStatus::Idle => "idle",
            WorkspaceStatus::Active => "active",
            WorkspaceStatus::Thinking => "thinking",
            WorkspaceStatus::Waiting => "waiting",
            WorkspaceStatus::Done => "done",
            WorkspaceStatus::Error => "error",
            WorkspaceStatus::Unknown => "unknown",
            WorkspaceStatus::Unsupported => "unsupported",
        }
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

        if self.can_start_selected_workspace() {
            actions.push(Self::palette_action(
                PALETTE_CMD_START_AGENT,
                "Start Agent",
                "Open start-agent dialog for selected workspace (s)",
                &["start", "agent", "workspace", "s"],
                "Workspace",
            ));
        }

        if self.can_stop_selected_workspace() {
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

        if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
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
                self.scroll_preview(-1);
                false
            }
            PALETTE_CMD_SCROLL_DOWN => {
                self.scroll_preview(1);
                false
            }
            PALETTE_CMD_PAGE_UP => {
                self.scroll_preview(-5);
                false
            }
            PALETTE_CMD_PAGE_DOWN => {
                self.scroll_preview(5);
                false
            }
            PALETTE_CMD_SCROLL_BOTTOM => {
                self.jump_preview_to_bottom();
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
                self.open_start_dialog();
                false
            }
            PALETTE_CMD_STOP_AGENT => {
                self.stop_selected_workspace_agent();
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

    fn selected_session_for_live_preview(&self) -> Option<(String, bool)> {
        let workspace = self.state.selected_workspace()?;
        if workspace.status.has_session() {
            return Some((Self::workspace_session_name(workspace), true));
        }

        None
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
        let live_preview = self.selected_session_for_live_preview();
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
        if self.selected_session_for_live_preview().is_none() {
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

        let live_preview = self.selected_session_for_live_preview();
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
        if self.selected_session_for_live_preview().is_none() {
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
            self.clear_agent_activity_tracking();
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

    fn is_ctrl_char_key(key_event: &KeyEvent, character: char) -> bool {
        matches!(
            key_event.code,
            KeyCode::Char(value)
                if value == character
                    && key_event.kind == KeyEventKind::Press
                    && key_event.modifiers == Modifiers::CTRL
        )
    }

    fn keybinding_task_running(&self) -> bool {
        self.refresh_in_flight
            || self.delete_in_flight
            || self.create_in_flight
            || self.start_in_flight
            || self.stop_in_flight
    }

    fn keybinding_input_nonempty(&self) -> bool {
        if let Some(dialog) = self.launch_dialog.as_ref() {
            return !dialog.prompt.is_empty() || !dialog.pre_launch_command.is_empty();
        }
        if let Some(dialog) = self.create_dialog.as_ref() {
            return !dialog.workspace_name.is_empty() || !dialog.base_branch.is_empty();
        }
        if let Some(project_dialog) = self.project_dialog.as_ref() {
            if !project_dialog.filter.is_empty() {
                return true;
            }
            if let Some(add_dialog) = project_dialog.add_dialog.as_ref() {
                return !add_dialog.name.is_empty() || !add_dialog.path.is_empty();
            }
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
                    self.clear_create_branch_picker();
                } else if self.edit_dialog.is_some() {
                    self.log_dialog_event("edit", "dialog_cancelled");
                    self.edit_dialog = None;
                } else if self.launch_dialog.is_some() {
                    self.log_dialog_event("launch", "dialog_cancelled");
                    self.launch_dialog = None;
                } else if self.delete_dialog.is_some() {
                    self.log_dialog_event("delete", "dialog_cancelled");
                    self.delete_dialog = None;
                } else if self.settings_dialog.is_some() {
                    self.log_dialog_event("settings", "dialog_cancelled");
                    self.settings_dialog = None;
                } else if self.project_dialog.is_some() {
                    self.project_dialog = None;
                } else if self.keybind_help_open {
                    self.keybind_help_open = false;
                }
                false
            }
            KeybindingAction::ClearInput => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => dialog.prompt.clear(),
                        LaunchDialogField::PreLaunchCommand => dialog.pre_launch_command.clear(),
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                    return false;
                }
                if let Some(dialog) = self.create_dialog.as_mut() {
                    let mut refresh_base_branch = false;
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => dialog.workspace_name.clear(),
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.clear();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                    if refresh_base_branch {
                        self.refresh_create_branch_filtered();
                    }
                }
                false
            }
            KeybindingAction::CancelTask => {
                self.show_toast("cannot cancel running lifecycle task", true);
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

        workspace.status.has_session()
    }

    fn enter_interactive(&mut self, now: Instant) -> bool {
        if !self.can_enter_interactive() {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };

        let session_name = Self::workspace_session_name(workspace);
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
        if !workspace.supported_agent {
            return false;
        }

        matches!(
            workspace.status,
            WorkspaceStatus::Main
                | WorkspaceStatus::Idle
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
                | WorkspaceStatus::Unknown
        )
    }

    fn open_keybind_help(&mut self) {
        if self.modal_open() {
            return;
        }
        self.keybind_help_open = true;
    }

    fn handle_keybind_help_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::Char('?') => {
                self.keybind_help_open = false;
            }
            _ => {}
        }
    }

    fn filtered_project_indices(&self, query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..self.projects.len()).collect();
        }

        let query_lower = query.to_ascii_lowercase();
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| {
                project.name.to_ascii_lowercase().contains(&query_lower)
                    || project
                        .path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query_lower)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn refresh_project_dialog_filtered(&mut self) {
        let query = match self.project_dialog.as_ref() {
            Some(dialog) => dialog.filter.clone(),
            None => return,
        };
        let filtered = self.filtered_project_indices(&query);
        let Some(dialog) = self.project_dialog.as_mut() else {
            return;
        };

        dialog.filtered_project_indices = filtered;
        if dialog.filtered_project_indices.is_empty() {
            dialog.selected_filtered_index = 0;
            return;
        }
        if dialog.selected_filtered_index >= dialog.filtered_project_indices.len() {
            dialog.selected_filtered_index =
                dialog.filtered_project_indices.len().saturating_sub(1);
        }
    }

    fn selected_project_dialog_project_index(&self) -> Option<usize> {
        let dialog = self.project_dialog.as_ref()?;
        if dialog.filtered_project_indices.is_empty() {
            return None;
        }
        dialog
            .filtered_project_indices
            .get(dialog.selected_filtered_index)
            .copied()
    }

    fn focus_project_by_index(&mut self, project_index: usize) {
        let Some(project) = self.projects.get(project_index) else {
            return;
        };

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace.is_main
                        && workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
            return;
        }

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
        }
    }

    fn open_project_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let selected_project_index = self.selected_project_index();
        let filtered_project_indices: Vec<usize> = (0..self.projects.len()).collect();
        let selected_filtered_index = filtered_project_indices
            .iter()
            .position(|index| *index == selected_project_index)
            .unwrap_or(0);
        self.project_dialog = Some(ProjectDialogState {
            filter: String::new(),
            filtered_project_indices,
            selected_filtered_index,
            add_dialog: None,
        });
    }

    fn open_project_add_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        project_dialog.add_dialog = Some(ProjectAddDialogState {
            name: String::new(),
            path: String::new(),
            focused_field: ProjectAddDialogField::Name,
        });
    }

    fn normalized_project_path(raw: &str) -> PathBuf {
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(raw)
    }

    fn save_projects_config(&self) -> Result<(), String> {
        let config = GroveConfig {
            multiplexer: self.multiplexer,
            projects: self.projects.clone(),
        };
        crate::config::save_to_path(&self.config_path, &config)
    }

    fn add_project_from_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_ref() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };

        let path_input = add_dialog.path.trim();
        if path_input.is_empty() {
            self.show_toast("project path is required", true);
            return;
        }
        let normalized = Self::normalized_project_path(path_input);
        let canonical = match normalized.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                self.show_toast(format!("invalid project path: {error}"), true);
                return;
            }
        };

        let repo_root_output = Command::new("git")
            .current_dir(&canonical)
            .args(["rev-parse", "--show-toplevel"])
            .output();
        let repo_root = match repo_root_output {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8(output.stdout).unwrap_or_default();
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    canonical.clone()
                } else {
                    PathBuf::from(trimmed)
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                self.show_toast(format!("not a git repository: {stderr}"), true);
                return;
            }
            Err(error) => {
                self.show_toast(format!("git check failed: {error}"), true);
                return;
            }
        };
        let repo_root = repo_root.canonicalize().unwrap_or(repo_root);

        if self
            .projects
            .iter()
            .any(|project| project_paths_equal(&project.path, &repo_root))
        {
            self.show_toast("project already exists", true);
            return;
        }

        let project_name = if add_dialog.name.trim().is_empty() {
            project_display_name(&repo_root)
        } else {
            add_dialog.name.trim().to_string()
        };
        self.projects.push(ProjectConfig {
            name: project_name.clone(),
            path: repo_root.clone(),
        });
        if let Err(error) = self.save_projects_config() {
            self.show_toast(format!("project save failed: {error}"), true);
            return;
        }

        if let Some(dialog) = self.project_dialog.as_mut() {
            dialog.add_dialog = None;
        }
        self.refresh_project_dialog_filtered();
        self.refresh_workspaces(None);
        self.show_toast(format!("project '{}' added", project_name), false);
    }

    fn handle_project_add_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        match key_event.code {
            KeyCode::Escape => {
                project_dialog.add_dialog = None;
            }
            KeyCode::Tab => {
                add_dialog.focused_field = add_dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                add_dialog.focused_field = add_dialog.focused_field.previous();
            }
            KeyCode::Enter => match add_dialog.focused_field {
                ProjectAddDialogField::AddButton => self.add_project_from_dialog(),
                ProjectAddDialogField::CancelButton => project_dialog.add_dialog = None,
                ProjectAddDialogField::Name | ProjectAddDialogField::Path => {
                    add_dialog.focused_field = add_dialog.focused_field.next();
                }
            },
            KeyCode::Backspace => match add_dialog.focused_field {
                ProjectAddDialogField::Name => {
                    add_dialog.name.pop();
                }
                ProjectAddDialogField::Path => {
                    add_dialog.path.pop();
                }
                ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                match add_dialog.focused_field {
                    ProjectAddDialogField::Name => add_dialog.name.push(character),
                    ProjectAddDialogField::Path => add_dialog.path.push(character),
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                }
            }
            _ => {}
        }
    }

    fn handle_project_dialog_key(&mut self, key_event: KeyEvent) {
        if self
            .project_dialog
            .as_ref()
            .and_then(|dialog| dialog.add_dialog.as_ref())
            .is_some()
        {
            self.handle_project_add_dialog_key(key_event);
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && !dialog.filter.is_empty()
                {
                    dialog.filter.clear();
                    self.refresh_project_dialog_filtered();
                    return;
                }
                self.project_dialog = None;
            }
            KeyCode::Enter => {
                if let Some(project_index) = self.selected_project_dialog_project_index() {
                    self.focus_project_by_index(project_index);
                    self.project_dialog = None;
                }
            }
            KeyCode::Up => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index > 0
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index =
                            dialog.selected_filtered_index.saturating_add(1) % len;
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index = if dialog.selected_filtered_index == 0 {
                            len.saturating_sub(1)
                        } else {
                            dialog.selected_filtered_index.saturating_sub(1)
                        };
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.pop();
                }
                self.refresh_project_dialog_filtered();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'a' || character == 'A') =>
            {
                self.open_project_add_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'n' || character == 'N') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'p' || character == 'P') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.push(character);
                }
                self.refresh_project_dialog_filtered();
            }
            _ => {}
        }
    }

    fn allows_text_input_modifiers(modifiers: Modifiers) -> bool {
        modifiers.is_empty() || modifiers == Modifiers::SHIFT
    }

    fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.settings_dialog = Some(SettingsDialogState {
            multiplexer: self.multiplexer,
            focused_field: SettingsDialogField::Multiplexer,
        });
    }

    fn has_running_workspace_sessions(&self) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|workspace| workspace.status.has_session())
    }

    fn apply_settings_dialog_save(&mut self) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };

        if dialog.multiplexer != self.multiplexer && self.has_running_workspace_sessions() {
            self.show_toast("stop running workspaces before switching multiplexer", true);
            return;
        }

        let selected = dialog.multiplexer;
        self.multiplexer = selected;
        self.tmux_input = input_for_multiplexer(selected);
        let config = GroveConfig {
            multiplexer: selected,
            projects: self.projects.clone(),
        };
        if let Err(error) = crate::config::save_to_path(&self.config_path, &config) {
            self.show_toast(format!("settings save failed: {error}"), true);
            return;
        }

        self.settings_dialog = None;
        self.interactive = None;
        self.refresh_workspaces(None);
        self.poll_preview();
        self.show_toast(format!("multiplexer set to {}", selected.label()), false);
    }

    fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = MultiplexerKind::Tmux;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = MultiplexerKind::Zellij;
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = match dialog.multiplexer {
                        MultiplexerKind::Tmux => MultiplexerKind::Zellij,
                        MultiplexerKind::Zellij => MultiplexerKind::Tmux,
                    };
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::Multiplexer => {
                    dialog.multiplexer = match dialog.multiplexer {
                        MultiplexerKind::Tmux => MultiplexerKind::Zellij,
                        MultiplexerKind::Zellij => MultiplexerKind::Tmux,
                    };
                }
                SettingsDialogField::SaveButton => post_action = PostAction::Save,
                SettingsDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.settings_dialog = None;
            }
        }
    }

    fn open_start_dialog(&mut self) {
        if self.start_in_flight {
            self.show_toast("agent start already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        if !workspace.supported_agent {
            self.show_toast("unsupported workspace agent marker", true);
            return;
        }
        if workspace.status.is_running() {
            self.show_toast("agent already running", true);
            return;
        }
        if !self.can_start_selected_workspace() {
            self.show_toast("workspace cannot be started", true);
            return;
        }

        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let skip_permissions = self.launch_skip_permissions;
        self.launch_dialog = Some(LaunchDialogState {
            prompt: prompt.clone(),
            pre_launch_command: String::new(),
            skip_permissions,
            focused_field: LaunchDialogField::Prompt,
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
                ("pre_launch_len".to_string(), Value::from(0_u64)),
            ],
        );
        self.last_tmux_error = None;
    }

    fn open_delete_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.delete_in_flight {
            self.show_toast("workspace delete already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        if workspace.is_main {
            self.show_toast("cannot delete base workspace", true);
            return;
        }

        let is_missing = !workspace.path.exists();
        self.delete_dialog = Some(DeleteDialogState {
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            workspace_name: workspace.name.clone(),
            branch: workspace.branch.clone(),
            path: workspace.path.clone(),
            is_missing,
            delete_local_branch: is_missing,
            focused_field: DeleteDialogField::DeleteLocalBranch,
        });
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("is_missing".to_string(), Value::from(is_missing)),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn handle_delete_dialog_key(&mut self, key_event: KeyEvent) {
        if self.delete_in_flight {
            return;
        }
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.delete_dialog = None;
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.delete_dialog = None;
                return;
            }
            KeyCode::Char('D') if no_modifiers => {
                self.confirm_delete_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_delete = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.delete_dialog.as_mut() else {
            return;
        };

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                DeleteDialogField::DeleteLocalBranch => {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                }
                DeleteDialogField::DeleteButton => {
                    confirm_delete = true;
                }
                DeleteDialogField::CancelButton => {
                    cancel_dialog = true;
                }
            },
            KeyCode::Tab => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Up | KeyCode::Char('k') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Down | KeyCode::Char('j') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(' ') if no_modifiers => {
                if dialog.focused_field == DeleteDialogField::DeleteLocalBranch {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                }
            }
            KeyCode::Char(character) if no_modifiers => {
                if (dialog.focused_field == DeleteDialogField::DeleteButton
                    || dialog.focused_field == DeleteDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == DeleteDialogField::DeleteButton {
                            DeleteDialogField::CancelButton
                        } else {
                            DeleteDialogField::DeleteButton
                        };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("delete", "dialog_cancelled");
            self.delete_dialog = None;
            return;
        }
        if confirm_delete {
            self.confirm_delete_dialog();
        }
    }

    fn confirm_delete_dialog(&mut self) {
        if self.delete_in_flight {
            return;
        }

        let Some(dialog) = self.delete_dialog.take() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("branch".to_string(), Value::from(dialog.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(dialog.path.display().to_string()),
                ),
                (
                    "delete_local_branch".to_string(),
                    Value::from(dialog.delete_local_branch),
                ),
                ("is_missing".to_string(), Value::from(dialog.is_missing)),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.path.clone();
        if !self.tmux_input.supports_background_send() {
            let (result, warnings) = Self::run_delete_workspace(dialog, self.multiplexer);
            self.apply_delete_workspace_completion(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            });
            return;
        }

        let multiplexer = self.multiplexer;
        self.delete_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = Self::run_delete_workspace(dialog, multiplexer);
            Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            })
        }));
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

    fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.create_in_flight {
            return;
        }

        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.create_dialog = None;
                self.clear_create_branch_picker();
            }
            KeyCode::Enter => {
                if self.select_create_base_branch_from_dropdown() {
                    if let Some(dialog) = self.create_dialog.as_mut() {
                        dialog.focused_field = dialog.focused_field.next();
                    }
                    self.refresh_create_branch_filtered();
                    return;
                }

                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                let action = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                        CreateDialogField::CancelButton => EnterAction::CancelDialog,
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::Project
                        | CreateDialogField::BaseBranch
                        | CreateDialogField::Agent => EnterAction::AdvanceField,
                    });

                match action {
                    Some(EnterAction::ConfirmCreate) => self.confirm_create_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.create_dialog = None;
                        self.clear_create_branch_picker();
                    }
                    Some(EnterAction::AdvanceField) => {
                        if let Some(dialog) = self.create_dialog.as_mut() {
                            dialog.focused_field = dialog.focused_field.next();
                        }
                    }
                    None => {}
                }
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
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Up => {
                if self.create_base_branch_dropdown_visible() && self.create_branch_index > 0 {
                    self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(-1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Down => {
                if self.create_base_branch_dropdown_visible()
                    && self.create_branch_index.saturating_add(1)
                        < self.create_branch_filtered.len()
                {
                    self.create_branch_index = self.create_branch_index.saturating_add(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Char(_) if ctrl_n || ctrl_p => {
                let focused_field = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| dialog.focused_field);
                if focused_field == Some(CreateDialogField::BaseBranch)
                    && !self.create_branch_filtered.is_empty()
                {
                    if ctrl_n
                        && self.create_branch_index.saturating_add(1)
                            < self.create_branch_filtered.len()
                    {
                        self.create_branch_index = self.create_branch_index.saturating_add(1);
                    }
                    if ctrl_p && self.create_branch_index > 0 {
                        self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    }
                } else if focused_field == Some(CreateDialogField::Project) {
                    if ctrl_n {
                        self.shift_create_dialog_project(1);
                    }
                    if ctrl_p {
                        self.shift_create_dialog_project(-1);
                    }
                } else if focused_field == Some(CreateDialogField::Agent)
                    && let Some(dialog) = self.create_dialog.as_mut()
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Backspace => {
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.workspace_name.pop();
                        }
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.pop();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    if character == 'j' {
                        self.shift_create_dialog_project(1);
                        return;
                    }
                    if character == 'k' {
                        self.shift_create_dialog_project(-1);
                        return;
                    }
                }
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    if dialog.focused_field == CreateDialogField::Agent
                        && (character == 'j' || character == 'k' || character == ' ')
                    {
                        Self::toggle_create_dialog_agent(dialog);
                        return;
                    }
                    if (dialog.focused_field == CreateDialogField::CreateButton
                        || dialog.focused_field == CreateDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == CreateDialogField::CreateButton {
                                CreateDialogField::CancelButton
                            } else {
                                CreateDialogField::CreateButton
                            };
                        return;
                    }
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            if character.is_ascii_alphanumeric()
                                || character == '-'
                                || character == '_'
                            {
                                dialog.workspace_name.push(character);
                            }
                        }
                        CreateDialogField::Project => {}
                        CreateDialogField::BaseBranch => {
                            if character == 'j'
                                && self.create_branch_index.saturating_add(1)
                                    < self.create_branch_filtered.len()
                            {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_add(1);
                                return;
                            }
                            if character == 'k' && self.create_branch_index > 0 {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_sub(1);
                                return;
                            }
                            if !character.is_control() {
                                dialog.base_branch.push(character);
                                refresh_base_branch = true;
                            }
                        }
                        CreateDialogField::Agent => {}
                        CreateDialogField::CreateButton | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            _ => {}
        }
    }

    fn handle_edit_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.edit_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                EditDialogField::Agent => Self::toggle_edit_dialog_agent(dialog),
                EditDialogField::SaveButton => post_action = PostAction::Save,
                EditDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_edit_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("edit", "dialog_cancelled");
                self.edit_dialog = None;
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

    fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.start_in_flight {
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("launch", "dialog_cancelled");
                self.launch_dialog = None;
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmStart,
                    CancelDialog,
                }

                let action = self
                    .launch_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        LaunchDialogField::StartButton => EnterAction::ConfirmStart,
                        LaunchDialogField::CancelButton => EnterAction::CancelDialog,
                        LaunchDialogField::Prompt
                        | LaunchDialogField::PreLaunchCommand
                        | LaunchDialogField::Unsafe => EnterAction::ConfirmStart,
                    });

                match action {
                    Some(EnterAction::ConfirmStart) => self.confirm_start_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("launch", "dialog_cancelled");
                        self.launch_dialog = None;
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => {
                            dialog.prompt.pop();
                        }
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.pop();
                        }
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    if (dialog.focused_field == LaunchDialogField::StartButton
                        || dialog.focused_field == LaunchDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == LaunchDialogField::StartButton {
                                LaunchDialogField::CancelButton
                            } else {
                                LaunchDialogField::StartButton
                            };
                        return;
                    }
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => dialog.prompt.push(character),
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.push(character)
                        }
                        LaunchDialogField::Unsafe => {
                            if character == ' ' || character == 'j' || character == 'k' {
                                dialog.skip_permissions = !dialog.skip_permissions;
                            }
                        }
                        LaunchDialogField::StartButton | LaunchDialogField::CancelButton => {}
                    }
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
        let Some(command) =
            multiplexer_send_input_command(self.multiplexer, target_session, action)
        else {
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

    fn enter_preview_or_interactive(&mut self) {
        if !self.enter_interactive(Instant::now()) {
            reduce(&mut self.state, Action::EnterPreviewMode);
            self.poll_preview();
        }
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => reduce(&mut self.state, Action::ToggleFocus),
            KeyCode::Enter => self.enter_preview_or_interactive(),
            KeyCode::Escape => reduce(&mut self.state, Action::EnterListMode),
            KeyCode::Char('!') => {
                self.launch_skip_permissions = !self.launch_skip_permissions;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => self.open_create_dialog(),
            KeyCode::Char('e') | KeyCode::Char('E') => self.open_edit_dialog(),
            KeyCode::Char('p') | KeyCode::Char('P') => self.open_project_dialog(),
            KeyCode::Char('?') => self.open_keybind_help(),
            KeyCode::Char('D') => self.open_delete_dialog(),
            KeyCode::Char('S') => self.open_settings_dialog(),
            KeyCode::Char('s') => self.open_start_dialog(),
            KeyCode::Char('x') => self.stop_selected_workspace_agent(),
            KeyCode::Char('h') => reduce(&mut self.state, Action::EnterListMode),
            KeyCode::Char('l') => {
                let mode_before = self.state.mode;
                let focus_before = self.state.focus;
                reduce(&mut self.state, Action::EnterPreviewMode);
                if self.state.mode != mode_before || self.state.focus != focus_before {
                    self.poll_preview();
                }
            }
            KeyCode::Char('[') => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(-1);
                }
            }
            KeyCode::Char(']') => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.cycle_preview_tab(1);
                }
            }
            KeyCode::PageUp => {
                if self.state.mode == UiMode::Preview
                    && self.state.focus == PaneFocus::Preview
                    && self.preview_tab == PreviewTab::Agent
                {
                    self.scroll_preview(-5);
                }
            }
            KeyCode::PageDown => {
                if self.state.mode == UiMode::Preview
                    && self.state.focus == PaneFocus::Preview
                    && self.preview_tab == PreviewTab::Agent
                {
                    self.scroll_preview(5);
                }
            }
            KeyCode::Char('G') => {
                if self.state.mode == UiMode::Preview
                    && self.state.focus == PaneFocus::Preview
                    && self.preview_tab == PreviewTab::Agent
                {
                    self.jump_preview_to_bottom();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(1);
                    }
                } else {
                    self.move_selection(Action::MoveSelectionDown);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(-1);
                    }
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

    fn effective_viewport_size(&self) -> (u16, u16) {
        let from_hit_grid = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .map(|grid| (grid.width(), grid.height()));
        let (width, height) = from_hit_grid.unwrap_or((self.viewport_width, self.viewport_height));
        (width.max(1), height.max(1))
    }

    fn view_layout(&self) -> ViewLayout {
        let (width, height) = self.effective_viewport_size();
        Self::view_layout_for_size(width, height, self.sidebar_width_pct)
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
                HIT_ID_CREATE_DIALOG
                | HIT_ID_LAUNCH_DIALOG
                | HIT_ID_DELETE_DIALOG
                | HIT_ID_KEYBIND_HELP_DIALOG => HitRegion::Outside,
                _ => HitRegion::Outside,
            };
            let row_data = if id.id() == HIT_ID_WORKSPACE_ROW {
                Some(data)
            } else {
                None
            };
            return (mapped, row_data);
        }

        let (viewport_width, viewport_height) = self.effective_viewport_size();
        let layout = self.view_layout();

        if x >= viewport_width || y >= viewport_height {
            return (HitRegion::Outside, None);
        }
        if y < layout.header.bottom() {
            return (HitRegion::Header, None);
        }
        if y >= layout.status.y {
            return (HitRegion::StatusLine, None);
        }

        let divider_area = Self::divider_hit_area(layout.divider, viewport_width);
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
        let interactive = self.interactive.as_ref()?;
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
        if self.preview_tab != PreviewTab::Agent {
            return None;
        }

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
        let line = self.preview_plain_line(line_idx)?;
        let line_width = line_visual_width(&line);
        if x < viewport.output_x {
            return Some(TextSelectionPoint {
                line: line_idx,
                col: 0,
            });
        }

        let relative_x = usize::from(x - viewport.output_x);
        let col = if line_width == 0 {
            0
        } else {
            relative_x.min(line_width.saturating_sub(1))
        };

        Some(TextSelectionPoint {
            line: line_idx,
            col,
        })
    }

    fn preview_plain_line(&self, line_idx: usize) -> Option<String> {
        if let Some(line) = self.preview.render_lines.get(line_idx) {
            return Some(ansi_line_to_plain_text(line));
        }

        self.preview.lines.get(line_idx).cloned()
    }

    fn preview_plain_lines_range(&self, start: usize, end: usize) -> Vec<String> {
        if start >= end {
            return Vec::new();
        }

        let mut lines = Vec::with_capacity(end.saturating_sub(start));
        for line_idx in start..end {
            if let Some(line) = self.preview_plain_line(line_idx) {
                lines.push(line);
                continue;
            }
            break;
        }

        lines
    }

    fn add_selection_point_snapshot_fields(
        &self,
        mut event: LogEvent,
        key_prefix: &str,
        point: TextSelectionPoint,
    ) -> LogEvent {
        let raw_line = self.preview.lines.get(point.line).cloned();
        let clean_line = self.preview_plain_line(point.line);
        let render_line = self.preview.render_lines.get(point.line).cloned();

        if let Some(line) = raw_line {
            event = event.with_data(
                format!("{key_prefix}line_raw_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        if let Some(line) = clean_line {
            event = event
                .with_data(
                    format!("{key_prefix}line_clean_preview"),
                    Value::from(truncate_for_log(&line, 120)),
                )
                .with_data(
                    format!("{key_prefix}line_visual_width"),
                    Value::from(u64::try_from(line_visual_width(&line)).unwrap_or(u64::MAX)),
                )
                .with_data(
                    format!("{key_prefix}line_context"),
                    Value::from(truncate_for_log(
                        &visual_substring(
                            &line,
                            point.col.saturating_sub(16),
                            Some(point.col.saturating_add(16)),
                        ),
                        120,
                    )),
                );

            if let Some((grapheme, start_col, end_col)) = visual_grapheme_at(&line, point.col) {
                event = event
                    .with_data(
                        format!("{key_prefix}grapheme"),
                        Value::from(truncate_for_log(&grapheme, 16)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_start_col"),
                        Value::from(u64::try_from(start_col).unwrap_or(u64::MAX)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_end_col"),
                        Value::from(u64::try_from(end_col).unwrap_or(u64::MAX)),
                    );
            }
        }

        if let Some(line) = render_line {
            event = event.with_data(
                format!("{key_prefix}line_render_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        event
    }

    fn log_preview_drag_started(&self, x: u16, y: u16, point: Option<TextSelectionPoint>) {
        let mut event = LogEvent::new("selection", "preview_drag_started")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("mode", Value::from(Self::mode_name(self.state.mode)))
            .with_data("focus", Value::from(Self::focus_name(self.state.focus)))
            .with_data(
                "preview_offset",
                Value::from(u64::try_from(self.preview.offset).unwrap_or(u64::MAX)),
            );

        if let Some(viewport) = self.preview_content_viewport() {
            event = event
                .with_data("output_x", Value::from(viewport.output_x))
                .with_data("output_y", Value::from(viewport.output_y))
                .with_data(
                    "visible_start",
                    Value::from(u64::try_from(viewport.visible_start).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "visible_end",
                    Value::from(u64::try_from(viewport.visible_end).unwrap_or(u64::MAX)),
                );
        }

        if let Some(point) = point {
            event = event
                .with_data(
                    "line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "", point);
            if let Some(line) = self.preview_plain_line(point.line) {
                event = event.with_data("line_preview", Value::from(truncate_for_log(&line, 120)));
            }
            if let Some(render_line) = self.preview.render_lines.get(point.line) {
                event = event.with_data(
                    "render_line_preview",
                    Value::from(truncate_for_log(render_line, 120)),
                );
            }
        }

        self.event_log.log(event);
    }

    fn log_preview_drag_finished(&self, x: u16, y: u16, point: Option<TextSelectionPoint>) {
        let mut event = LogEvent::new("selection", "preview_drag_finished")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data(
                "has_selection",
                Value::from(self.preview_selection.has_selection()),
            )
            .with_data("interactive", Value::from(self.interactive.is_some()));

        if let Some(point) = point {
            event = event
                .with_data(
                    "release_line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "release_col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "release_", point);
        }

        if let Some(anchor) = self.preview_selection.anchor {
            event = event
                .with_data(
                    "anchor_line",
                    Value::from(u64::try_from(anchor.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "anchor_col",
                    Value::from(u64::try_from(anchor.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "anchor_", anchor);
        }

        if let Some(start) = self.preview_selection.start {
            event = event
                .with_data(
                    "start_line",
                    Value::from(u64::try_from(start.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "start_col",
                    Value::from(u64::try_from(start.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "start_", start);
        }
        if let Some(end) = self.preview_selection.end {
            event = event
                .with_data(
                    "end_line",
                    Value::from(u64::try_from(end.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "end_col",
                    Value::from(u64::try_from(end.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "end_", end);
        }

        if let Some(lines) = self.selected_preview_text_lines() {
            let text = lines.join("\n");
            event = event
                .with_data(
                    "selected_line_count",
                    Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "selected_char_count",
                    Value::from(u64::try_from(text.chars().count()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "selected_preview",
                    Value::from(truncate_for_log(&text, 240)),
                );
        }

        self.event_log.log(event);
    }

    fn prepare_preview_selection_drag(&mut self, x: u16, y: u16) {
        let point = self.preview_text_point_at(x, y);
        self.log_preview_drag_started(x, y, point);
        if let Some(point) = point {
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
        let release_point = self.preview_text_point_at(x, y);
        if !self.preview_selection.has_selection()
            && let Some(point) = release_point
        {
            self.preview_selection.handle_drag(point);
        }
        self.log_preview_drag_finished(x, y, release_point);
        self.preview_selection.finish_drag();
    }

    fn apply_preview_selection_highlight_cells(
        &self,
        frame: &mut Frame,
        inner: Rect,
        visible_plain_lines: &[String],
        visible_start: usize,
    ) {
        if !self.preview_selection.has_selection() {
            return;
        }

        let selection_bg = ui_theme().surface1;
        let output_y = inner.y.saturating_add(PREVIEW_METADATA_ROWS);
        for (offset, line) in visible_plain_lines.iter().enumerate() {
            let line_idx = visible_start.saturating_add(offset);
            let Some((start_col, end_col)) = self.preview_selection.line_selection_cols(line_idx)
            else {
                continue;
            };

            let line_width = line_visual_width(line);
            if line_width == 0 {
                continue;
            }

            let start = start_col.min(line_width.saturating_sub(1));
            let end = end_col
                .unwrap_or_else(|| line_width.saturating_sub(1))
                .min(line_width.saturating_sub(1));
            if end < start {
                continue;
            }

            let y = output_y.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            if y >= inner.bottom() {
                break;
            }

            let x_start = inner
                .x
                .saturating_add(u16::try_from(start).unwrap_or(u16::MAX));
            let x_end = inner
                .x
                .saturating_add(u16::try_from(end).unwrap_or(u16::MAX))
                .min(inner.right().saturating_sub(1));
            if x_start > x_end {
                continue;
            }

            for x in x_start..=x_end {
                if let Some(cell) = frame.buffer.get_mut(x, y) {
                    cell.bg = selection_bg;
                }
            }
        }
    }

    fn selected_preview_text_lines(&self) -> Option<Vec<String>> {
        let (start, end) = self.preview_selection.bounds()?;
        let source_len = self
            .preview
            .lines
            .len()
            .max(self.preview.render_lines.len());
        if source_len == 0 {
            return None;
        }

        let start_line = start.line.min(source_len.saturating_sub(1));
        let end_line = end.line.min(source_len.saturating_sub(1));
        if end_line < start_line {
            return None;
        }

        let mut lines = self.preview_plain_lines_range(start_line, end_line.saturating_add(1));
        if lines.is_empty() {
            return None;
        }

        if lines.len() == 1 {
            lines[0] = visual_substring(&lines[0], start.col, Some(end.col));
            return Some(lines);
        }

        lines[0] = visual_substring(&lines[0], start.col, None);
        let last_idx = lines.len().saturating_sub(1);
        lines[last_idx] = visual_substring(&lines[last_idx], 0, Some(end.col));

        Some(lines)
    }

    fn visible_preview_output_lines(&self) -> Vec<String> {
        let Some((_, output_height)) = self.preview_output_dimensions() else {
            return Vec::new();
        };
        let (visible_start, visible_end) =
            self.preview_visible_range_for_height(usize::from(output_height));
        self.preview_plain_lines_range(visible_start, visible_end)
    }

    fn copy_interactive_selection_or_visible(&mut self) {
        let selected_lines = self.selected_preview_text_lines();
        let copied_from_selection = selected_lines.is_some();
        let mut lines = selected_lines.unwrap_or_else(|| self.visible_preview_output_lines());
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            self.show_toast("No output to copy", true);
            return;
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            self.last_tmux_error = Some("no output to copy".to_string());
            self.show_toast("No output to copy", true);
            return;
        }
        let text = lines.join("\n");
        self.event_log.log(
            LogEvent::new("selection", "interactive_copy_payload")
                .with_data("from_selection", Value::from(copied_from_selection))
                .with_data(
                    "line_count",
                    Value::from(u64::try_from(lines.len()).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "char_count",
                    Value::from(u64::try_from(text.chars().count()).unwrap_or(u64::MAX)),
                )
                .with_data("preview", Value::from(truncate_for_log(&text, 240))),
        );
        self.copied_text = Some(text.clone());
        match self.clipboard.write_text(&text) {
            Ok(()) => {
                self.last_tmux_error = None;
                self.show_toast(format!("Copied {} line(s)", lines.len()), false);
            }
            Err(error) => {
                self.last_tmux_error = Some(format!("clipboard write failed: {error}"));
                self.show_toast(format!("Copy failed: {error}"), true);
            }
        }
        self.clear_preview_selection();
    }

    fn sidebar_workspace_index_at_y(&self, y: u16) -> Option<usize> {
        if self.projects.is_empty() {
            return None;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            return None;
        }

        let layout = self.view_layout();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return None;
        }

        let target_row = usize::from(y.saturating_sub(sidebar_inner.y));
        let mut visual_row = 0usize;
        for (project_index, project) in self.projects.iter().enumerate() {
            if project_index > 0 {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
            }

            if visual_row == target_row {
                return None;
            }
            visual_row = visual_row.saturating_add(1);

            let workspace_indices: Vec<usize> = self
                .state
                .workspaces
                .iter()
                .enumerate()
                .filter(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| project_paths_equal(path, &project.path))
                })
                .map(|(index, _)| index)
                .collect();
            if workspace_indices.is_empty() {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
                continue;
            }

            for workspace_index in workspace_indices {
                if visual_row == target_row {
                    return Some(workspace_index);
                }
                visual_row = visual_row.saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT));
            }
        }

        None
    }

    fn select_workspace_by_mouse(&mut self, y: u16) {
        let Some(row) = self.sidebar_workspace_index_at_y(y) else {
            return;
        };

        if row != self.state.selected_index {
            self.state.selected_index = row;
            self.preview.jump_to_bottom();
            self.clear_agent_activity_tracking();
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
        self.clear_agent_activity_tracking();
        self.clear_preview_selection();
        self.poll_preview();
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(state) = self.interactive.as_mut() {
            state.note_mouse_event(Instant::now());
        }

        let (region, row_data) = self.hit_region_for_point(mouse_event.x, mouse_event.y);
        let mut event = LogEvent::new("mouse", "event")
            .with_data("x", Value::from(mouse_event.x))
            .with_data("y", Value::from(mouse_event.y))
            .with_data("kind", Value::from(format!("{:?}", mouse_event.kind)))
            .with_data("region", Value::from(Self::hit_region_name(region)))
            .with_data("modal_open", Value::from(self.modal_open()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("divider_drag_active", Value::from(self.divider_drag_active))
            .with_data("focus", Value::from(Self::focus_name(self.state.focus)))
            .with_data("mode", Value::from(Self::mode_name(self.state.mode)));
        if let Some(row_data) = row_data {
            event = event.with_data("row_data", Value::from(row_data));
        }
        if matches!(region, HitRegion::Preview)
            && let Some(point) = self.preview_text_point_at(mouse_event.x, mouse_event.y)
        {
            event = event
                .with_data(
                    "mapped_line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "mapped_col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "mapped_", point);
        }
        self.event_log.log(event);

        if self.modal_open() {
            return;
        }

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
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(-1);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_tab == PreviewTab::Agent {
                        self.scroll_preview(1);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key_event: KeyEvent) -> (bool, Cmd<Msg>) {
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return (false, Cmd::None);
        }

        if Self::is_ctrl_char_key(&key_event, 'k') {
            self.open_command_palette();
            return (false, Cmd::None);
        }

        if self.command_palette.is_visible() {
            let event = Event::Key(key_event);
            if let Some(action) = self.command_palette.handle_event(&event) {
                return match action {
                    PaletteAction::Dismiss => (false, Cmd::None),
                    PaletteAction::Execute(id) => {
                        (self.execute_command_palette_action(id.as_str()), Cmd::None)
                    }
                };
            }
            return (false, Cmd::None);
        }

        if self.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        if self.create_dialog.is_some()
            && key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('p'))
        {
            self.handle_create_dialog_key(key_event);
            return (false, Cmd::None);
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

        if self.edit_dialog.is_some() {
            self.handle_edit_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.launch_dialog.is_some() {
            self.handle_launch_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.delete_dialog.is_some() {
            self.handle_delete_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.project_dialog.is_some() {
            self.handle_project_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.settings_dialog.is_some() {
            self.handle_settings_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.keybind_help_open {
            self.handle_keybind_help_key(key_event);
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

    fn pane_border_style(&self, focused: bool) -> Style {
        let theme = ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    fn workspace_agent_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.peach,
            AgentType::Codex => theme.text,
        }
    }

    fn activity_effect_secondary_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.text,
            AgentType::Codex => theme.overlay0,
        }
    }

    fn activity_effect_gradient(&self, agent: AgentType) -> ColorGradient {
        let primary = self.workspace_agent_color(agent);
        let secondary = self.activity_effect_secondary_color(agent);
        ColorGradient::new(vec![(0.0, primary), (0.5, secondary), (1.0, primary)])
    }

    fn activity_effect_time(&self) -> f64 {
        self.fast_animation_frame as f64 * (FAST_ANIMATION_INTERVAL_MS as f64 / 1000.0)
    }

    fn render_activity_effect_label(
        &self,
        label: &str,
        agent: AgentType,
        area: Rect,
        frame: &mut Frame,
    ) {
        if area.is_empty() || label.is_empty() {
            return;
        }

        let primary = self.workspace_agent_color(agent);
        StyledText::new(label)
            .bold()
            .base_color(primary)
            .effect(TextEffect::AnimatedGradient {
                gradient: self.activity_effect_gradient(agent),
                speed: 1.8,
            })
            .time(self.activity_effect_time())
            .render(area, frame);
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

    fn workspace_display_name(workspace: &Workspace) -> String {
        if workspace.is_main {
            "base".to_string()
        } else {
            workspace.name.clone()
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let base_style = Style::new().bg(theme.crust).fg(theme.text);
        let left_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let repo_style = Style::new().bg(theme.mantle).fg(theme.subtext0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Grove ".to_string(), left_style),
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(format!(" {} ", self.repo_name), repo_style),
        ];
        if self.command_palette.is_visible() {
            left.push(FtSpan::styled(
                " [Palette] ".to_string(),
                Style::new().bg(theme.surface1).fg(theme.mauve).bold(),
            ));
        }

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
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
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let max_lines = usize::from(inner.height);
        if self.projects.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No projects configured",
                Style::new().fg(theme.subtext0),
            )]));
            lines.push(FtLine::raw(""));
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "Press 'p' to add a project",
                Style::new().fg(theme.text).bold(),
            )]));
        } else if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "Discovery error",
                    Style::new().fg(theme.red).bold(),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    message.as_str(),
                    Style::new().fg(theme.peach),
                )]));
            }
        } else {
            for (project_index, project) in self.projects.iter().enumerate() {
                if lines.len() >= max_lines {
                    break;
                }
                if project_index > 0 && lines.len() < max_lines {
                    lines.push(FtLine::raw(""));
                }
                if lines.len() >= max_lines {
                    break;
                }
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("▾ {}", project.name),
                    Style::new().fg(theme.overlay0).bold(),
                )]));

                let project_workspaces: Vec<(usize, &Workspace)> = self
                    .state
                    .workspaces
                    .iter()
                    .enumerate()
                    .filter(|(_, workspace)| {
                        workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                    })
                    .collect();

                if project_workspaces.is_empty() {
                    if lines.len() < max_lines {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            "  (no workspaces)",
                            Style::new().fg(theme.subtext0),
                        )]));
                    }
                    continue;
                }

                for (idx, workspace) in project_workspaces {
                    if lines
                        .len()
                        .saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT))
                        > max_lines
                    {
                        break;
                    }

                    let row_y = inner
                        .y
                        .saturating_add(u16::try_from(lines.len()).unwrap_or(u16::MAX));
                    let is_selected = idx == self.state.selected_index;
                    let is_working = self.status_is_visually_working(
                        Some(workspace.path.as_path()),
                        workspace.status,
                        is_selected,
                    );
                    let selected = if is_selected { "▸" } else { " " };
                    let row_background = if is_selected {
                        if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                            Some(theme.surface1)
                        } else {
                            Some(theme.surface0)
                        }
                    } else {
                        None
                    };

                    let mut primary_style = Style::new().fg(theme.text);
                    let mut secondary_style = Style::new().fg(theme.subtext0);
                    if let Some(bg) = row_background {
                        primary_style = primary_style.bg(bg);
                        secondary_style = secondary_style.bg(bg);
                    }
                    if is_selected {
                        primary_style = primary_style.bold();
                    }

                    let workspace_label_style = if is_working {
                        primary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold()
                    } else {
                        primary_style
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    let show_branch = workspace.branch != workspace_name;
                    let branch_text = if show_branch {
                        format!(" · {}", workspace.branch)
                    } else {
                        String::new()
                    };
                    let agent_separator = " · ";
                    let mut row_spans = vec![
                        FtSpan::styled(format!("{selected} "), primary_style),
                        FtSpan::styled(workspace_name.clone(), workspace_label_style),
                    ];
                    if !branch_text.is_empty() {
                        row_spans.push(FtSpan::styled(branch_text.clone(), secondary_style));
                    }
                    row_spans.push(FtSpan::styled(agent_separator, secondary_style));
                    row_spans.push(FtSpan::styled(
                        workspace.agent.label().to_string(),
                        secondary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold(),
                    ));
                    if workspace.is_orphaned {
                        row_spans.push(FtSpan::styled(
                            " · session ended",
                            secondary_style.fg(theme.peach),
                        ));
                    }
                    lines.push(FtLine::from_spans(row_spans));

                    if is_working {
                        let primary_label_x = inner.x.saturating_add(
                            u16::try_from(text_display_width("▸ ")).unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace_name.clone(),
                            workspace.agent,
                            primary_label_x,
                            row_y,
                        ));
                        let agent_prefix =
                            format!("{workspace_name}{branch_text}{agent_separator}");
                        let secondary_label_x = inner.x.saturating_add(
                            u16::try_from(
                                text_display_width("▸ ")
                                    .saturating_add(text_display_width(&agent_prefix)),
                            )
                            .unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace.agent.label().to_string(),
                            workspace.agent,
                            secondary_label_x,
                            row_y,
                        ));
                    }

                    if let Ok(data) = u64::try_from(idx) {
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
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
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
        let divider = std::iter::repeat_n(glyph, usize::from(area.height))
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

        let title = "Preview";
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
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let selected_workspace_header = selected_workspace.map(|workspace| {
            let workspace_name = Self::workspace_display_name(workspace);
            let is_working = self.status_is_visually_working(
                Some(workspace.path.as_path()),
                workspace.status,
                true,
            );
            let branch_label = if workspace.branch != workspace_name {
                Some(workspace.branch.clone())
            } else {
                None
            };
            let age_label = self.relative_age_label(workspace.last_activity_unix_secs);
            (
                workspace_name,
                branch_label,
                age_label,
                is_working,
                workspace.agent,
                workspace.is_orphaned,
            )
        });

        let metadata_rows = usize::from(PREVIEW_METADATA_ROWS);
        let preview_height = usize::from(inner.height)
            .saturating_sub(metadata_rows)
            .max(1);

        let mut text_lines = vec![if let Some((
            name_label,
            branch_label,
            age_label,
            is_working,
            agent,
            is_orphaned,
        )) = selected_workspace_header.as_ref()
        {
            let mut spans = vec![FtSpan::styled(
                name_label.clone(),
                if *is_working {
                    Style::new().fg(self.workspace_agent_color(*agent)).bold()
                } else {
                    Style::new().fg(theme.text).bold()
                },
            )];
            if let Some(branch_label) = branch_label {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    branch_label.clone(),
                    Style::new().fg(theme.subtext0),
                ));
            }
            spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            spans.push(FtSpan::styled(
                agent.label().to_string(),
                Style::new().fg(self.workspace_agent_color(*agent)).bold(),
            ));
            if !age_label.is_empty() {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    age_label.clone(),
                    Style::new().fg(theme.overlay0),
                ));
            }
            if *is_orphaned {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    "session ended",
                    Style::new().fg(theme.peach),
                ));
            }
            FtLine::from_spans(spans)
        } else {
            FtLine::from_spans(vec![FtSpan::styled(
                "none selected",
                Style::new().fg(theme.subtext0),
            )])
        }];
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut tab_spans = Vec::new();
        for (index, tab) in [PreviewTab::Agent, PreviewTab::Git]
            .iter()
            .copied()
            .enumerate()
        {
            if index > 0 {
                tab_spans.push(FtSpan::raw(" ".to_string()));
            }
            let style = if tab == self.preview_tab {
                tab_active_style
            } else {
                tab_inactive_style
            };
            tab_spans.push(FtSpan::styled(format!(" {} ", tab.label()), style));
        }
        if let Some(workspace) = selected_workspace {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                workspace.path.display().to_string(),
                Style::new().fg(theme.overlay0),
            ));
        } else {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                "no workspace",
                Style::new().fg(theme.overlay0),
            ));
        }
        text_lines.push(FtLine::from_spans(tab_spans));
        if let Some((name_label, branch_label, _, true, agent, _)) =
            selected_workspace_header.as_ref()
        {
            animated_labels.push((name_label.clone(), *agent, inner.x, inner.y));
            let branch_prefix = branch_label
                .as_ref()
                .map_or(String::new(), |branch| format!(" · {branch}"));
            let agent_prefix = format!("{name_label}{branch_prefix} · ");
            animated_labels.push((
                agent.label().to_string(),
                *agent,
                inner.x.saturating_add(
                    u16::try_from(text_display_width(&agent_prefix)).unwrap_or(u16::MAX),
                ),
                inner.y,
            ));
        }

        let mut visible_start = 0usize;
        let mut visible_plain_lines = Vec::new();
        match self.preview_tab {
            PreviewTab::Agent => {
                let visible_range = self.preview_visible_range_for_height(preview_height);
                visible_start = visible_range.0;
                let visible_end = visible_range.1;
                visible_plain_lines = self.preview_plain_lines_range(visible_start, visible_end);
                let mut visible_render_lines = if self.preview.render_lines.is_empty() {
                    Vec::new()
                } else {
                    let render_start = visible_start.min(self.preview.render_lines.len());
                    let render_end = visible_end.min(self.preview.render_lines.len());
                    if render_start < render_end {
                        self.preview.render_lines[render_start..render_end].to_vec()
                    } else {
                        Vec::new()
                    }
                };
                if visible_render_lines.len() < visible_plain_lines.len() {
                    visible_render_lines.extend(
                        visible_plain_lines[visible_render_lines.len()..]
                            .iter()
                            .cloned(),
                    );
                }
                if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
                    visible_render_lines = visible_plain_lines.clone();
                }
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
            }
            PreviewTab::Git => {
                if let Some(workspace) = selected_workspace {
                    let base_branch = workspace.base_branch.as_deref().unwrap_or("-");
                    let project_name = workspace.project_name.as_deref().unwrap_or("-");
                    let project_path = workspace
                        .project_path
                        .as_ref()
                        .map_or_else(|| "-".to_string(), |path| path.display().to_string());
                    let age_label = self.relative_age_label(workspace.last_activity_unix_secs);
                    let last_activity = if age_label.is_empty() {
                        "unknown".to_string()
                    } else {
                        age_label
                    };
                    let session_state = if workspace.is_orphaned {
                        "orphaned"
                    } else if workspace.status.has_session() {
                        "running"
                    } else {
                        "not running"
                    };
                    let lines = vec![
                        format!("Branch: {}", workspace.branch),
                        format!("Base branch: {base_branch}"),
                        format!("Status: {}", Self::workspace_status_label(workspace.status)),
                        format!("Session: {session_state}"),
                        format!(
                            "Main worktree: {}",
                            if workspace.is_main { "yes" } else { "no" }
                        ),
                        format!("Project: {project_name}"),
                        format!("Project path: {project_path}"),
                        format!("Workspace path: {}", workspace.path.display()),
                        format!("Last activity: {last_activity}"),
                    ];
                    text_lines.extend(lines.into_iter().take(preview_height).map(|line| {
                        FtLine::from_spans(vec![FtSpan::styled(line, Style::new().fg(theme.text))])
                    }));
                } else {
                    text_lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        "No workspace selected",
                        Style::new().fg(theme.subtext0),
                    )]));
                }
            }
        }

        Paragraph::new(FtText::from_lines(text_lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
        self.apply_preview_selection_highlight_cells(
            frame,
            inner,
            &visible_plain_lines,
            visible_start,
        );
    }

    fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let hints = self.keybind_hints_line();
        let base_style = Style::new().bg(theme.mantle).fg(theme.text);
        let chip_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let key_style = Style::new().bg(theme.mantle).fg(theme.lavender).bold();
        let text_style = Style::new().bg(theme.mantle).fg(theme.subtext0);
        let sep_style = Style::new().bg(theme.mantle).fg(theme.overlay0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Keys ".to_string(), chip_style),
            FtSpan::styled(" ".to_string(), base_style),
        ];
        left.extend(keybind_hint_spans(hints, text_style, key_style, sep_style));

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }

    fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }

    fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 11 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 11u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let unsafe_focused = focused(LaunchDialogField::Unsafe);
        let unsafe_state = if dialog.skip_permissions {
            "on, bypass approvals and sandbox"
        } else {
            "off, standard safety checks"
        };
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Launch profile", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Prompt",
                dialog.prompt.as_str(),
                "Describe initial task for the agent",
                focused(LaunchDialogField::Prompt),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "PreLaunch",
                dialog.pre_launch_command.as_str(),
                "Optional command to run before launch",
                focused(LaunchDialogField::PreLaunchCommand),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Unsafe",
                unsafe_state,
                unsafe_focused,
                theme.peach,
                if dialog.skip_permissions {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Start",
                "Cancel",
                start_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, Space toggle unsafe, Enter start, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);
        let content = OverlayModalContent {
            title: "Start Agent",
            body,
            theme,
            border_color: theme.mauve,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_LAUNCH_DIALOG))
            .render(area, frame);
    }

    fn render_delete_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.delete_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 16u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let warning_lines = if dialog.is_missing {
            (
                "  • Directory already removed",
                "  • Clean up git worktree metadata",
            )
        } else {
            (
                "  • Remove the working directory",
                "  • Uncommitted changes will be lost",
            )
        };
        let cleanup_focused = focused(DeleteDialogField::DeleteLocalBranch);
        let cleanup_state = if dialog.delete_local_branch {
            format!("enabled, remove '{}' branch locally", dialog.branch)
        } else {
            "disabled, keep local branch".to_string()
        };
        let delete_focused = focused(DeleteDialogField::DeleteButton);
        let cancel_focused = focused(DeleteDialogField::CancelButton);
        let delete_hint = pad_or_truncate_to_display_width(
            "Tab move, Space toggle branch cleanup, Enter or D delete, Esc cancel",
            content_width,
        );
        let path = dialog.path.display().to_string();
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Deletion plan", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                theme.blue,
                theme.overlay0,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  [Risk] Changes are destructive", content_width),
                Style::new().fg(theme.peach).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.0,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.1,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "BranchCleanup",
                cleanup_state.as_str(),
                cleanup_focused,
                theme.peach,
                if dialog.delete_local_branch {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Delete",
                "Cancel",
                delete_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                delete_hint,
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Delete Worktree?",
            body,
            theme,
            border_color: theme.red,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_DELETE_DIALOG))
            .render(area, frame);
    }

    fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };
        if area.width < 40 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 12u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let current = dialog.multiplexer.label();
        let multiplexer_focused = focused(SettingsDialogField::Multiplexer);
        let save_focused = focused(SettingsDialogField::SaveButton);
        let cancel_focused = focused(SettingsDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Global settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Multiplexer",
                current,
                multiplexer_focused,
                theme.blue,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  h/l, Left/Right, Space toggles", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                save_focused,
                cancel_focused,
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Saved to ~/.config/grove/config.toml",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Settings",
            body,
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_SETTINGS_DIALOG))
            .render(area, frame);
    }

    fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog.as_ref() else {
            return;
        };
        if area.width < 44 || area.height < 14 {
            return;
        }

        let theme = ui_theme();
        let dialog_width = area.width.saturating_sub(8).min(96);
        let content_width = usize::from(dialog_width.saturating_sub(2));

        if let Some(add_dialog) = dialog.add_dialog.as_ref() {
            let dialog_height = 12u16;
            let focused = |field| add_dialog.focused_field == field;
            let body = FtText::from_lines(vec![
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Name",
                    add_dialog.name.as_str(),
                    "Optional, defaults to directory name",
                    focused(ProjectAddDialogField::Name),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Path",
                    add_dialog.path.as_str(),
                    "Absolute path or ~/path to repo root",
                    focused(ProjectAddDialogField::Path),
                ),
                FtLine::raw(""),
                modal_actions_row(
                    content_width,
                    theme,
                    "Add",
                    "Cancel",
                    focused(ProjectAddDialogField::AddButton),
                    focused(ProjectAddDialogField::CancelButton),
                ),
                FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "Tab move, Enter confirm, Esc back",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]),
            ]);
            let content = OverlayModalContent {
                title: "Add Project",
                body,
                theme,
                border_color: theme.mauve,
            };

            Modal::new(content)
                .size(
                    ModalSizeConstraints::new()
                        .min_width(dialog_width)
                        .max_width(dialog_width)
                        .min_height(dialog_height)
                        .max_height(dialog_height),
                )
                .backdrop(BackdropConfig::new(theme.crust, 0.55))
                .hit_id(HitId::new(HIT_ID_PROJECT_ADD_DIALOG))
                .render(area, frame);
            return;
        }

        let mut lines = Vec::new();
        lines.push(modal_labeled_input_row(
            content_width,
            theme,
            "Filter",
            dialog.filter.as_str(),
            "Type project name or path",
            true,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                format!("{} projects", self.projects.len()).as_str(),
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        lines.push(FtLine::raw(""));

        if dialog.filtered_project_indices.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No matches",
                Style::new().fg(theme.subtext0),
            )]));
        } else {
            for (visible_index, project_index) in
                dialog.filtered_project_indices.iter().take(8).enumerate()
            {
                let Some(project) = self.projects.get(*project_index) else {
                    continue;
                };
                let selected = visible_index == dialog.selected_filtered_index;
                let marker = if selected { ">" } else { " " };
                let name_style = if selected {
                    Style::new().fg(theme.mauve).bold()
                } else {
                    Style::new().fg(theme.text)
                };
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("{marker} {}", project.name),
                    name_style,
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        format!("  {}", project.path.display()).as_str(),
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Enter focus, Up/Down or Tab/S-Tab navigate, Ctrl+A add, Esc close",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));

        let content = OverlayModalContent {
            title: "Projects",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(16)
                    .max_height(20),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG))
            .render(area, frame);
    }

    fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.command_palette.is_visible() {
            return;
        }

        self.command_palette.render(area, frame);
    }

    fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.keybind_help_open {
            return;
        }
        if area.width < 56 || area.height < 18 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(108);
        let dialog_height = area.height.saturating_sub(6).clamp(18, 26);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));

        let lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Global]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  ? help, q quit, Tab/h/l switch pane, Enter open/attach, Esc list pane",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  n new, e edit, p projects, s start, x stop, D delete, S settings, ! unsafe toggle",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  Ctrl+K command palette", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[List]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  j/k or Up/Down move selection", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Preview]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  [/] switch tab, j/k or Up/Down scroll, PgUp/PgDn page, G bottom",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Interactive]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  type sends input to agent", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Esc Esc or Ctrl+\\ exit, Alt+C copy, Alt+V paste",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Create: Tab/S-Tab fields, j/k or C-n/C-p move, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Edit:   Tab/S-Tab fields, h/l or Space toggle agent, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Start:  Tab/S-Tab fields, Space toggle unsafe, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Delete: Tab/S-Tab fields, j/k move, Space toggle, Enter/D confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Close help: Esc, Enter, or ?", content_width),
                Style::new().fg(theme.lavender).bold(),
            )]),
        ];

        let content = OverlayModalContent {
            title: "Keybind Help",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.blue,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_KEYBIND_HELP_DIALOG))
            .render(area, frame);
    }

    fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 16u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let selected_project_label = self
            .projects
            .get(dialog.project_index)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "(missing project)".to_string());

        let focused = |field| dialog.focused_field == field;
        let selected_agent = dialog.agent;
        let selected_agent_style = Style::new()
            .fg(theme.text)
            .bg(if focused(CreateDialogField::Agent) {
                theme.surface1
            } else {
                theme.base
            })
            .bold();
        let unselected_agent_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let selected_dropdown_style = Style::new().fg(theme.text).bg(theme.surface1).bold();
        let unselected_dropdown_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let agent_row = |agent: AgentType| {
            let is_selected = selected_agent == agent;
            let prefix = if is_selected { "▸" } else { " " };
            let line = pad_or_truncate_to_display_width(
                format!("{} [Agent] {}", prefix, agent.label()).as_str(),
                content_width,
            );
            if is_selected {
                FtLine::from_spans(vec![FtSpan::styled(line, selected_agent_style)])
            } else {
                FtLine::from_spans(vec![FtSpan::styled(line, unselected_agent_style)])
            }
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace setup", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                "feature-name",
                focused(CreateDialogField::WorkspaceName),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "Project",
                selected_project_label.as_str(),
                "j/k or C-n/C-p select",
                focused(CreateDialogField::Project),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "BaseBranch",
                dialog.base_branch.as_str(),
                "current branch (fallback: main/master)",
                focused(CreateDialogField::BaseBranch),
            ),
        ];
        if focused(CreateDialogField::Project)
            && let Some(project) = self.projects.get(dialog.project_index)
        {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [ProjectPath] {}", project.path.display()).as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]));
        }
        if focused(CreateDialogField::BaseBranch) {
            if self.create_branch_all.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] Loading branches...",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else if self.create_branch_filtered.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] No matching branches",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else {
                let max_dropdown = 4usize;
                for (index, branch) in self.create_branch_filtered.iter().enumerate() {
                    if index >= max_dropdown {
                        break;
                    }
                    let is_selected = index == self.create_branch_index;
                    let prefix = if is_selected { "▸" } else { " " };
                    let line = pad_or_truncate_to_display_width(
                        format!("{prefix} [Branches] {branch}").as_str(),
                        content_width,
                    );
                    if is_selected {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            selected_dropdown_style,
                        )]));
                    } else {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            unselected_dropdown_style,
                        )]));
                    }
                }
                if self.create_branch_filtered.len() > max_dropdown {
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        pad_or_truncate_to_display_width(
                            format!(
                                "  [Branches] ... and {} more",
                                self.create_branch_filtered.len() - max_dropdown
                            )
                            .as_str(),
                            content_width,
                        ),
                        Style::new().fg(theme.overlay0),
                    )]));
                }
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(agent_row(AgentType::Claude));
        lines.push(agent_row(AgentType::Codex));
        lines.push(FtLine::raw(""));
        let create_focused = focused(CreateDialogField::CreateButton);
        let cancel_focused = focused(CreateDialogField::CancelButton);
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Create",
            "Cancel",
            create_focused,
            cancel_focused,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Tab move, j/k or C-n/C-p adjust project/branch, Enter create, Esc cancel",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        let content = OverlayModalContent {
            title: "New Workspace",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.mauve,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_CREATE_DIALOG))
            .render(area, frame);
    }

    fn render_edit_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.edit_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(10).min(80);
        let dialog_height = 13u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let path = dialog.workspace_path.display().to_string();
        let running_note = if dialog.was_running {
            "Running now, restart agent to apply change"
        } else {
            "Agent change applies on next agent start"
        };

        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                theme.blue,
                theme.overlay0,
            ),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Agent",
                dialog.agent.label(),
                focused(EditDialogField::Agent),
                theme.peach,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [Note] {running_note}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                focused(EditDialogField::SaveButton),
                focused(EditDialogField::CancelButton),
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, h/l or Space toggle agent, Enter save, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Edit Workspace",
            body,
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_EDIT_DIALOG))
            .render(area, frame);
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
            "Workspaces (j/k, arrows, Tab/h/l focus, Enter preview, n create, e edit, s/x start-stop, D delete, S settings, ? help, ! unsafe, Esc list, mouse)"
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
                        "▸"
                    } else {
                        " "
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    lines.push(format!(
                        "{} {} | {} | {} | {}{}",
                        selected,
                        workspace_name,
                        workspace.branch,
                        workspace.agent.label(),
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
            lines.push(format!("Field: {}", dialog.focused_field.label()));
            lines.push(format!(
                "Prompt: {}",
                if dialog.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.prompt.clone()
                }
            ));
            lines.push(format!(
                "Pre-launch command: {}",
                if dialog.pre_launch_command.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.pre_launch_command.clone()
                }
            ));
            lines.push(format!(
                "Unsafe launch: {}",
                if dialog.skip_permissions { "on" } else { "off" }
            ));
        }
        if let Some(dialog) = &self.delete_dialog {
            lines.push(String::new());
            lines.push("Delete Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.branch));
            lines.push(format!(
                "Delete local branch: {}",
                if dialog.delete_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    Self::workspace_display_name(workspace),
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
                let _ = self
                    .notifications
                    .tick(Duration::from_millis(TOAST_TICK_INTERVAL_MS));
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
            Msg::DeleteWorkspaceCompleted(completion) => {
                self.apply_delete_workspace_completion(completion);
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
        frame.set_cursor(None);
        frame.set_cursor_visible(false);
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
        self.render_edit_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        self.render_delete_dialog_overlay(frame, area);
        self.render_settings_dialog_overlay(frame, area);
        self.render_project_dialog_overlay(frame, area);
        self.render_keybind_help_overlay(frame, area);
        self.render_command_palette_overlay(frame, area);
        self.render_toasts(frame, area);
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
        AppDependencies, AppPaths, ClipboardAccess, CommandZellijInput, CreateDialogField,
        CreateWorkspaceCompletion, CursorCapture, DeleteDialogField, GroveApp, HIT_ID_HEADER,
        HIT_ID_PREVIEW, HIT_ID_STATUS, HIT_ID_WORKSPACE_LIST, HIT_ID_WORKSPACE_ROW,
        LaunchDialogField, LaunchDialogState, LivePreviewCapture, Msg, PALETTE_CMD_FOCUS_LIST,
        PALETTE_CMD_MOVE_SELECTION_DOWN, PALETTE_CMD_OPEN_PREVIEW, PALETTE_CMD_SCROLL_DOWN,
        PREVIEW_METADATA_ROWS, PendingResizeVerification, PreviewPollCompletion, PreviewTab,
        StartAgentCompletion, StopAgentCompletion, TextSelectionPoint, TmuxInput,
        WORKSPACE_ITEM_HEIGHT, WorkspaceStatusCapture, ansi_16_color, ansi_line_to_styled_line,
        parse_cursor_metadata, ui_theme,
    };
    use crate::adapters::{BootstrapData, DiscoveryState};
    use crate::config::{MultiplexerKind, ProjectConfig};
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use crate::event_log::{Event as LoggedEvent, EventLogger, NullEventLogger};
    use crate::interactive::InteractiveState;
    use crate::state::{PaneFocus, UiMode};
    use crate::workspace_lifecycle::{BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult};
    use ftui::core::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        PasteEvent,
    };
    use ftui::render::frame::HitId;
    use ftui::widgets::block::Block;
    use ftui::widgets::borders::Borders;
    use ftui::widgets::toast::ToastStyle;
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
        let mut main_workspace = Workspace::try_new(
            "grove".to_string(),
            PathBuf::from("/repos/grove"),
            "main".to_string(),
            Some(1_700_000_200),
            AgentType::Claude,
            WorkspaceStatus::Main,
            true,
        )
        .expect("workspace should be valid");
        main_workspace.project_path = Some(PathBuf::from("/repos/grove"));

        let mut feature_workspace = Workspace::try_new(
            "feature-a".to_string(),
            PathBuf::from("/repos/grove-feature-a"),
            "feature-a".to_string(),
            Some(1_700_000_100),
            AgentType::Codex,
            status,
            false,
        )
        .expect("workspace should be valid");
        feature_workspace.project_path = Some(PathBuf::from("/repos/grove"));

        BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: vec![main_workspace, feature_workspace],
            discovery_state: DiscoveryState::Ready,
            orphaned_sessions: Vec::new(),
        }
    }

    fn fixture_projects() -> Vec<ProjectConfig> {
        vec![ProjectConfig {
            name: "grove".to_string(),
            path: PathBuf::from("/repos/grove"),
        }]
    }

    fn fixture_app() -> GroveApp {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture");
        let config_path = unique_config_path("fixture");
        GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(WorkspaceStatus::Idle),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(RecordingTmuxInput {
                    commands: Rc::new(RefCell::new(Vec::new())),
                    captures: Rc::new(RefCell::new(Vec::new())),
                    cursor_captures: Rc::new(RefCell::new(Vec::new())),
                    calls: Rc::new(RefCell::new(Vec::new())),
                }),
                clipboard: test_clipboard(),
                paths: AppPaths::new(sidebar_ratio_path, config_path),
                multiplexer: MultiplexerKind::Tmux,
                event_log: Box::new(NullEventLogger),
                debug_record_start_ts: None,
            },
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

    fn unique_config_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        std::env::temp_dir().join(format!(
            "grove-config-{label}-{}-{timestamp}.toml",
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
            GroveApp::from_parts_with_clipboard_and_projects(
                fixture_bootstrap(status),
                fixture_projects(),
                AppDependencies {
                    tmux_input: Box::new(tmux),
                    clipboard: test_clipboard(),
                    paths: AppPaths::new(
                        sidebar_ratio_path,
                        unique_config_path("fixture-with-tmux"),
                    ),
                    multiplexer: MultiplexerKind::Tmux,
                    event_log: Box::new(NullEventLogger),
                    debug_record_start_ts: None,
                },
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
            GroveApp::from_parts_with_clipboard_and_projects(
                fixture_bootstrap(status),
                fixture_projects(),
                AppDependencies {
                    tmux_input: Box::new(tmux),
                    clipboard: test_clipboard(),
                    paths: AppPaths::new(
                        sidebar_ratio_path,
                        unique_config_path("fixture-with-calls"),
                    ),
                    multiplexer: MultiplexerKind::Tmux,
                    event_log: Box::new(NullEventLogger),
                    debug_record_start_ts: None,
                },
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
            GroveApp::from_parts_with_clipboard_and_projects(
                fixture_bootstrap(status),
                fixture_projects(),
                AppDependencies {
                    tmux_input: Box::new(tmux),
                    clipboard: test_clipboard(),
                    paths: AppPaths::new(
                        sidebar_ratio_path,
                        unique_config_path("fixture-with-events"),
                    ),
                    multiplexer: MultiplexerKind::Tmux,
                    event_log: Box::new(event_log),
                    debug_record_start_ts: None,
                },
            ),
            commands,
            captures,
            cursor_captures,
            events,
        )
    }

    fn fixture_background_app(status: WorkspaceStatus) -> GroveApp {
        GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(status),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(BackgroundOnlyTmuxInput),
                clipboard: test_clipboard(),
                paths: AppPaths::new(
                    unique_sidebar_ratio_path("background"),
                    unique_config_path("background"),
                ),
                multiplexer: MultiplexerKind::Tmux,
                event_log: Box::new(NullEventLogger),
                debug_record_start_ts: None,
            },
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
                let active_modals = [
                    app.launch_dialog.is_some(),
                    app.create_dialog.is_some(),
                    app.delete_dialog.is_some(),
                    app.keybind_help_open,
                    app.command_palette.is_visible(),
                    app.interactive.is_some(),
                ]
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
    fn workspace_age_renders_in_preview_header_not_sidebar_row() {
        let mut app = fixture_app();
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_secs();
        let last_activity =
            i64::try_from(now_secs.saturating_sub(17 * 60)).expect("timestamp should fit i64");
        app.state.workspaces[0].last_activity_unix_secs = Some(last_activity);
        app.state.selected_index = 0;
        let expected_age = app.relative_age_label(app.state.workspaces[0].last_activity_unix_secs);

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let sidebar_x_start = layout.sidebar.x.saturating_add(1);
        let sidebar_x_end = layout.sidebar.right().saturating_sub(1);
        let preview_x_start = layout.preview.x.saturating_add(1);
        let preview_x_end = layout.preview.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(sidebar_row) =
                find_row_containing(frame, "▸ base", sidebar_x_start, sidebar_x_end)
            else {
                panic!("sidebar workspace row should be rendered");
            };
            let sidebar_text = row_text(frame, sidebar_row, sidebar_x_start, sidebar_x_end);
            assert!(
                !sidebar_text.contains(expected_age.as_str()),
                "sidebar row should not include age label, got: {sidebar_text}"
            );

            let Some(preview_row) = find_row_containing(
                frame,
                "base · main · Claude",
                preview_x_start,
                preview_x_end,
            ) else {
                panic!("preview header row should be rendered");
            };
            let preview_text = row_text(frame, preview_row, preview_x_start, preview_x_end);
            assert!(
                preview_text.contains(expected_age.as_str()),
                "preview header should include age label, got: {preview_text}"
            );
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
                rendered_row.starts_with("▸ "),
                "selected row should start with selection marker, got: {rendered_row}"
            );
        });
    }

    #[test]
    fn sidebar_row_omits_duplicate_workspace_and_branch_text() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("feature row should be rendered");
            };
            let row_text = row_text(frame, row, x_start, x_end);
            assert!(
                !row_text.contains("feature-a · feature-a"),
                "row should not duplicate workspace and branch when they match, got: {row_text}"
            );
            assert!(
                row_text.contains("feature-a · Codex"),
                "row should include workspace and agent labels, got: {row_text}"
            );
        });
    }

    #[test]
    fn shell_lines_show_workspace_and_agent_labels_without_status_badges() {
        let app = fixture_app();
        let lines = app.shell_lines(12);
        let Some(base_line) = lines.iter().find(|line| line.contains("base | main")) else {
            panic!("base workspace shell line should be present");
        };
        let Some(feature_line) = lines
            .iter()
            .find(|line| line.contains("feature-a | feature-a"))
        else {
            panic!("feature workspace shell line should be present");
        };
        assert!(
            !base_line.contains("["),
            "base workspace should not show status badge, got: {base_line}"
        );
        assert!(
            !feature_line.contains("["),
            "feature workspace should not show status badge, got: {feature_line}"
        );
        assert!(
            base_line.contains("Claude"),
            "base workspace should include Claude label, got: {base_line}"
        );
        assert!(
            feature_line.contains("Codex"),
            "feature workspace should include Codex label, got: {feature_line}"
        );
    }

    #[test]
    fn active_workspace_without_recent_activity_uses_static_indicators() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = false;
        app.agent_output_changing = false;
        assert!(!app.status_is_visually_working(
            Some(app.state.workspaces[1].path.as_path()),
            WorkspaceStatus::Active,
            true
        ));

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(
                !sidebar_row_text.contains("["),
                "active workspace should not show status badge when not changing, got: {sidebar_row_text}"
            );
            assert!(!sidebar_row_text.contains("run."));

            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(!status_text.contains("run."));
        });
    }

    #[test]
    fn active_workspace_with_recent_activity_window_animates_indicators() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = false;
        app.agent_output_changing = false;
        app.push_agent_activity_frame(true);
        assert!(app.status_is_visually_working(
            Some(app.state.workspaces[1].path.as_path()),
            WorkspaceStatus::Active,
            true
        ));

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(!sidebar_row_text.contains("run."));
        });
    }

    #[test]
    fn active_workspace_with_recent_activity_animates_indicators() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = true;
        app.agent_output_changing = true;
        assert!(app.status_is_visually_working(
            Some(app.state.workspaces[1].path.as_path()),
            WorkspaceStatus::Active,
            true
        ));

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(!sidebar_row_text.contains("run."));
        });
    }

    #[test]
    fn active_workspace_activity_window_expires_after_inactive_frames() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.state.selected_index = 1;
        app.output_changing = false;
        app.agent_output_changing = false;
        app.push_agent_activity_frame(true);
        for _ in 0..super::AGENT_ACTIVITY_WINDOW_FRAMES {
            app.push_agent_activity_frame(false);
        }
        assert!(!app.status_is_visually_working(
            Some(app.state.workspaces[1].path.as_path()),
            WorkspaceStatus::Active,
            true
        ));

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(!sidebar_row_text.contains("run."));
        });
    }

    #[test]
    fn waiting_workspace_row_has_no_status_badge_or_input_banner() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Waiting, Vec::new());
        app.state.selected_index = 1;
        app.sidebar_width_pct = 70;

        let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 120, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(
                !sidebar_row_text.contains("["),
                "waiting workspace should not show status badge, got: {sidebar_row_text}"
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
                    idle_header, active_header,
                    "header layout should remain stable when spinner state changes"
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
        assert!(!app.status_is_visually_working(
            Some(app.state.workspaces[1].path.as_path()),
            WorkspaceStatus::Active,
            true
        ));

        let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
        let x_start = layout.sidebar.x.saturating_add(1);
        let x_end = layout.sidebar.right().saturating_sub(1);

        with_rendered_frame(&app, 80, 24, |frame| {
            let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
                panic!("selected workspace row should be rendered");
            };
            let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
            assert!(!sidebar_row_text.contains("run."));

            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(
                status_text.contains("j/k move, h/l pane, Enter open"),
                "status row should show keybind hints, got: {status_text}"
            );
        });
    }

    #[test]
    fn modal_dialog_renders_over_sidebar() {
        let mut app = fixture_app();
        app.launch_dialog = Some(LaunchDialogState {
            prompt: String::new(),
            pre_launch_command: String::new(),
            skip_permissions: false,
            focused_field: LaunchDialogField::Prompt,
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
            pre_launch_command: String::new(),
            skip_permissions: false,
            focused_field: LaunchDialogField::Prompt,
        });

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(100);
            let dialog_height = 11u16;
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
            let dialog_height = 14u16;
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
    fn create_dialog_selected_agent_row_uses_highlight_background() {
        let mut app = fixture_app();
        app.open_create_dialog();
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
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(90);
            let dialog_height = 14u16;
            let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
            let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
            let x_start = dialog_x.saturating_add(1);
            let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
            let y_start = dialog_y.saturating_add(1);
            let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));
            let find_dialog_row = |needle: &str| {
                (y_start..y_end).find(|&row| row_text(frame, row, x_start, x_end).contains(needle))
            };

            let Some(selected_row) = find_dialog_row("Claude") else {
                panic!("selected agent row should be rendered");
            };
            assert_row_bg(frame, selected_row, x_start, x_end, ui_theme().surface1);

            let Some(unselected_row) = find_dialog_row("Codex") else {
                panic!("unselected agent row should be rendered");
            };
            assert_row_bg(frame, unselected_row, x_start, x_end, ui_theme().base);

            let Some(cell) = frame.buffer.get(x_start, dialog_y.saturating_add(1)) else {
                panic!(
                    "expected dialog cell at ({x_start},{})",
                    dialog_y.saturating_add(1)
                );
            };
            assert_eq!(cell.bg, ui_theme().base);
        });
    }

    #[test]
    fn create_dialog_unfocused_agent_row_uses_base_background() {
        let mut app = fixture_app();
        app.open_create_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(90);
            let dialog_height = 14u16;
            let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
            let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
            let x_start = dialog_x.saturating_add(1);
            let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
            let y_start = dialog_y.saturating_add(1);
            let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));
            let find_dialog_row = |needle: &str| {
                (y_start..y_end).find(|&row| row_text(frame, row, x_start, x_end).contains(needle))
            };

            let Some(name_row) = find_dialog_row("[Name]") else {
                panic!("name row should be rendered");
            };
            assert_row_bg(frame, name_row, x_start, x_end, ui_theme().surface1);

            let Some(selected_agent_row) = find_dialog_row("Claude") else {
                panic!("selected agent row should be rendered");
            };
            assert_row_bg(frame, selected_agent_row, x_start, x_end, ui_theme().base);
        });
    }

    #[test]
    fn create_dialog_renders_action_buttons() {
        let mut app = fixture_app();
        app.open_create_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let dialog_width = frame.width().saturating_sub(8).min(90);
            let dialog_height = 14u16;
            let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
            let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
            let x_start = dialog_x.saturating_add(1);
            let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
            let y_start = dialog_y.saturating_add(1);
            let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));

            let has_buttons = (y_start..y_end).any(|row| {
                let text = row_text(frame, row, x_start, x_end);
                text.contains("Create") && text.contains("Cancel")
            });
            assert!(
                has_buttons,
                "create dialog action buttons should be visible"
            );
        });
    }

    #[test]
    fn status_row_shows_keybind_hints_not_toast_state() {
        let mut app = fixture_app();
        app.show_toast("Agent started", false);

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(!status_text.contains("Agent started"));
            assert!(status_text.contains("j/k move, h/l pane, Enter open"));
        });
    }

    #[test]
    fn status_row_shows_start_hint_in_preview_mode() {
        let mut app = fixture_app();
        app.state.mode = UiMode::Preview;
        app.state.focus = PaneFocus::Preview;

        with_rendered_frame(&app, 180, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("s start"));
            assert!(status_text.contains("x stop"));
            assert!(status_text.contains("D delete"));
        });
    }

    #[test]
    fn question_key_opens_keybind_help_modal() {
        let mut app = fixture_app();

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('?')).with_kind(KeyEventKind::Press));

        assert!(app.keybind_help_open);
    }

    #[test]
    fn keybind_help_modal_closes_on_escape() {
        let mut app = fixture_app();
        app.keybind_help_open = true;

        let _ = app.handle_key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press));

        assert!(!app.keybind_help_open);
    }

    #[test]
    fn keybind_help_modal_blocks_navigation_keys() {
        let mut app = fixture_app();
        app.keybind_help_open = true;
        let selected_before = app.state.selected_index;

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));

        assert_eq!(app.state.selected_index, selected_before);
    }

    #[test]
    fn ctrl_k_opens_command_palette() {
        let mut app = fixture_app();

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );

        assert!(app.command_palette.is_visible());
    }

    #[test]
    fn ctrl_k_is_blocked_while_modal_is_open() {
        let mut app = fixture_app();
        app.open_create_dialog();
        assert!(app.create_dialog.is_some());

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );

        assert!(app.create_dialog.is_some());
        assert!(!app.command_palette.is_visible());
    }

    #[test]
    fn ctrl_k_is_blocked_in_interactive_mode() {
        let mut app = fixture_app();
        app.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            24,
            80,
        ));

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );

        assert!(app.interactive.is_some());
        assert!(!app.command_palette.is_visible());
    }

    #[test]
    fn command_palette_blocks_background_navigation_keys() {
        let mut app = fixture_app();
        let selected_before = app.state.selected_index;

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert!(app.command_palette.is_visible());

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));

        assert_eq!(app.state.selected_index, selected_before);
        assert_eq!(app.command_palette.query(), "j");
    }

    #[test]
    fn command_palette_enter_executes_selected_action() {
        let mut app = fixture_app();

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert!(app.command_palette.is_visible());

        for character in ['n', 'e', 'w'] {
            let _ = app
                .handle_key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press));
        }
        let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

        assert!(!app.command_palette.is_visible());
        assert!(app.create_dialog.is_some());
    }

    #[test]
    fn command_palette_action_set_scopes_to_focus_and_mode() {
        let mut app = fixture_app();
        let list_ids: Vec<String> = app
            .build_command_palette_actions()
            .into_iter()
            .map(|action| action.id)
            .collect();
        assert!(
            list_ids
                .iter()
                .any(|id| id == PALETTE_CMD_MOVE_SELECTION_DOWN)
        );
        assert!(list_ids.iter().any(|id| id == PALETTE_CMD_OPEN_PREVIEW));
        assert!(!list_ids.iter().any(|id| id == PALETTE_CMD_SCROLL_DOWN));

        app.state.mode = UiMode::Preview;
        app.state.focus = PaneFocus::Preview;
        let preview_ids: Vec<String> = app
            .build_command_palette_actions()
            .into_iter()
            .map(|action| action.id)
            .collect();
        assert!(preview_ids.iter().any(|id| id == PALETTE_CMD_SCROLL_DOWN));
        assert!(preview_ids.iter().any(|id| id == PALETTE_CMD_FOCUS_LIST));
        assert!(
            !preview_ids
                .iter()
                .any(|id| id == PALETTE_CMD_MOVE_SELECTION_DOWN)
        );
    }

    #[test]
    fn uppercase_s_opens_settings_dialog() {
        let mut app = fixture_app();

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));

        assert!(app.settings_dialog.is_some());
    }

    #[test]
    fn settings_dialog_save_switches_multiplexer_and_persists_config() {
        let mut app = fixture_app();
        assert_eq!(app.multiplexer, MultiplexerKind::Tmux);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
        assert!(app.settings_dialog.is_some());

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

        assert!(app.settings_dialog.is_none());
        assert_eq!(app.multiplexer, MultiplexerKind::Zellij);
        let loaded = crate::config::load_from_path(&app.config_path).expect("config should load");
        assert_eq!(loaded.multiplexer, MultiplexerKind::Zellij);
    }

    #[test]
    fn settings_dialog_blocks_switch_when_workspace_running() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        assert_eq!(app.multiplexer, MultiplexerKind::Tmux);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
        assert!(app.settings_dialog.is_some());

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

        assert!(app.settings_dialog.is_some());
        assert_eq!(app.multiplexer, MultiplexerKind::Tmux);
        assert!(app.status_bar_line().contains("stop running workspaces"));
    }

    #[test]
    fn zellij_capture_session_output_emulates_ansi_from_session_log() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let session_name = format!(
            "grove-ws-zellij-emulator-test-{}-{timestamp}",
            std::process::id()
        );
        let log_path = crate::agent_runtime::zellij_capture_log_path(&session_name);
        let log_dir = log_path
            .parent()
            .expect("capture log path should have parent")
            .to_path_buf();
        fs::create_dir_all(&log_dir).expect("capture log directory should exist");
        fs::write(
            &log_path,
            concat!(
                "Script started on 2026-02-14 21:24:17-05:00 [COMMAND=\"codex\"]\n",
                "\0line one\n",
                "\u{1b}[31mline two red\u{1b}[0m\n",
                "\u{1b}[32mline three green\u{1b}[0m\n",
                "Script done on 2026-02-14 21:25:06-05:00 [COMMAND_EXIT_CODE=\"0\"]\n"
            ),
        )
        .expect("capture log should be written");
        let input = CommandZellijInput::default();

        let captured = input
            .capture_session_output(&session_name, 4)
            .expect("capture should load from log file");

        assert!(captured.contains("line one"));
        assert!(captured.contains("line two red"));
        assert!(captured.contains("line three green"));
        assert!(captured.contains("exited with code 0"));
        assert!(captured.contains("\u{1b}["));
        assert!(!captured.contains("Script started on "));
        assert!(!captured.contains("Script done on "));

        let _ = fs::remove_file(log_path);
    }

    #[test]
    fn zellij_capture_session_output_returns_empty_when_log_missing() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let session_name = format!(
            "grove-ws-zellij-missing-log-{}-{timestamp}",
            std::process::id()
        );
        let input = CommandZellijInput::default();

        let captured = input
            .capture_session_output(&session_name, 50)
            .expect("missing log should return empty output");
        assert!(captured.is_empty());
    }

    #[test]
    fn status_row_shows_help_close_hint_when_help_modal_open() {
        let mut app = fixture_app();
        app.keybind_help_open = true;

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Esc/? close help"));
        });
    }

    #[test]
    fn status_row_shows_palette_hints_when_palette_open() {
        let mut app = fixture_app();
        app.open_command_palette();

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Type to search"));
            assert!(status_text.contains("Enter run"));
        });
    }

    #[test]
    fn toast_overlay_renders_message() {
        let mut app = fixture_app();
        app.show_toast("Copied 2 line(s)", false);

        with_rendered_frame(&app, 80, 24, |frame| {
            let found = (0..frame.height())
                .any(|row| row_text(frame, row, 0, frame.width()).contains("Copied 2 line(s)"));
            assert!(found, "toast message should render in frame");
        });
    }

    #[test]
    fn interactive_copy_sets_success_toast_message() {
        let mut app = fixture_app();
        app.preview.lines = vec!["alpha".to_string()];
        app.preview.render_lines = app.preview.lines.clone();

        app.copy_interactive_selection_or_visible();

        let Some(toast) = app.notifications.visible().last() else {
            panic!("copy should set toast message");
        };
        assert!(matches!(toast.config.style_variant, ToastStyle::Success));
        assert_eq!(toast.content.message, "Copied 1 line(s)");
    }

    #[test]
    fn status_row_shows_create_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.open_create_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Tab/S-Tab field"));
            assert!(status_text.contains("Enter select/create"));
        });
    }

    #[test]
    fn status_row_shows_edit_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.open_edit_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Space toggle agent"));
            assert!(status_text.contains("Enter save/select"));
        });
    }

    #[test]
    fn status_row_shows_launch_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.launch_dialog = Some(LaunchDialogState {
            prompt: String::new(),
            pre_launch_command: String::new(),
            skip_permissions: false,
            focused_field: LaunchDialogField::Prompt,
        });

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Tab/S-Tab field"));
            assert!(status_text.contains("Enter select/start"));
        });
    }

    #[test]
    fn status_row_shows_delete_dialog_keybind_hints_when_modal_open() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        app.open_delete_dialog();

        with_rendered_frame(&app, 80, 24, |frame| {
            let status_row = frame.height().saturating_sub(1);
            let status_text = row_text(frame, status_row, 0, frame.width());
            assert!(status_text.contains("Tab/S-Tab field"));
            assert!(status_text.contains("Space toggle"));
        });
    }

    #[test]
    fn view_hides_terminal_cursor_without_focused_input_widget() {
        let app = fixture_app();

        with_rendered_frame(&app, 80, 24, |frame| {
            assert!(frame.cursor_position.is_none());
            assert!(!frame.cursor_visible);
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
                Some(HitId::new(HIT_ID_WORKSPACE_LIST))
            );
            assert_eq!(
                frame
                    .hit_test(sidebar_inner.x, sidebar_inner.y.saturating_add(1))
                    .map(|hit| hit.0),
                Some(HitId::new(HIT_ID_WORKSPACE_ROW))
            );
            assert_eq!(
                frame
                    .hit_test(sidebar_inner.x, sidebar_inner.y.saturating_add(1))
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
        let second_row_y = sidebar_inner
            .y
            .saturating_add(1)
            .saturating_add(WORKSPACE_ITEM_HEIGHT);

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
        assert!(kinds.iter().any(|kind| kind == "toast_shown"));
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
            AppPaths::new(sidebar_ratio_path, unique_config_path("background-poll")),
            MultiplexerKind::Tmux,
            Box::new(NullEventLogger),
            None,
        );
        app.state.selected_index = 1;
        force_tick_due(&mut app);

        let cmd = ftui::Model::update(&mut app, Msg::Tick);
        assert!(cmd_contains_task(&cmd));
    }

    #[test]
    fn tick_queues_async_poll_for_background_workspace_statuses_only() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("background-status-only");
        let mut app = GroveApp::from_parts(
            fixture_bootstrap(WorkspaceStatus::Idle),
            Box::new(BackgroundOnlyTmuxInput),
            AppPaths::new(
                sidebar_ratio_path,
                unique_config_path("background-status-only"),
            ),
            MultiplexerKind::Tmux,
            Box::new(NullEventLogger),
            None,
        );
        app.state.selected_index = 0;
        force_tick_due(&mut app);

        let cmd = ftui::Model::update(&mut app, Msg::Tick);
        assert!(!cmd_contains_task(&cmd));
    }

    #[test]
    fn async_preview_capture_failure_sets_toast_message() {
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
                workspace_status_captures: Vec::new(),
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
                workspace_status_captures: Vec::new(),
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
                workspace_status_captures: Vec::new(),
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
                workspace_status_captures: Vec::new(),
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
                workspace_status_captures: Vec::new(),
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
    fn preview_poll_waiting_prompt_sets_waiting_status() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        if let Some(workspace) = app.state.selected_workspace_mut() {
            workspace.status = WorkspaceStatus::Active;
        }

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: true,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Ok("Approve command? [y/n]".to_string()),
                }),
                cursor_capture: None,
                workspace_status_captures: Vec::new(),
            }),
        );

        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Waiting)
        );
    }

    #[test]
    fn preview_poll_updates_non_selected_workspace_status_from_background_capture() {
        let mut app = fixture_app();
        app.state.selected_index = 0;

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: None,
                cursor_capture: None,
                workspace_status_captures: vec![WorkspaceStatusCapture {
                    workspace_name: "feature-a".to_string(),
                    workspace_path: PathBuf::from("/repos/grove-feature-a"),
                    session_name: "grove-ws-feature-a".to_string(),
                    supported_agent: true,
                    capture_ms: 1,
                    result: Ok("> Implement {feature}\n? for shortcuts\n".to_string()),
                }],
            }),
        );

        assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Waiting);
        assert!(!app.state.workspaces[1].is_orphaned);
    }

    #[test]
    fn zellij_workspace_status_poll_targets_include_idle_workspaces() {
        let mut app = fixture_app();
        app.multiplexer = MultiplexerKind::Zellij;
        app.state.selected_index = 0;
        app.state.workspaces[1].status = WorkspaceStatus::Idle;

        let targets = app.workspace_status_poll_targets(None);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].workspace_name, "feature-a");
        assert_eq!(targets[0].session_name, "grove-ws-feature-a");
    }

    #[test]
    fn tmux_workspace_status_poll_targets_skip_idle_workspaces() {
        let mut app = fixture_app();
        app.multiplexer = MultiplexerKind::Tmux;
        app.state.selected_index = 0;
        app.state.workspaces[1].status = WorkspaceStatus::Idle;

        let targets = app.workspace_status_poll_targets(None);
        assert!(targets.is_empty());
    }

    #[test]
    fn preview_poll_non_selected_missing_session_marks_orphaned_idle() {
        let mut app = fixture_app();
        app.state.selected_index = 0;
        app.state.workspaces[1].status = WorkspaceStatus::Active;
        app.state.workspaces[1].is_orphaned = false;

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: None,
                cursor_capture: None,
                workspace_status_captures: vec![WorkspaceStatusCapture {
                    workspace_name: "feature-a".to_string(),
                    workspace_path: PathBuf::from("/repos/grove-feature-a"),
                    session_name: "grove-ws-feature-a".to_string(),
                    supported_agent: true,
                    capture_ms: 1,
                    result: Err(
                        "tmux capture-pane failed for 'grove-ws-feature-a': can't find pane"
                            .to_string(),
                    ),
                }],
            }),
        );

        assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Idle);
        assert!(app.state.workspaces[1].is_orphaned);
    }

    #[test]
    fn preview_poll_missing_session_marks_workspace_orphaned_idle() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        app.interactive = Some(InteractiveState::new(
            "%1".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            20,
            80,
        ));
        if let Some(workspace) = app.state.selected_workspace_mut() {
            workspace.status = WorkspaceStatus::Active;
            workspace.is_orphaned = false;
        }

        ftui::Model::update(
            &mut app,
            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation: 1,
                live_capture: Some(LivePreviewCapture {
                    session: "grove-ws-feature-a".to_string(),
                    include_escape_sequences: true,
                    capture_ms: 1,
                    total_ms: 1,
                    result: Err(
                        "tmux capture-pane failed for 'grove-ws-feature-a': can't find pane"
                            .to_string(),
                    ),
                }),
                cursor_capture: None,
                workspace_status_captures: Vec::new(),
            }),
        );

        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Idle)
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.is_orphaned),
            Some(true)
        );
        assert!(app.interactive.is_none());
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
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
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
            Some("new")
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
    fn project_add_dialog_accepts_shift_modified_uppercase_path_characters() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('A'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('/')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('U'))
                    .with_modifiers(Modifiers::SHIFT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('S'))
                    .with_modifiers(Modifiers::SHIFT)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert_eq!(
            app.project_dialog
                .as_ref()
                .and_then(|dialog| dialog.add_dialog.as_ref())
                .map(|dialog| dialog.path.clone()),
            Some("/US".to_string())
        );
    }

    #[test]
    fn project_dialog_filter_accepts_shift_modified_characters() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('G'))
                    .with_modifiers(Modifiers::SHIFT)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert_eq!(
            app.project_dialog
                .as_ref()
                .map(|dialog| dialog.filter.clone()),
            Some("G".to_string())
        );
    }

    #[test]
    fn project_dialog_j_and_k_are_treated_as_filter_input() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.project_dialog
                .as_ref()
                .map(|dialog| dialog.filter.clone()),
            Some("jk".to_string())
        );
    }

    #[test]
    fn project_dialog_tab_and_shift_tab_navigate_selection() {
        let mut app = fixture_app();
        app.projects.push(ProjectConfig {
            name: "site".to_string(),
            path: PathBuf::from("/repos/site"),
        });

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.project_dialog
                .as_ref()
                .map(|dialog| dialog.selected_filtered_index),
            Some(0)
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(
            app.project_dialog
                .as_ref()
                .map(|dialog| dialog.selected_filtered_index),
            Some(1)
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::BackTab).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(
            app.project_dialog
                .as_ref()
                .map(|dialog| dialog.selected_filtered_index),
            Some(0)
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
    fn ctrl_c_dismisses_delete_modal_via_action_mapper() {
        let mut app = fixture_app();
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
        );
        assert!(app.delete_dialog.is_some());

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(app.delete_dialog.is_none());
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
    fn h_and_l_switch_focus_between_workspace_and_preview_when_not_interactive() {
        let mut app = fixture_app();
        app.state.mode = UiMode::List;
        app.state.focus = PaneFocus::WorkspaceList;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.mode, UiMode::Preview);
        assert_eq!(app.state.focus, PaneFocus::Preview);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.mode, UiMode::List);
        assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
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
                workspace_path: PathBuf::from("/repos/grove-feature-a"),
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
    fn start_dialog_pre_launch_command_runs_before_agent() {
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

        for character in ['d', 'i', 'r', 'e', 'n', 'v', ' ', 'a', 'l', 'l', 'o', 'w'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }

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
                "direnv allow && codex".to_string(),
                "Enter".to_string(),
            ])
        );
    }

    #[test]
    fn start_dialog_field_navigation_can_toggle_unsafe_for_launch() {
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
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press)),
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
    fn edit_workspace_key_opens_edit_dialog() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
        );

        let Some(dialog) = app.edit_dialog.as_ref() else {
            panic!("edit dialog should be open");
        };
        assert_eq!(dialog.workspace_name, "grove");
        assert_eq!(dialog.branch, "main");
        assert_eq!(dialog.agent, AgentType::Claude);
    }

    #[test]
    fn edit_dialog_save_updates_workspace_agent_and_marker() {
        let mut app = fixture_app();
        let workspace_dir = unique_temp_workspace_dir("edit-save");
        app.state.workspaces[0].path = workspace_dir.clone();
        app.state.workspaces[0].agent = AgentType::Claude;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.edit_dialog.is_none());
        assert_eq!(app.state.workspaces[0].agent, AgentType::Codex);
        assert_eq!(
            fs::read_to_string(workspace_dir.join(".grove-agent"))
                .expect("agent marker should be readable")
                .trim(),
            "codex"
        );
        assert!(app.status_bar_line().contains("workspace updated"));
    }

    #[test]
    fn delete_key_opens_delete_dialog_for_selected_workspace() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
        );

        let Some(dialog) = app.delete_dialog.as_ref() else {
            panic!("delete dialog should be open");
        };
        assert_eq!(dialog.workspace_name, "feature-a");
        assert_eq!(dialog.branch, "feature-a");
        assert_eq!(dialog.focused_field, DeleteDialogField::DeleteLocalBranch);
    }

    #[test]
    fn delete_key_on_main_workspace_shows_guard_toast() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
        );

        assert!(app.delete_dialog.is_none());
        assert!(
            app.status_bar_line()
                .contains("cannot delete base workspace")
        );
    }

    #[test]
    fn delete_dialog_blocks_navigation_and_escape_cancels() {
        let mut app = fixture_app();
        app.state.selected_index = 1;
        app.open_delete_dialog();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.selected_index, 1);
        assert_eq!(
            app.delete_dialog
                .as_ref()
                .map(|dialog| dialog.focused_field),
            Some(DeleteDialogField::DeleteButton)
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        assert!(app.delete_dialog.is_none());
    }

    #[test]
    fn delete_dialog_confirm_queues_background_task() {
        let mut app = fixture_background_app(WorkspaceStatus::Idle);
        app.state.selected_index = 1;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
        );
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
        );

        assert!(cmd_contains_task(&cmd));
        assert!(app.delete_dialog.is_none());
        assert!(app.delete_in_flight);
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
            Some(CreateDialogField::Project)
        );
    }

    #[test]
    fn create_dialog_j_and_k_on_agent_field_toggle_agent() {
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
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Codex)
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Claude)
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
    fn create_dialog_ctrl_n_and_ctrl_p_toggle_agent() {
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
            Msg::Key(
                KeyEvent::new(KeyCode::Char('n'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Codex)
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('p'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert_eq!(
            app.create_dialog.as_ref().map(|dialog| dialog.agent),
            Some(AgentType::Claude)
        );
    }

    #[test]
    fn create_dialog_ctrl_n_and_ctrl_p_move_base_branch_dropdown() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        app.create_branch_all = vec![
            "main".to_string(),
            "develop".to_string(),
            "release".to_string(),
        ];
        if let Some(dialog) = app.create_dialog.as_mut() {
            dialog.base_branch.clear();
        }
        app.refresh_create_branch_filtered();

        for _ in 0..2 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('n'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert_eq!(app.create_branch_index, 1);
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('p'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        assert_eq!(app.create_branch_index, 0);
    }

    #[test]
    fn create_dialog_base_branch_dropdown_selects_with_enter() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );

        app.create_branch_all = vec![
            "main".to_string(),
            "develop".to_string(),
            "release".to_string(),
        ];
        app.refresh_create_branch_filtered();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
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
        for character in ['d', 'e'] {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.base_branch.clone()),
            Some("develop".to_string())
        );
        assert_eq!(
            app.create_dialog
                .as_ref()
                .map(|dialog| dialog.focused_field),
            Some(CreateDialogField::Agent)
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
    fn create_dialog_enter_without_name_shows_validation_toast() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        for _ in 0..4 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.create_dialog.is_some());
        assert!(app.status_bar_line().contains("workspace name is required"));
    }

    #[test]
    fn create_dialog_enter_on_cancel_closes_modal() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
        );
        for _ in 0..5 {
            ftui::Model::update(
                &mut app,
                Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
            );
        }
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.create_dialog.is_none());
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
                workspace_path: PathBuf::from("/repos/grove-feature-a"),
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
    fn start_key_opens_dialog_for_main_workspace() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
        assert!(app.launch_dialog.is_some());
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Main)
        );
    }

    #[test]
    fn start_key_on_running_workspace_shows_toast_and_no_dialog() {
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
    fn stop_key_without_running_agent_shows_toast() {
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
    fn stop_key_on_active_main_workspace_stops_agent() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
        app.state.workspaces[0].status = WorkspaceStatus::Active;

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
                    "grove-ws-grove".to_string(),
                    "C-c".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-grove".to_string(),
                ],
            ]
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Main)
        );
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
    fn enter_on_active_main_workspace_starts_interactive_mode() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
        app.state.workspaces[0].status = WorkspaceStatus::Active;

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_some());
        assert_eq!(
            app.interactive
                .as_ref()
                .map(|state| state.target_session.as_str()),
            Some("grove-ws-grove")
        );
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
                workspace_status_captures: Vec::new(),
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
                "paste-buffer:grove-ws-feature-a:14".to_string(),
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
        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        let second_row_y = sidebar_inner
            .y
            .saturating_add(1)
            .saturating_add(WORKSPACE_ITEM_HEIGHT);

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
                sidebar_inner.x.saturating_add(1),
                second_row_y,
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
    fn interactive_mouse_drag_logs_click_mapping_and_selected_preview() {
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
        app.preview.lines = vec!["alpha beta".to_string()];
        app.preview.render_lines = app.preview.lines.clone();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        clear_recorded_events(&events);

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

        let recorded = recorded_events(&events);
        let started = recorded
            .iter()
            .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
            .expect("drag start event should be logged");
        assert_eq!(started.data.get("mapped"), Some(&Value::from(true)));
        assert_eq!(started.data.get("line"), Some(&Value::from(0)));
        assert_eq!(started.data.get("col"), Some(&Value::from(0)));
        assert_eq!(
            started.data.get("line_clean_preview"),
            Some(&Value::from("alpha beta"))
        );
        assert_eq!(started.data.get("grapheme"), Some(&Value::from("a")));

        let finished = recorded
            .iter()
            .find(|event| event.event == "selection" && event.kind == "preview_drag_finished")
            .expect("drag finish event should be logged");
        assert_eq!(finished.data.get("has_selection"), Some(&Value::from(true)));
        assert_eq!(finished.data.get("start_line"), Some(&Value::from(0)));
        assert_eq!(finished.data.get("start_col"), Some(&Value::from(0)));
        assert_eq!(finished.data.get("end_line"), Some(&Value::from(0)));
        assert_eq!(finished.data.get("end_col"), Some(&Value::from(4)));
        assert_eq!(
            finished.data.get("selected_preview"),
            Some(&Value::from("alpha"))
        );
        assert_eq!(
            finished.data.get("release_grapheme"),
            Some(&Value::from("a"))
        );
        assert_eq!(finished.data.get("end_grapheme"), Some(&Value::from("a")));

        let mouse_event = recorded
            .iter()
            .find(|event| {
                event.event == "mouse"
                    && event.kind == "event"
                    && event.data.get("kind") == Some(&Value::from("Down(Left)"))
            })
            .expect("mouse event telemetry should be logged");
        assert_eq!(
            mouse_event.data.get("region"),
            Some(&Value::from("preview"))
        );
        assert_eq!(mouse_event.data.get("mapped_line"), Some(&Value::from(0)));
        assert_eq!(mouse_event.data.get("mapped_col"), Some(&Value::from(0)));
        assert_eq!(
            mouse_event.data.get("mapped_grapheme"),
            Some(&Value::from("a"))
        );
    }

    #[test]
    fn interactive_drag_mapping_prefers_render_line_when_clean_line_empty() {
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
        app.preview.lines = vec![String::new()];
        app.preview.render_lines = vec!["hello".to_string()];

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        clear_recorded_events(&events);

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

        let recorded = recorded_events(&events);
        let started = recorded
            .iter()
            .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
            .expect("drag start event should be logged");
        assert_eq!(
            started.data.get("line_preview"),
            Some(&Value::from("hello"))
        );
        assert_eq!(
            started.data.get("line_clean_preview"),
            Some(&Value::from("hello"))
        );
    }

    #[test]
    fn interactive_drag_mapping_uses_rendered_frame_size_without_resize_message() {
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
        app.preview.lines = (0..120).map(|index| format!("line-{index:03}")).collect();
        app.preview.render_lines = app.preview.lines.clone();

        with_rendered_frame(&app, 100, 50, |_| {});
        clear_recorded_events(&events);

        let layout = GroveApp::view_layout_for_size(100, 50, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let select_x = preview_inner.x;
        let select_y = preview_inner
            .y
            .saturating_add(PREVIEW_METADATA_ROWS)
            .saturating_add(40);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                select_x,
                select_y,
            )),
        );

        let recorded = recorded_events(&events);
        let started = recorded
            .iter()
            .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
            .expect("drag start event should be logged");
        assert_eq!(started.data.get("mapped"), Some(&Value::from(true)));

        let output_height = usize::from(preview_inner.height.saturating_sub(PREVIEW_METADATA_ROWS));
        let output_row = usize::from(
            select_y.saturating_sub(preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS)),
        );
        let expected_visible_start = app.preview.lines.len().saturating_sub(output_height);
        let expected_line = expected_visible_start.saturating_add(output_row);
        assert_eq!(
            started.data.get("line"),
            Some(&Value::from(
                u64::try_from(expected_line).unwrap_or(u64::MAX)
            ))
        );
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
    fn selected_preview_text_lines_use_visual_columns() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.preview.lines = vec!["A😀B".to_string()];
        app.preview.render_lines = app.preview.lines.clone();
        app.preview_selection
            .prepare_drag(TextSelectionPoint { line: 0, col: 0 });
        app.preview_selection
            .handle_drag(TextSelectionPoint { line: 0, col: 2 });
        app.preview_selection.finish_drag();

        assert_eq!(
            app.selected_preview_text_lines(),
            Some(vec!["A😀".to_string()])
        );
    }

    #[test]
    fn preview_render_lines_align_with_plain_visible_range_when_lengths_differ() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.preview.lines = (0..40).map(|index| format!("p{index}")).collect();
        app.preview.render_lines = (0..42).map(|index| format!("r{index}")).collect();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );

        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);
        let x_start = layout.preview.x.saturating_add(1);
        let x_end = layout.preview.right().saturating_sub(1);
        with_rendered_frame(&app, 100, 40, |frame| {
            let rendered = row_text(frame, output_y, x_start, x_end);
            assert!(
                rendered.contains("r6"),
                "expected first visible rendered row to start from aligned render index, got: {rendered}"
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
        assert!(content.contains("feature-a | feature-a | Codex | /repos/grove-feature-a"));
        assert!(content.contains("Press 'n' to create a workspace"));
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
            AppPaths::new(sidebar_ratio_path, unique_config_path("error-state")),
            MultiplexerKind::Tmux,
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
    fn preview_mode_bracket_keys_cycle_tabs() {
        let mut app = fixture_app();
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
        assert_eq!(app.preview_tab, PreviewTab::Agent);

        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
        assert_eq!(app.preview_tab, PreviewTab::Git);

        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
        assert_eq!(app.preview_tab, PreviewTab::Agent);

        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('['))));
        assert_eq!(app.preview_tab, PreviewTab::Git);
    }

    #[test]
    fn preview_mode_scroll_keys_noop_in_git_tab() {
        let mut app = fixture_app();
        app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
        app.preview.render_lines = app.preview.lines.clone();
        app.preview.offset = 0;
        app.preview.auto_scroll = true;
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
        assert_eq!(app.preview_tab, PreviewTab::Git);

        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::PageDown)));
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('G'))));

        assert_eq!(app.preview.offset, 0);
        assert!(app.preview.auto_scroll);
    }

    #[test]
    fn git_tab_renders_git_details() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);
        let x_start = preview_inner.x;
        let x_end = preview_inner.right();

        with_rendered_frame(&app, 100, 40, |frame| {
            let tabs_line = row_text(frame, preview_inner.y.saturating_add(1), x_start, x_end);
            let output_line = row_text(frame, output_y, x_start, x_end);

            assert!(tabs_line.contains("Agent"));
            assert!(tabs_line.contains("Git"));
            assert!(output_line.contains("Branch:"));
        });
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
            AppPaths::new(sidebar_ratio_path, unique_config_path("frame-log")),
            MultiplexerKind::Tmux,
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
            AppPaths::new(sidebar_ratio_path, unique_config_path("frame-lines")),
            MultiplexerKind::Tmux,
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
        assert_eq!(
            frame_event
                .data
                .get("frame_cursor_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            frame_event
                .data
                .get("frame_cursor_has_position")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn frame_debug_record_includes_interactive_cursor_snapshot() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("frame-cursor-snapshot");
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
            AppPaths::new(
                sidebar_ratio_path,
                unique_config_path("frame-cursor-snapshot"),
            ),
            MultiplexerKind::Tmux,
            Box::new(event_log),
            Some(1_771_023_000_124),
        );
        app.interactive = Some(InteractiveState::new(
            "%1".to_string(),
            "grove-ws-feature-a".to_string(),
            Instant::now(),
            3,
            80,
        ));
        if let Some(state) = app.interactive.as_mut() {
            state.update_cursor(1, 2, true, 3, 80);
        }
        app.preview.lines = vec![
            "line-0".to_string(),
            "line-1".to_string(),
            "line-2".to_string(),
        ];
        app.preview.render_lines = app.preview.lines.clone();

        with_rendered_frame(&app, 80, 24, |_frame| {});

        let frame_event = recorded_events(&events)
            .into_iter()
            .find(|event| event.event == "frame" && event.kind == "rendered")
            .expect("frame event should be present");
        assert_eq!(
            frame_event
                .data
                .get("interactive_cursor_row")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            frame_event
                .data
                .get("interactive_cursor_col")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            frame_event
                .data
                .get("interactive_cursor_in_viewport")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            frame_event
                .data
                .get("interactive_cursor_visible_index")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            frame_event
                .data
                .get("interactive_cursor_target_col")
                .and_then(Value::as_u64),
            Some(2)
        );
    }
}
