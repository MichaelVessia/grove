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

use std::path::PathBuf;
use std::time::Instant;

use crate::domain::AgentType;
use crate::infrastructure::config::ThemeName;
use ftui::{Color, PackedRgba, ResolvedTheme, Theme, ThemeBuilder};

pub(super) const WORKSPACE_LAUNCH_PROMPT_FILENAME: &str = ".grove/prompt";
pub(super) const WORKSPACE_INIT_COMMAND_FILENAME: &str = ".grove/init_command";
pub(super) const WORKSPACE_SKIP_PERMISSIONS_FILENAME: &str = ".grove/skip_permissions";
pub(super) const HEADER_HEIGHT: u16 = 1;
pub(super) const STATUS_HEIGHT: u16 = 1;
pub(super) const DIVIDER_WIDTH: u16 = 1;
pub(super) const PREVIEW_METADATA_ROWS: u16 = 2;
pub(super) const TICK_EARLY_TOLERANCE_MS: u64 = 5;
pub(super) const FOCUS_ID_WORKSPACE_LIST: u64 = 1;
pub(super) const FOCUS_ID_PREVIEW: u64 = 2;
pub(super) const FOCUS_GROUP_MAIN_PANES: u32 = 1;
pub(super) const FOCUS_GROUP_CONFIRM_DIALOG: u32 = 10;
pub(super) const FOCUS_GROUP_PROJECT_DIALOG: u32 = 11;
pub(super) const FOCUS_GROUP_PROJECT_ADD_DIALOG: u32 = 12;
pub(super) const FOCUS_GROUP_STOP_DIALOG: u32 = 13;
pub(super) const FOCUS_GROUP_SESSION_CLEANUP_DIALOG: u32 = 14;
pub(super) const FOCUS_GROUP_DELETE_DIALOG: u32 = 15;
pub(super) const FOCUS_GROUP_MERGE_DIALOG: u32 = 16;
pub(super) const FOCUS_GROUP_UPDATE_FROM_BASE_DIALOG: u32 = 17;
pub(super) const FOCUS_GROUP_PULL_UPSTREAM_DIALOG: u32 = 18;
pub(super) const FOCUS_GROUP_SETTINGS_DIALOG: u32 = 19;
pub(super) const FOCUS_GROUP_RENAME_TAB_DIALOG: u32 = 20;
pub(super) const FOCUS_GROUP_EDIT_DIALOG: u32 = 21;
pub(super) const FOCUS_ID_CONFIRM_CONFIRM_BUTTON: u64 = 1001;
pub(super) const FOCUS_ID_CONFIRM_CANCEL_BUTTON: u64 = 1002;
pub(super) const FOCUS_ID_PROJECT_DIALOG_FILTER_INPUT: u64 = 1101;
pub(super) const FOCUS_ID_PROJECT_DIALOG_LIST: u64 = 1102;
pub(super) const FOCUS_ID_PROJECT_ADD_PATH_INPUT: u64 = 1201;
pub(super) const FOCUS_ID_PROJECT_ADD_NAME_INPUT: u64 = 1202;
pub(super) const FOCUS_ID_PROJECT_ADD_ADD_BUTTON: u64 = 1203;
pub(super) const FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON: u64 = 1204;
pub(super) const FOCUS_ID_STOP_CONFIRM_BUTTON: u64 = 1301;
pub(super) const FOCUS_ID_STOP_CANCEL_BUTTON: u64 = 1302;
pub(super) const FOCUS_ID_SESSION_CLEANUP_INCLUDE_STALE: u64 = 1401;
pub(super) const FOCUS_ID_SESSION_CLEANUP_INCLUDE_ATTACHED: u64 = 1402;
pub(super) const FOCUS_ID_SESSION_CLEANUP_APPLY_BUTTON: u64 = 1403;
pub(super) const FOCUS_ID_SESSION_CLEANUP_CANCEL_BUTTON: u64 = 1404;
pub(super) const FOCUS_ID_DELETE_LOCAL_BRANCH: u64 = 1501;
pub(super) const FOCUS_ID_DELETE_KILL_TMUX_SESSIONS: u64 = 1502;
pub(super) const FOCUS_ID_DELETE_CONFIRM_BUTTON: u64 = 1503;
pub(super) const FOCUS_ID_DELETE_CANCEL_BUTTON: u64 = 1504;
pub(super) const FOCUS_ID_MERGE_CLEANUP_WORKSPACE: u64 = 1601;
pub(super) const FOCUS_ID_MERGE_CLEANUP_LOCAL_BRANCH: u64 = 1602;
pub(super) const FOCUS_ID_MERGE_CONFIRM_BUTTON: u64 = 1603;
pub(super) const FOCUS_ID_MERGE_CANCEL_BUTTON: u64 = 1604;
pub(super) const FOCUS_ID_UPDATE_FROM_BASE_CONFIRM_BUTTON: u64 = 1701;
pub(super) const FOCUS_ID_UPDATE_FROM_BASE_CANCEL_BUTTON: u64 = 1702;
pub(super) const FOCUS_ID_PULL_UPSTREAM_CONFIRM_BUTTON: u64 = 1801;
pub(super) const FOCUS_ID_PULL_UPSTREAM_CANCEL_BUTTON: u64 = 1802;
pub(super) const FOCUS_ID_SETTINGS_THEME: u64 = 1901;
pub(super) const FOCUS_ID_SETTINGS_SAVE_BUTTON: u64 = 1902;
pub(super) const FOCUS_ID_SETTINGS_CANCEL_BUTTON: u64 = 1903;
pub(super) const FOCUS_ID_RENAME_TAB_TITLE: u64 = 2001;
pub(super) const FOCUS_ID_RENAME_TAB_RENAME_BUTTON: u64 = 2002;
pub(super) const FOCUS_ID_RENAME_TAB_CANCEL_BUTTON: u64 = 2003;
pub(super) const FOCUS_ID_EDIT_BASE_BRANCH: u64 = 2101;
pub(super) const FOCUS_ID_EDIT_SAVE_BUTTON: u64 = 2102;
pub(super) const FOCUS_ID_EDIT_CANCEL_BUTTON: u64 = 2103;
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
pub(super) const HIT_ID_CREATE_DIALOG_TAB: u32 = 21;
pub(super) const HIT_ID_SESSION_CLEANUP_DIALOG: u32 = 22;
pub(super) const HIT_ID_RENAME_TAB_DIALOG: u32 = 23;
pub(super) const HIT_ID_PROJECT_DIALOG_LIST: u32 = 24;
pub(super) const HIT_ID_PROJECT_ADD_RESULTS_LIST: u32 = 25;
pub(super) const HIT_ID_PULL_UPSTREAM_DIALOG: u32 = 26;
pub(super) const HIT_ID_PERFORMANCE_DIALOG: u32 = 27;
pub(super) const MAX_PENDING_INPUT_TRACES: usize = 256;
pub(super) const INTERACTIVE_KEYSTROKE_DEBOUNCE_MS: u64 = 20;
pub(super) const FAST_ANIMATION_INTERVAL_MS: u64 = 100;
pub(super) const TOAST_TICK_INTERVAL_MS: u64 = 100;
pub(super) const PREVIEW_POLL_IN_FLIGHT_TICK_MS: u64 = 20;
pub(super) const WORKSPACE_STATUS_POLL_INTERVAL_MS: u64 = 2_000;
pub(super) const LIVE_PREVIEW_SCROLLBACK_LINES: usize = 600;
pub(super) const LIVE_PREVIEW_IDLE_SCROLLBACK_LINES: usize = 200;
pub(super) const LIVE_PREVIEW_FULL_SCROLLBACK_LINES: usize = 0;
pub(super) const LAZYGIT_COMMAND: &str = "lazygit";
pub(super) const WORKING_STATUS_HOLD_MS: u64 = 3_000;
pub(super) const WORKING_IDLE_POLLS_TO_CLEAR: u8 = 2;
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

pub(super) fn encode_create_dialog_tab_hit_data(tab: crate::ui::tui::CreateDialogTab) -> u64 {
    match tab {
        crate::ui::tui::CreateDialogTab::Manual => 0,
        crate::ui::tui::CreateDialogTab::PullRequest => 1,
    }
}

pub(super) fn decode_create_dialog_tab_hit_data(
    data: u64,
) -> Option<crate::ui::tui::CreateDialogTab> {
    match data {
        0 => Some(crate::ui::tui::CreateDialogTab::Manual),
        1 => Some(crate::ui::tui::CreateDialogTab::PullRequest),
        _ => None,
    }
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
struct ThemePreset {
    name: ThemeName,
    display_name: &'static str,
}

const THEME_PRESETS: [ThemePreset; 8] = [
    ThemePreset {
        name: ThemeName::Monokai,
        display_name: "Monokai",
    },
    ThemePreset {
        name: ThemeName::CatppuccinLatte,
        display_name: "Catppuccin Latte",
    },
    ThemePreset {
        name: ThemeName::CatppuccinFrappe,
        display_name: "Catppuccin Frappe",
    },
    ThemePreset {
        name: ThemeName::CatppuccinMacchiato,
        display_name: "Catppuccin Macchiato",
    },
    ThemePreset {
        name: ThemeName::CatppuccinMocha,
        display_name: "Catppuccin Mocha",
    },
    ThemePreset {
        name: ThemeName::RosePine,
        display_name: "Rosé Pine",
    },
    ThemePreset {
        name: ThemeName::RosePineMoon,
        display_name: "Rosé Pine Moon",
    },
    ThemePreset {
        name: ThemeName::RosePineDawn,
        display_name: "Rosé Pine Dawn",
    },
];

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r, g, b)
}

fn build_theme(theme_name: ThemeName) -> Theme {
    match theme_name {
        ThemeName::Monokai => ThemeBuilder::new()
            .background(rgb(39, 40, 34))
            .surface(rgb(62, 61, 50))
            .overlay(rgb(92, 88, 76))
            .text(rgb(248, 248, 242))
            .text_muted(rgb(212, 208, 191))
            .text_subtle(rgb(162, 160, 142))
            .primary(rgb(102, 217, 239))
            .secondary(rgb(174, 129, 255))
            .accent(rgb(253, 151, 31))
            .info(rgb(17, 168, 205))
            .success(rgb(166, 226, 46))
            .warning(rgb(230, 219, 116))
            .error(rgb(249, 38, 114))
            .border(rgb(117, 113, 94))
            .border_focused(rgb(102, 217, 239))
            .selection_bg(rgb(73, 72, 62))
            .selection_fg(rgb(248, 248, 242))
            .scrollbar_track(rgb(39, 40, 34))
            .scrollbar_thumb(rgb(62, 61, 50))
            .build(),
        ThemeName::CatppuccinLatte => ThemeBuilder::new()
            .background(rgb(239, 241, 245))
            .surface(rgb(204, 208, 218))
            .overlay(rgb(172, 176, 190))
            .text(rgb(76, 79, 105))
            .text_muted(rgb(92, 95, 119))
            .text_subtle(rgb(108, 111, 133))
            .primary(rgb(30, 102, 245))
            .secondary(rgb(114, 135, 253))
            .accent(rgb(254, 100, 11))
            .info(rgb(23, 146, 153))
            .success(rgb(64, 160, 43))
            .warning(rgb(223, 142, 29))
            .error(rgb(210, 15, 57))
            .border(rgb(156, 160, 176))
            .border_focused(rgb(30, 102, 245))
            .selection_bg(rgb(188, 192, 204))
            .selection_fg(rgb(76, 79, 105))
            .scrollbar_track(rgb(239, 241, 245))
            .scrollbar_thumb(rgb(204, 208, 218))
            .build(),
        ThemeName::CatppuccinFrappe => ThemeBuilder::new()
            .background(rgb(48, 52, 70))
            .surface(rgb(65, 69, 89))
            .overlay(rgb(98, 104, 128))
            .text(rgb(198, 208, 245))
            .text_muted(rgb(181, 191, 226))
            .text_subtle(rgb(165, 173, 206))
            .primary(rgb(140, 170, 238))
            .secondary(rgb(186, 187, 241))
            .accent(rgb(239, 159, 118))
            .info(rgb(129, 200, 190))
            .success(rgb(166, 209, 137))
            .warning(rgb(229, 200, 144))
            .error(rgb(231, 130, 132))
            .border(rgb(115, 121, 148))
            .border_focused(rgb(140, 170, 238))
            .selection_bg(rgb(81, 87, 109))
            .selection_fg(rgb(198, 208, 245))
            .scrollbar_track(rgb(48, 52, 70))
            .scrollbar_thumb(rgb(65, 69, 89))
            .build(),
        ThemeName::CatppuccinMacchiato => ThemeBuilder::new()
            .background(rgb(36, 39, 58))
            .surface(rgb(54, 58, 79))
            .overlay(rgb(91, 96, 120))
            .text(rgb(202, 211, 245))
            .text_muted(rgb(184, 192, 224))
            .text_subtle(rgb(165, 173, 203))
            .primary(rgb(138, 173, 244))
            .secondary(rgb(183, 189, 248))
            .accent(rgb(245, 169, 127))
            .info(rgb(139, 213, 202))
            .success(rgb(166, 218, 149))
            .warning(rgb(238, 212, 159))
            .error(rgb(237, 135, 150))
            .border(rgb(110, 115, 141))
            .border_focused(rgb(138, 173, 244))
            .selection_bg(rgb(73, 77, 100))
            .selection_fg(rgb(202, 211, 245))
            .scrollbar_track(rgb(36, 39, 58))
            .scrollbar_thumb(rgb(54, 58, 79))
            .build(),
        ThemeName::CatppuccinMocha => ThemeBuilder::new()
            .background(rgb(30, 30, 46))
            .surface(rgb(49, 50, 68))
            .overlay(rgb(88, 91, 112))
            .text(rgb(205, 214, 244))
            .text_muted(rgb(186, 194, 222))
            .text_subtle(rgb(166, 173, 200))
            .primary(rgb(137, 180, 250))
            .secondary(rgb(180, 190, 254))
            .accent(rgb(250, 179, 135))
            .info(rgb(148, 226, 213))
            .success(rgb(166, 227, 161))
            .warning(rgb(249, 226, 175))
            .error(rgb(243, 139, 168))
            .border(rgb(108, 112, 134))
            .border_focused(rgb(137, 180, 250))
            .selection_bg(rgb(69, 71, 90))
            .selection_fg(rgb(205, 214, 244))
            .scrollbar_track(rgb(30, 30, 46))
            .scrollbar_thumb(rgb(49, 50, 68))
            .build(),
        ThemeName::RosePine => ThemeBuilder::new()
            .background(rgb(25, 23, 36))
            .surface(rgb(31, 29, 46))
            .overlay(rgb(64, 61, 82))
            .text(rgb(224, 222, 244))
            .text_muted(rgb(184, 181, 207))
            .text_subtle(rgb(144, 140, 170))
            .primary(rgb(156, 207, 216))
            .secondary(rgb(235, 188, 186))
            .accent(rgb(246, 193, 119))
            .info(rgb(49, 116, 143))
            .success(rgb(156, 207, 216))
            .warning(rgb(246, 193, 119))
            .error(rgb(235, 111, 146))
            .border(rgb(110, 106, 134))
            .border_focused(rgb(156, 207, 216))
            .selection_bg(rgb(38, 35, 58))
            .selection_fg(rgb(224, 222, 244))
            .scrollbar_track(rgb(25, 23, 36))
            .scrollbar_thumb(rgb(31, 29, 46))
            .build(),
        ThemeName::RosePineMoon => ThemeBuilder::new()
            .background(rgb(35, 33, 54))
            .surface(rgb(42, 39, 63))
            .overlay(rgb(68, 65, 90))
            .text(rgb(224, 222, 244))
            .text_muted(rgb(184, 181, 207))
            .text_subtle(rgb(144, 140, 170))
            .primary(rgb(156, 207, 216))
            .secondary(rgb(235, 188, 186))
            .accent(rgb(246, 193, 119))
            .info(rgb(62, 143, 176))
            .success(rgb(156, 207, 216))
            .warning(rgb(246, 193, 119))
            .error(rgb(235, 111, 146))
            .border(rgb(110, 106, 134))
            .border_focused(rgb(156, 207, 216))
            .selection_bg(rgb(57, 53, 82))
            .selection_fg(rgb(224, 222, 244))
            .scrollbar_track(rgb(35, 33, 54))
            .scrollbar_thumb(rgb(42, 39, 63))
            .build(),
        ThemeName::RosePineDawn => ThemeBuilder::new()
            .background(rgb(250, 244, 237))
            .surface(rgb(223, 218, 217))
            .overlay(rgb(188, 186, 193))
            .text(rgb(87, 82, 121))
            .text_muted(rgb(104, 100, 134))
            .text_subtle(rgb(121, 117, 147))
            .primary(rgb(86, 148, 159))
            .secondary(rgb(215, 130, 126))
            .accent(rgb(234, 157, 52))
            .info(rgb(40, 105, 131))
            .success(rgb(86, 148, 159))
            .warning(rgb(234, 157, 52))
            .error(rgb(180, 99, 122))
            .border(rgb(152, 147, 165))
            .border_focused(rgb(86, 148, 159))
            .selection_bg(rgb(206, 202, 205))
            .selection_fg(rgb(87, 82, 121))
            .scrollbar_track(rgb(250, 244, 237))
            .scrollbar_thumb(rgb(223, 218, 217))
            .build(),
    }
}

pub(crate) fn packed(color: Color) -> PackedRgba {
    let rgb = color.to_rgb();
    PackedRgba::rgb(rgb.r, rgb.g, rgb.b)
}

fn theme_preset(theme_name: ThemeName) -> ThemePreset {
    THEME_PRESETS
        .iter()
        .copied()
        .find(|preset| preset.name == theme_name)
        .unwrap_or(THEME_PRESETS[4])
}

pub(super) fn theme_display_name(theme_name: ThemeName) -> &'static str {
    theme_preset(theme_name).display_name
}

pub(super) fn next_theme_name(theme_name: ThemeName) -> ThemeName {
    let index = THEME_PRESETS
        .iter()
        .position(|preset| preset.name == theme_name)
        .unwrap_or(0);
    THEME_PRESETS[(index + 1) % THEME_PRESETS.len()].name
}

pub(super) fn previous_theme_name(theme_name: ThemeName) -> ThemeName {
    let index = THEME_PRESETS
        .iter()
        .position(|preset| preset.name == theme_name)
        .unwrap_or(0);
    THEME_PRESETS[(index + THEME_PRESETS.len() - 1) % THEME_PRESETS.len()].name
}

pub(crate) fn ui_theme_for(theme_name: ThemeName) -> ResolvedTheme {
    build_theme(theme_name).resolve(Theme::detect_dark_mode())
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn ui_theme() -> ResolvedTheme {
    ui_theme_for(ThemeName::default())
}

impl super::GroveApp {
    pub(super) fn active_ui_theme(&self) -> ResolvedTheme {
        ui_theme_for(self.theme_name)
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
    Home,
    #[default]
    Agent,
    Shell,
    Git,
    Diff,
}

impl PreviewTab {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Agent => "Agent",
            Self::Shell => "Shell",
            Self::Git => "Git",
            Self::Diff => "Diff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceTabKind {
    Home,
    Agent,
    Shell,
    Git,
    Diff,
}

impl WorkspaceTabKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Agent => "Agent",
            Self::Shell => "Shell",
            Self::Git => "Git",
            Self::Diff => "Diff",
        }
    }
}

impl From<WorkspaceTabKind> for PreviewTab {
    fn from(value: WorkspaceTabKind) -> Self {
        match value {
            WorkspaceTabKind::Home => PreviewTab::Home,
            WorkspaceTabKind::Agent => PreviewTab::Agent,
            WorkspaceTabKind::Shell => PreviewTab::Shell,
            WorkspaceTabKind::Git => PreviewTab::Git,
            WorkspaceTabKind::Diff => PreviewTab::Diff,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceTabRuntimeState {
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceTab {
    pub(super) id: u64,
    pub(super) display_order: u64,
    pub(super) kind: WorkspaceTabKind,
    pub(super) title: String,
    pub(super) session_name: Option<String>,
    pub(super) agent_type: Option<AgentType>,
    pub(super) state: WorkspaceTabRuntimeState,
}

impl WorkspaceTab {
    pub(super) fn home(id: u64) -> Self {
        Self {
            id,
            display_order: 0,
            kind: WorkspaceTabKind::Home,
            title: WorkspaceTabKind::Home.label().to_string(),
            session_name: None,
            agent_type: None,
            state: WorkspaceTabRuntimeState::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceTabsState {
    pub(super) tabs: Vec<WorkspaceTab>,
    pub(super) active_tab_id: u64,
    pub(super) next_seq: u64,
}

impl Default for WorkspaceTabsState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceTabsState {
    fn normalize_display_order(&mut self) {
        if let Some(home_index) = self
            .tabs
            .iter()
            .position(|tab| tab.kind == WorkspaceTabKind::Home)
            && home_index != 0
        {
            let home = self.tabs.remove(home_index);
            self.tabs.insert(0, home);
        }

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let Some(display_order) = u64::try_from(index).ok() else {
                continue;
            };
            tab.display_order = display_order;
        }
    }

    pub(super) fn new() -> Self {
        let home = WorkspaceTab::home(1);
        Self {
            tabs: vec![home],
            active_tab_id: 1,
            next_seq: 2,
        }
    }

    pub(super) fn activate_first_running_non_home_tab(&mut self) {
        if let Some(tab_id) = self
            .tabs
            .iter()
            .find(|tab| {
                tab.kind != WorkspaceTabKind::Home && tab.state == WorkspaceTabRuntimeState::Running
            })
            .map(|tab| tab.id)
        {
            self.active_tab_id = tab_id;
        }
    }

    pub(super) fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.tabs.iter().find(|tab| tab.id == self.active_tab_id)
    }

    pub(super) fn active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.id == self.active_tab_id)
    }

    pub(super) fn tab_by_id(&self, tab_id: u64) -> Option<&WorkspaceTab> {
        self.tabs.iter().find(|tab| tab.id == tab_id)
    }

    pub(super) fn tab_by_id_mut(&mut self, tab_id: u64) -> Option<&mut WorkspaceTab> {
        self.tabs.iter_mut().find(|tab| tab.id == tab_id)
    }

    pub(super) fn find_kind(&self, kind: WorkspaceTabKind) -> Option<&WorkspaceTab> {
        self.tabs.iter().find(|tab| tab.kind == kind)
    }

    pub(super) fn set_active(&mut self, tab_id: u64) -> bool {
        if self.tab_by_id(tab_id).is_none() {
            return false;
        }
        self.active_tab_id = tab_id;
        true
    }

    pub(super) fn active_index(&self) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.id == self.active_tab_id)
    }

    pub(super) fn insert_tab_adjacent(&mut self, mut tab: WorkspaceTab) -> u64 {
        let tab_id = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        tab.id = tab_id;
        let last_same_kind = self.tabs.iter().rposition(|entry| entry.kind == tab.kind);
        let insert_at = if let Some(index) = last_same_kind {
            index.saturating_add(1)
        } else {
            self.active_index()
                .map_or(self.tabs.len(), |index| index.saturating_add(1))
        };
        self.tabs.insert(insert_at, tab);
        self.normalize_display_order();
        self.active_tab_id = tab_id;
        tab_id
    }

    pub(super) fn insert_restored_tab(&mut self, tab: WorkspaceTab) -> bool {
        if tab.kind == WorkspaceTabKind::Home {
            return false;
        }
        if self.tab_by_id(tab.id).is_some() {
            return false;
        }
        if tab.kind == WorkspaceTabKind::Git && self.find_kind(WorkspaceTabKind::Git).is_some() {
            return false;
        }
        if let Some(ref session_name) = tab.session_name
            && self
                .tabs
                .iter()
                .any(|existing| existing.session_name.as_deref() == Some(session_name.as_str()))
        {
            return false;
        }

        self.next_seq = self.next_seq.max(tab.id.saturating_add(1));
        self.tabs.push(tab);
        self.tabs.sort_by_key(|entry| {
            (
                entry.kind != WorkspaceTabKind::Home,
                entry.display_order,
                entry.id,
            )
        });
        self.normalize_display_order();
        true
    }

    pub(super) fn next_tab_ordinal(&self, kind: WorkspaceTabKind) -> u64 {
        let suffix = match kind {
            WorkspaceTabKind::Agent => "-agent-",
            WorkspaceTabKind::Shell => "-shell-",
            _ => {
                return 1;
            }
        };
        let max_ordinal = self
            .tabs
            .iter()
            .filter(|tab| tab.kind == kind)
            .filter_map(|tab| tab.session_name.as_deref())
            .filter_map(|name| name.rsplit_once(suffix))
            .filter_map(|(_, ordinal_str)| ordinal_str.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        max_ordinal.saturating_add(1)
    }

    pub(super) fn close_tab(&mut self, tab_id: u64) -> Option<WorkspaceTab> {
        let index = self.tabs.iter().position(|tab| tab.id == tab_id)?;
        if self.tabs[index].kind == WorkspaceTabKind::Home {
            return None;
        }
        let removed = self.tabs.remove(index);
        self.normalize_display_order();
        if self.active_tab_id == tab_id {
            let fallback_index = index
                .saturating_sub(1)
                .min(self.tabs.len().saturating_sub(1));
            if let Some(fallback) = self.tabs.get(fallback_index) {
                self.active_tab_id = fallback.id;
            }
        }
        Some(removed)
    }

    pub(super) fn ensure_home_tab(&mut self) {
        if self
            .tabs
            .iter()
            .any(|tab| tab.kind == WorkspaceTabKind::Home)
        {
            if self.tab_by_id(self.active_tab_id).is_none()
                && let Some(home) = self
                    .tabs
                    .iter()
                    .find(|tab| tab.kind == WorkspaceTabKind::Home)
            {
                self.active_tab_id = home.id;
            }
            self.normalize_display_order();
            return;
        }
        let home = WorkspaceTab::home(self.next_seq);
        self.next_seq = self.next_seq.saturating_add(1);
        self.tabs.insert(0, home.clone());
        self.active_tab_id = home.id;
        self.normalize_display_order();
    }

    pub(super) fn set_home_title(&mut self, title: &str) {
        if let Some(home) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.kind == WorkspaceTabKind::Home)
        {
            home.title = title.to_string();
        }
    }

    pub(super) fn move_active_tab_by(&mut self, direction: i8) -> bool {
        if direction == 0 {
            return false;
        }
        let Some(active_index) = self.active_index() else {
            return false;
        };
        let Some(active_tab) = self.tabs.get(active_index) else {
            return false;
        };
        if active_tab.kind == WorkspaceTabKind::Home {
            return false;
        }

        let target_index = if direction.is_negative() {
            if active_index <= 1 {
                return false;
            }
            active_index.saturating_sub(1)
        } else {
            let next_index = active_index.saturating_add(1);
            if next_index >= self.tabs.len() {
                return false;
            }
            next_index
        };

        if self
            .tabs
            .get(target_index)
            .is_some_and(|tab| tab.kind == WorkspaceTabKind::Home)
        {
            return false;
        }

        self.tabs.swap(active_index, target_index);
        self.normalize_display_order();
        true
    }
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
    pub(super) suppresses_agent_activity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingResizeVerification {
    pub(super) session: String,
    pub(super) expected_width: u16,
    pub(super) expected_height: u16,
    pub(super) retried: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreviewSessionGeometry {
    pub(super) session: String,
    pub(super) width: u16,
    pub(super) height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QueuedInteractiveSend {
    pub(super) command: Vec<String>,
    pub(super) target_session: String,
    pub(super) attention_ack_workspace_path: Option<PathBuf>,
    pub(super) action_kind: String,
    pub(super) trace_context: Option<InputTraceContext>,
    pub(super) literal_chars: Option<u64>,
    pub(super) suppresses_agent_activity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InteractiveSendCompletion {
    pub(super) send: QueuedInteractiveSend,
    pub(super) tmux_send_ms: u64,
    pub(super) error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_tab(session_name: &str) -> WorkspaceTab {
        WorkspaceTab {
            id: 0,
            display_order: 0,
            kind: WorkspaceTabKind::Agent,
            title: String::new(),
            session_name: Some(session_name.to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        }
    }

    fn shell_tab(session_name: &str) -> WorkspaceTab {
        WorkspaceTab {
            id: 0,
            display_order: 0,
            kind: WorkspaceTabKind::Shell,
            title: String::new(),
            session_name: Some(session_name.to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        }
    }

    fn git_tab(session_name: &str) -> WorkspaceTab {
        WorkspaceTab {
            id: 0,
            display_order: 0,
            kind: WorkspaceTabKind::Git,
            title: String::new(),
            session_name: Some(session_name.to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        }
    }

    fn tab_kinds(tabs: &WorkspaceTabsState) -> Vec<WorkspaceTabKind> {
        tabs.tabs.iter().map(|tab| tab.kind).collect()
    }

    #[test]
    fn next_tab_ordinal_returns_1_when_no_tabs_of_kind() {
        let tabs = WorkspaceTabsState::new();
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Agent), 1);
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Shell), 1);
    }

    #[test]
    fn next_tab_ordinal_uses_max_not_count() {
        let mut tabs = WorkspaceTabsState::new();
        // Insert agents 1 and 3 (simulating agent 2 was closed)
        tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        tabs.insert_tab_adjacent(agent_tab("ws-agent-3"));
        // With count-based logic this would return 3, colliding with agent-3.
        // With max-based logic it returns 4.
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Agent), 4);
    }

    #[test]
    fn next_tab_ordinal_sequential_agents() {
        let mut tabs = WorkspaceTabsState::new();
        tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        tabs.insert_tab_adjacent(agent_tab("ws-agent-2"));
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Agent), 3);
    }

    #[test]
    fn next_tab_ordinal_ignores_other_kinds() {
        let mut tabs = WorkspaceTabsState::new();
        tabs.insert_tab_adjacent(shell_tab("ws-shell-5"));
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Agent), 1);
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Shell), 6);
    }

    #[test]
    fn next_tab_ordinal_returns_1_for_home_and_git() {
        let tabs = WorkspaceTabsState::new();
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Home), 1);
        assert_eq!(tabs.next_tab_ordinal(WorkspaceTabKind::Git), 1);
    }

    #[test]
    fn activate_first_running_non_home_tab_selects_running_agent() {
        let mut tabs = WorkspaceTabsState::new();
        let agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        tabs.active_tab_id = 1; // reset to Home

        tabs.activate_first_running_non_home_tab();

        assert_eq!(tabs.active_tab_id, agent_id);
    }

    #[test]
    fn activate_first_running_non_home_tab_skips_stopped_tabs() {
        let mut tabs = WorkspaceTabsState::new();
        let agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        if let Some(tab) = tabs.tab_by_id_mut(agent_id) {
            tab.state = WorkspaceTabRuntimeState::Stopped;
        }
        let shell_id = tabs.insert_tab_adjacent(shell_tab("ws-shell-1"));
        tabs.active_tab_id = 1;

        tabs.activate_first_running_non_home_tab();

        assert_eq!(tabs.active_tab_id, shell_id);
    }

    #[test]
    fn activate_first_running_non_home_tab_keeps_home_when_no_running_tabs() {
        let mut tabs = WorkspaceTabsState::new();

        tabs.activate_first_running_non_home_tab();

        assert_eq!(tabs.active_tab_id, 1);
    }

    #[test]
    fn insert_restored_tab_rejects_duplicate_session_name() {
        let mut tabs = WorkspaceTabsState::new();
        let tab1 = WorkspaceTab {
            id: 10,
            display_order: 1,
            kind: WorkspaceTabKind::Agent,
            title: "Agent 1".to_string(),
            session_name: Some("ws-agent-1".to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        };
        assert!(tabs.insert_restored_tab(tab1));

        let tab2 = WorkspaceTab {
            id: 20,
            display_order: 2,
            kind: WorkspaceTabKind::Agent,
            title: "Agent 2".to_string(),
            session_name: Some("ws-agent-1".to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        };
        assert!(!tabs.insert_restored_tab(tab2));
    }

    #[test]
    fn move_active_tab_left_swaps_with_left_neighbor_and_keeps_active_tab() {
        let mut tabs = WorkspaceTabsState::new();
        let first_agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        let shell_id = tabs.insert_tab_adjacent(shell_tab("ws-shell-1"));
        let git_id = tabs.insert_tab_adjacent(git_tab("ws-git"));

        assert_eq!(
            tab_kinds(&tabs),
            vec![
                WorkspaceTabKind::Home,
                WorkspaceTabKind::Agent,
                WorkspaceTabKind::Shell,
                WorkspaceTabKind::Git,
            ]
        );
        assert_eq!(tabs.active_tab_id, git_id);

        assert!(tabs.move_active_tab_by(-1));
        assert_eq!(
            tab_kinds(&tabs),
            vec![
                WorkspaceTabKind::Home,
                WorkspaceTabKind::Agent,
                WorkspaceTabKind::Git,
                WorkspaceTabKind::Shell,
            ]
        );
        assert_eq!(tabs.active_tab_id, git_id);
        assert_eq!(
            tabs.active_tab().map(|tab| tab.kind),
            Some(WorkspaceTabKind::Git)
        );
        assert_ne!(tabs.active_tab_id, first_agent_id);
        assert_ne!(tabs.active_tab_id, shell_id);
    }

    #[test]
    fn move_active_tab_right_swaps_with_right_neighbor_and_keeps_active_tab() {
        let mut tabs = WorkspaceTabsState::new();
        let _first_agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        let shell_id = tabs.insert_tab_adjacent(shell_tab("ws-shell-1"));
        let _git_id = tabs.insert_tab_adjacent(git_tab("ws-git"));
        assert!(tabs.set_active(shell_id));

        assert!(tabs.move_active_tab_by(1));
        assert_eq!(
            tab_kinds(&tabs),
            vec![
                WorkspaceTabKind::Home,
                WorkspaceTabKind::Agent,
                WorkspaceTabKind::Git,
                WorkspaceTabKind::Shell,
            ]
        );
        assert_eq!(tabs.active_tab_id, shell_id);
        assert_eq!(
            tabs.active_tab().map(|tab| tab.kind),
            Some(WorkspaceTabKind::Shell)
        );
    }

    #[test]
    fn move_active_tab_left_is_noop_at_first_movable_slot() {
        let mut tabs = WorkspaceTabsState::new();
        let agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));

        assert_eq!(
            tab_kinds(&tabs),
            vec![WorkspaceTabKind::Home, WorkspaceTabKind::Agent]
        );
        assert!(!tabs.move_active_tab_by(-1));
        assert_eq!(
            tab_kinds(&tabs),
            vec![WorkspaceTabKind::Home, WorkspaceTabKind::Agent]
        );
        assert_eq!(tabs.active_tab_id, agent_id);
    }

    #[test]
    fn move_active_tab_right_is_noop_at_last_slot() {
        let mut tabs = WorkspaceTabsState::new();
        let _agent_id = tabs.insert_tab_adjacent(agent_tab("ws-agent-1"));
        let shell_id = tabs.insert_tab_adjacent(shell_tab("ws-shell-1"));

        assert!(!tabs.move_active_tab_by(1));
        assert_eq!(
            tab_kinds(&tabs),
            vec![
                WorkspaceTabKind::Home,
                WorkspaceTabKind::Agent,
                WorkspaceTabKind::Shell,
            ]
        );
        assert_eq!(tabs.active_tab_id, shell_id);
    }

    #[test]
    fn move_active_tab_by_rejects_home_tab() {
        let mut tabs = WorkspaceTabsState::new();

        assert!(!tabs.move_active_tab_by(-1));
        assert_eq!(tab_kinds(&tabs), vec![WorkspaceTabKind::Home]);
        assert_eq!(
            tabs.active_tab().map(|tab| tab.kind),
            Some(WorkspaceTabKind::Home)
        );
    }
}
