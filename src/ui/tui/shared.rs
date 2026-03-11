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
use ftui::PackedRgba;

pub(super) const WORKSPACE_LAUNCH_PROMPT_FILENAME: &str = ".grove/prompt";
pub(super) const WORKSPACE_INIT_COMMAND_FILENAME: &str = ".grove/init_command";
pub(super) const WORKSPACE_SKIP_PERMISSIONS_FILENAME: &str = ".grove/skip_permissions";
pub(super) const HEADER_HEIGHT: u16 = 1;
pub(super) const STATUS_HEIGHT: u16 = 1;
pub(super) const DIVIDER_WIDTH: u16 = 1;
#[cfg_attr(not(test), allow(dead_code))]
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
pub(super) const HIT_ID_CREATE_DIALOG_TAB: u32 = 21;
pub(super) const HIT_ID_SESSION_CLEANUP_DIALOG: u32 = 22;
pub(super) const HIT_ID_RENAME_TAB_DIALOG: u32 = 23;
pub(super) const HIT_ID_PROJECT_DIALOG_LIST: u32 = 24;
pub(super) const HIT_ID_PROJECT_ADD_RESULTS_LIST: u32 = 25;
pub(super) const HIT_ID_PULL_UPSTREAM_DIALOG: u32 = 26;
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
pub(super) const AGENT_ACTIVITY_WINDOW_FRAMES: usize = 6;
pub(super) const LOCAL_TYPING_SUPPRESS_MS: u64 = 400;
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
pub(super) struct UiTheme {
    pub(super) base: PackedRgba,
    pub(super) mantle: PackedRgba,
    pub(super) crust: PackedRgba,
    pub(super) surface0: PackedRgba,
    pub(super) surface1: PackedRgba,
    pub(super) surface2: PackedRgba,
    pub(super) overlay0: PackedRgba,
    pub(super) text: PackedRgba,
    pub(super) subtext1: PackedRgba,
    pub(super) subtext0: PackedRgba,
    pub(super) blue: PackedRgba,
    pub(super) lavender: PackedRgba,
    pub(super) green: PackedRgba,
    pub(super) yellow: PackedRgba,
    pub(super) red: PackedRgba,
    pub(super) peach: PackedRgba,
    pub(super) mauve: PackedRgba,
    pub(super) teal: PackedRgba,
}

#[derive(Debug, Clone, Copy)]
struct ThemePalette {
    background_color: PackedRgba,
    header_color: PackedRgba,
    backdrop_color: PackedRgba,
    surface_color: PackedRgba,
    surface_focused_color: PackedRgba,
    surface_elevated_color: PackedRgba,
    border_color: PackedRgba,
    text_color: PackedRgba,
    strong_muted_text_color: PackedRgba,
    muted_text_color: PackedRgba,
    accent_color: PackedRgba,
    accent_soft_color: PackedRgba,
    accent_teal_color: PackedRgba,
    success_color: PackedRgba,
    warning_color: PackedRgba,
    error_color: PackedRgba,
    accent_warm_color: PackedRgba,
    accent_alt_color: PackedRgba,
}

impl ThemePalette {
    const fn to_ui_theme(self) -> UiTheme {
        UiTheme {
            base: self.background_color,
            mantle: self.header_color,
            crust: self.backdrop_color,
            surface0: self.surface_color,
            surface1: self.surface_focused_color,
            surface2: self.surface_elevated_color,
            overlay0: self.border_color,
            text: self.text_color,
            subtext1: self.strong_muted_text_color,
            subtext0: self.muted_text_color,
            blue: self.accent_color,
            lavender: self.accent_soft_color,
            green: self.success_color,
            yellow: self.warning_color,
            red: self.error_color,
            peach: self.accent_warm_color,
            mauve: self.accent_alt_color,
            teal: self.accent_teal_color,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ThemePreset {
    name: ThemeName,
    display_name: &'static str,
    palette: ThemePalette,
}

const THEME_PRESETS: [ThemePreset; 8] = [
    ThemePreset {
        name: ThemeName::Monokai,
        display_name: "Monokai",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(39, 40, 34),
            header_color: PackedRgba::rgb(30, 31, 28),
            backdrop_color: PackedRgba::rgb(22, 22, 19),
            surface_color: PackedRgba::rgb(62, 61, 50),
            surface_focused_color: PackedRgba::rgb(73, 72, 62),
            surface_elevated_color: PackedRgba::rgb(92, 88, 76),
            border_color: PackedRgba::rgb(117, 113, 94),
            text_color: PackedRgba::rgb(248, 248, 242),
            strong_muted_text_color: PackedRgba::rgb(212, 208, 191),
            muted_text_color: PackedRgba::rgb(162, 160, 142),
            accent_color: PackedRgba::rgb(102, 217, 239),
            accent_soft_color: PackedRgba::rgb(174, 129, 255),
            accent_teal_color: PackedRgba::rgb(17, 168, 205),
            success_color: PackedRgba::rgb(166, 226, 46),
            warning_color: PackedRgba::rgb(230, 219, 116),
            error_color: PackedRgba::rgb(249, 38, 114),
            accent_warm_color: PackedRgba::rgb(253, 151, 31),
            accent_alt_color: PackedRgba::rgb(249, 38, 114),
        },
    },
    ThemePreset {
        name: ThemeName::CatppuccinLatte,
        display_name: "Catppuccin Latte",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(239, 241, 245),
            header_color: PackedRgba::rgb(230, 233, 239),
            backdrop_color: PackedRgba::rgb(220, 224, 232),
            surface_color: PackedRgba::rgb(204, 208, 218),
            surface_focused_color: PackedRgba::rgb(188, 192, 204),
            surface_elevated_color: PackedRgba::rgb(172, 176, 190),
            border_color: PackedRgba::rgb(156, 160, 176),
            text_color: PackedRgba::rgb(76, 79, 105),
            strong_muted_text_color: PackedRgba::rgb(92, 95, 119),
            muted_text_color: PackedRgba::rgb(108, 111, 133),
            accent_color: PackedRgba::rgb(30, 102, 245),
            accent_soft_color: PackedRgba::rgb(114, 135, 253),
            accent_teal_color: PackedRgba::rgb(23, 146, 153),
            success_color: PackedRgba::rgb(64, 160, 43),
            warning_color: PackedRgba::rgb(223, 142, 29),
            error_color: PackedRgba::rgb(210, 15, 57),
            accent_warm_color: PackedRgba::rgb(254, 100, 11),
            accent_alt_color: PackedRgba::rgb(136, 57, 239),
        },
    },
    ThemePreset {
        name: ThemeName::CatppuccinFrappe,
        display_name: "Catppuccin Frappe",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(48, 52, 70),
            header_color: PackedRgba::rgb(41, 44, 60),
            backdrop_color: PackedRgba::rgb(35, 38, 52),
            surface_color: PackedRgba::rgb(65, 69, 89),
            surface_focused_color: PackedRgba::rgb(81, 87, 109),
            surface_elevated_color: PackedRgba::rgb(98, 104, 128),
            border_color: PackedRgba::rgb(115, 121, 148),
            text_color: PackedRgba::rgb(198, 208, 245),
            strong_muted_text_color: PackedRgba::rgb(181, 191, 226),
            muted_text_color: PackedRgba::rgb(165, 173, 206),
            accent_color: PackedRgba::rgb(140, 170, 238),
            accent_soft_color: PackedRgba::rgb(186, 187, 241),
            accent_teal_color: PackedRgba::rgb(129, 200, 190),
            success_color: PackedRgba::rgb(166, 209, 137),
            warning_color: PackedRgba::rgb(229, 200, 144),
            error_color: PackedRgba::rgb(231, 130, 132),
            accent_warm_color: PackedRgba::rgb(239, 159, 118),
            accent_alt_color: PackedRgba::rgb(202, 158, 230),
        },
    },
    ThemePreset {
        name: ThemeName::CatppuccinMacchiato,
        display_name: "Catppuccin Macchiato",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(36, 39, 58),
            header_color: PackedRgba::rgb(30, 32, 48),
            backdrop_color: PackedRgba::rgb(24, 25, 38),
            surface_color: PackedRgba::rgb(54, 58, 79),
            surface_focused_color: PackedRgba::rgb(73, 77, 100),
            surface_elevated_color: PackedRgba::rgb(91, 96, 120),
            border_color: PackedRgba::rgb(110, 115, 141),
            text_color: PackedRgba::rgb(202, 211, 245),
            strong_muted_text_color: PackedRgba::rgb(184, 192, 224),
            muted_text_color: PackedRgba::rgb(165, 173, 203),
            accent_color: PackedRgba::rgb(138, 173, 244),
            accent_soft_color: PackedRgba::rgb(183, 189, 248),
            accent_teal_color: PackedRgba::rgb(139, 213, 202),
            success_color: PackedRgba::rgb(166, 218, 149),
            warning_color: PackedRgba::rgb(238, 212, 159),
            error_color: PackedRgba::rgb(237, 135, 150),
            accent_warm_color: PackedRgba::rgb(245, 169, 127),
            accent_alt_color: PackedRgba::rgb(198, 160, 246),
        },
    },
    ThemePreset {
        name: ThemeName::CatppuccinMocha,
        display_name: "Catppuccin Mocha",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(30, 30, 46),
            header_color: PackedRgba::rgb(24, 24, 37),
            backdrop_color: PackedRgba::rgb(17, 17, 27),
            surface_color: PackedRgba::rgb(49, 50, 68),
            surface_focused_color: PackedRgba::rgb(69, 71, 90),
            surface_elevated_color: PackedRgba::rgb(88, 91, 112),
            border_color: PackedRgba::rgb(108, 112, 134),
            text_color: PackedRgba::rgb(205, 214, 244),
            strong_muted_text_color: PackedRgba::rgb(186, 194, 222),
            muted_text_color: PackedRgba::rgb(166, 173, 200),
            accent_color: PackedRgba::rgb(137, 180, 250),
            accent_soft_color: PackedRgba::rgb(180, 190, 254),
            accent_teal_color: PackedRgba::rgb(148, 226, 213),
            success_color: PackedRgba::rgb(166, 227, 161),
            warning_color: PackedRgba::rgb(249, 226, 175),
            error_color: PackedRgba::rgb(243, 139, 168),
            accent_warm_color: PackedRgba::rgb(250, 179, 135),
            accent_alt_color: PackedRgba::rgb(203, 166, 247),
        },
    },
    ThemePreset {
        name: ThemeName::RosePine,
        display_name: "Rosé Pine",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(25, 23, 36),
            header_color: PackedRgba::rgb(19, 17, 30),
            backdrop_color: PackedRgba::rgb(13, 11, 24),
            surface_color: PackedRgba::rgb(31, 29, 46),
            surface_focused_color: PackedRgba::rgb(38, 35, 58),
            surface_elevated_color: PackedRgba::rgb(64, 61, 82),
            border_color: PackedRgba::rgb(110, 106, 134),
            text_color: PackedRgba::rgb(224, 222, 244),
            strong_muted_text_color: PackedRgba::rgb(184, 181, 207),
            muted_text_color: PackedRgba::rgb(144, 140, 170),
            accent_color: PackedRgba::rgb(156, 207, 216),
            accent_soft_color: PackedRgba::rgb(235, 188, 186),
            accent_teal_color: PackedRgba::rgb(49, 116, 143),
            success_color: PackedRgba::rgb(156, 207, 216),
            warning_color: PackedRgba::rgb(246, 193, 119),
            error_color: PackedRgba::rgb(235, 111, 146),
            accent_warm_color: PackedRgba::rgb(246, 193, 119),
            accent_alt_color: PackedRgba::rgb(196, 167, 231),
        },
    },
    ThemePreset {
        name: ThemeName::RosePineMoon,
        display_name: "Rosé Pine Moon",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(35, 33, 54),
            header_color: PackedRgba::rgb(28, 26, 45),
            backdrop_color: PackedRgba::rgb(21, 19, 36),
            surface_color: PackedRgba::rgb(42, 39, 63),
            surface_focused_color: PackedRgba::rgb(57, 53, 82),
            surface_elevated_color: PackedRgba::rgb(68, 65, 90),
            border_color: PackedRgba::rgb(110, 106, 134),
            text_color: PackedRgba::rgb(224, 222, 244),
            strong_muted_text_color: PackedRgba::rgb(184, 181, 207),
            muted_text_color: PackedRgba::rgb(144, 140, 170),
            accent_color: PackedRgba::rgb(156, 207, 216),
            accent_soft_color: PackedRgba::rgb(235, 188, 186),
            accent_teal_color: PackedRgba::rgb(62, 143, 176),
            success_color: PackedRgba::rgb(156, 207, 216),
            warning_color: PackedRgba::rgb(246, 193, 119),
            error_color: PackedRgba::rgb(235, 111, 146),
            accent_warm_color: PackedRgba::rgb(246, 193, 119),
            accent_alt_color: PackedRgba::rgb(196, 167, 231),
        },
    },
    ThemePreset {
        name: ThemeName::RosePineDawn,
        display_name: "Rosé Pine Dawn",
        palette: ThemePalette {
            background_color: PackedRgba::rgb(250, 244, 237),
            header_color: PackedRgba::rgb(242, 233, 225),
            backdrop_color: PackedRgba::rgb(234, 225, 218),
            surface_color: PackedRgba::rgb(223, 218, 217),
            surface_focused_color: PackedRgba::rgb(206, 202, 205),
            surface_elevated_color: PackedRgba::rgb(188, 186, 193),
            border_color: PackedRgba::rgb(152, 147, 165),
            text_color: PackedRgba::rgb(87, 82, 121),
            strong_muted_text_color: PackedRgba::rgb(104, 100, 134),
            muted_text_color: PackedRgba::rgb(121, 117, 147),
            accent_color: PackedRgba::rgb(86, 148, 159),
            accent_soft_color: PackedRgba::rgb(215, 130, 126),
            accent_teal_color: PackedRgba::rgb(40, 105, 131),
            success_color: PackedRgba::rgb(86, 148, 159),
            warning_color: PackedRgba::rgb(234, 157, 52),
            error_color: PackedRgba::rgb(180, 99, 122),
            accent_warm_color: PackedRgba::rgb(234, 157, 52),
            accent_alt_color: PackedRgba::rgb(144, 122, 169),
        },
    },
];

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

fn theme_palette(theme_name: ThemeName) -> ThemePalette {
    theme_preset(theme_name).palette
}

pub(super) fn ui_theme_for(theme_name: ThemeName) -> UiTheme {
    theme_palette(theme_name).to_ui_theme()
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn ui_theme() -> UiTheme {
    ui_theme_for(ThemeName::default())
}

impl super::GroveApp {
    pub(super) fn active_ui_theme(&self) -> UiTheme {
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
}

impl PreviewTab {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Agent => "Agent",
            Self::Shell => "Shell",
            Self::Git => "Git",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceTabKind {
    Home,
    Agent,
    Shell,
    Git,
}

impl WorkspaceTabKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Agent => "Agent",
            Self::Shell => "Shell",
            Self::Git => "Git",
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
    pub(super) fn new() -> Self {
        let home = WorkspaceTab::home(1);
        Self {
            tabs: vec![home],
            active_tab_id: 1,
            next_seq: 2,
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
        self.tabs
            .sort_by_key(|entry| (entry.kind != WorkspaceTabKind::Home, entry.id));
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
            return;
        }
        let home = WorkspaceTab::home(self.next_seq);
        self.next_seq = self.next_seq.saturating_add(1);
        self.tabs.insert(0, home.clone());
        self.active_tab_id = home.id;
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
    pub(super) attention_ack_workspace_path: Option<PathBuf>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_tab(session_name: &str) -> WorkspaceTab {
        WorkspaceTab {
            id: 0,
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
            kind: WorkspaceTabKind::Shell,
            title: String::new(),
            session_name: Some(session_name.to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        }
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
    fn insert_restored_tab_rejects_duplicate_session_name() {
        let mut tabs = WorkspaceTabsState::new();
        let tab1 = WorkspaceTab {
            id: 10,
            kind: WorkspaceTabKind::Agent,
            title: "Agent 1".to_string(),
            session_name: Some("ws-agent-1".to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        };
        assert!(tabs.insert_restored_tab(tab1));

        let tab2 = WorkspaceTab {
            id: 20,
            kind: WorkspaceTabKind::Agent,
            title: "Agent 2".to_string(),
            session_name: Some("ws-agent-1".to_string()),
            agent_type: None,
            state: WorkspaceTabRuntimeState::Running,
        };
        assert!(!tabs.insert_restored_tab(tab2));
    }
}
