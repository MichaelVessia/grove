/// Generates `next()` and `previous()` methods for a cyclic enum.
/// Variants are listed in order; last wraps to first and vice versa.
/// Requires the enum to derive `Copy` and `PartialEq`.
macro_rules! cyclic_field_nav {
    ($vis:vis $enum:ident { $($variant:ident),+ $(,)? }) => {
        impl $enum {
            const ALL: &[$enum] = &[$($enum::$variant),+];

            $vis fn next(self) -> Self {
                let index = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
                Self::ALL[(index + 1) % Self::ALL.len()]
            }

            $vis fn previous(self) -> Self {
                let index = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
                Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
            }
        }
    };
}

use std::time::Instant;

use ftui::PackedRgba;
use ftui::core::geometry::Rect;

pub(super) const WORKSPACE_LAUNCH_PROMPT_FILENAME: &str = ".grove/prompt";
pub(super) const WORKSPACE_SKIP_PERMISSIONS_FILENAME: &str = ".grove/skip_permissions";
pub(super) const HEADER_HEIGHT: u16 = 1;
pub(super) const STATUS_HEIGHT: u16 = 1;
pub(super) const DIVIDER_WIDTH: u16 = 1;
pub(super) const WORKSPACE_ITEM_HEIGHT: u16 = 3;
pub(super) const PREVIEW_METADATA_ROWS: u16 = 2;
pub(super) const TICK_EARLY_TOLERANCE_MS: u64 = 5;
pub(super) const HIT_ID_HEADER: u32 = 1;
pub(super) const HIT_ID_WORKSPACE_LIST: u32 = 2;
pub(super) const HIT_ID_PREVIEW: u32 = 3;
pub(super) const HIT_ID_DIVIDER: u32 = 4;
pub(super) const HIT_ID_STATUS: u32 = 5;
pub(super) const HIT_ID_WORKSPACE_ROW: u32 = 6;
pub(super) const HIT_ID_CREATE_DIALOG: u32 = 7;
pub(super) const HIT_ID_LAUNCH_DIALOG: u32 = 8;
pub(super) const HIT_ID_DELETE_DIALOG: u32 = 9;
pub(super) const HIT_ID_KEYBIND_HELP_DIALOG: u32 = 10;
pub(super) const HIT_ID_SETTINGS_DIALOG: u32 = 11;
pub(super) const HIT_ID_PROJECT_DIALOG: u32 = 12;
pub(super) const HIT_ID_PROJECT_ADD_DIALOG: u32 = 13;
pub(super) const HIT_ID_EDIT_DIALOG: u32 = 14;
pub(super) const HIT_ID_MERGE_DIALOG: u32 = 15;
pub(super) const HIT_ID_UPDATE_FROM_BASE_DIALOG: u32 = 16;
pub(super) const HIT_ID_PROJECT_DEFAULTS_DIALOG: u32 = 17;
pub(super) const HIT_ID_STOP_DIALOG: u32 = 18;
pub(super) const HIT_ID_CONFIRM_DIALOG: u32 = 19;
pub(super) const HIT_ID_WORKSPACE_PR_LINK: u32 = 20;
pub(super) const MAX_PENDING_INPUT_TRACES: usize = 256;
pub(super) const INTERACTIVE_KEYSTROKE_DEBOUNCE_MS: u64 = 20;
pub(super) const FAST_ANIMATION_INTERVAL_MS: u64 = 100;
pub(super) const TOAST_TICK_INTERVAL_MS: u64 = 100;
pub(super) const PREVIEW_POLL_IN_FLIGHT_TICK_MS: u64 = 20;
pub(super) const LAZYGIT_COMMAND: &str = "lazygit";
pub(super) const AGENT_ACTIVITY_WINDOW_FRAMES: usize = 6;
pub(super) const LOCAL_TYPING_SUPPRESS_MS: u64 = 400;
pub(super) const SETUP_COMMAND_SEPARATOR: char = ';';
pub(super) const AGENT_ENV_SEPARATOR: char = ';';

pub(super) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(super) fn encode_workspace_pr_hit_data(
    workspace_index: usize,
    pull_request_index: usize,
) -> Option<u64> {
    let workspace = u32::try_from(workspace_index).ok()?;
    let pull_request = u32::try_from(pull_request_index).ok()?;
    Some((u64::from(workspace) << 32) | u64::from(pull_request))
}

pub(super) fn decode_workspace_pr_hit_data(data: u64) -> Option<(usize, usize)> {
    let workspace = usize::try_from(data >> 32).ok()?;
    let mask = u64::from(u32::MAX);
    let pull_request = usize::try_from(data & mask).ok()?;
    Some((workspace, pull_request))
}

pub(super) fn parse_setup_commands(raw: &str) -> Vec<String> {
    raw.split(SETUP_COMMAND_SEPARATOR)
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn format_setup_commands(commands: &[String]) -> String {
    commands
        .iter()
        .map(|command| command.trim())
        .filter(|command| !command.is_empty())
        .collect::<Vec<&str>>()
        .join("; ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AgentEnvVar {
    pub(super) key: String,
    pub(super) value: String,
}

pub(super) fn parse_agent_env_vars(raw: &str) -> Result<Vec<AgentEnvVar>, String> {
    let mut parsed = Vec::new();
    for segment in raw
        .split([AGENT_ENV_SEPARATOR, '\n'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
    {
        let Some((raw_key, raw_value)) = segment.split_once('=') else {
            return Err(format!("'{segment}' must be KEY=VALUE"));
        };
        let key = raw_key.trim();
        if !env_var_key_is_valid(key) {
            return Err(format!("invalid env key '{key}'"));
        }
        let value = raw_value.trim();
        if value.is_empty() {
            return Err(format!("env '{key}' cannot have empty value"));
        }
        parsed.push(AgentEnvVar {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(parsed)
}

pub(super) fn parse_agent_env_vars_from_entries(
    entries: &[String],
) -> Result<Vec<AgentEnvVar>, String> {
    parse_agent_env_vars(entries.join("; ").as_str())
}

pub(super) fn format_agent_env_vars(entries: &[String]) -> String {
    entries
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<&str>>()
        .join("; ")
}

pub(super) fn encode_agent_env_vars(raw: &str) -> Result<Vec<String>, String> {
    parse_agent_env_vars(raw).map(|vars| {
        vars.into_iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect()
    })
}

fn env_var_key_is_valid(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UiTheme {
    pub(super) base: PackedRgba,
    pub(super) mantle: PackedRgba,
    pub(super) crust: PackedRgba,
    pub(super) surface0: PackedRgba,
    pub(super) surface1: PackedRgba,
    pub(super) overlay0: PackedRgba,
    pub(super) text: PackedRgba,
    pub(super) subtext0: PackedRgba,
    pub(super) blue: PackedRgba,
    pub(super) lavender: PackedRgba,
    pub(super) yellow: PackedRgba,
    pub(super) red: PackedRgba,
    pub(super) peach: PackedRgba,
    pub(super) mauve: PackedRgba,
    pub(super) teal: PackedRgba,
}

pub(super) fn ui_theme() -> UiTheme {
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
pub(super) enum HitRegion {
    WorkspaceList,
    WorkspacePullRequest,
    Preview,
    Divider,
    StatusLine,
    Header,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum PreviewTab {
    #[default]
    Agent,
    Shell,
    Git,
}

impl PreviewTab {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Agent => "Agent",
            Self::Shell => "Shell",
            Self::Git => "Git",
        }
    }

    pub(super) const fn next(self) -> Self {
        match self {
            Self::Agent => Self::Shell,
            Self::Shell => Self::Git,
            Self::Git => Self::Agent,
        }
    }

    pub(super) const fn previous(self) -> Self {
        match self {
            Self::Agent => Self::Git,
            Self::Shell => Self::Agent,
            Self::Git => Self::Shell,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewLayout {
    pub(super) header: Rect,
    pub(super) sidebar: Rect,
    pub(super) divider: Rect,
    pub(super) preview: Rect,
    pub(super) status: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CursorMetadata {
    pub(super) cursor_visible: bool,
    pub(super) cursor_col: u16,
    pub(super) cursor_row: u16,
    pub(super) pane_width: u16,
    pub(super) pane_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PreviewContentViewport {
    pub(super) output_x: u16,
    pub(super) output_y: u16,
    pub(super) visible_start: usize,
    pub(super) visible_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InputTraceContext {
    pub(super) seq: u64,
    pub(super) received_at: Instant,
}

#[derive(Debug, Clone)]
pub(super) struct PendingInteractiveInput {
    pub(super) seq: u64,
    pub(super) session: String,
    pub(super) received_at: Instant,
    pub(super) forwarded_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingResizeVerification {
    pub(super) session: String,
    pub(super) expected_width: u16,
    pub(super) expected_height: u16,
    pub(super) retried: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QueuedInteractiveSend {
    pub(super) command: Vec<String>,
    pub(super) target_session: String,
    pub(super) action_kind: String,
    pub(super) trace_context: Option<InputTraceContext>,
    pub(super) literal_chars: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InteractiveSendCompletion {
    pub(super) send: QueuedInteractiveSend,
    pub(super) tmux_send_ms: u64,
    pub(super) error: Option<String>,
}
