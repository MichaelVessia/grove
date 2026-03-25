use std::collections::HashSet;

use ftui::Color;

use crate::infrastructure::config::ThemeName;

pub fn tmux_theme_commands(session_name: &str, theme_name: ThemeName) -> Vec<Vec<String>> {
    let theme = crate::ui::tui::ui_theme_for(theme_name);

    vec![
        tmux_style_option(
            session_name,
            "window-style",
            Some(theme.text),
            Some(theme.background),
        ),
        tmux_style_option(
            session_name,
            "window-active-style",
            Some(theme.text),
            Some(theme.background),
        ),
        tmux_style_option(session_name, "pane-border-style", Some(theme.border), None),
        tmux_style_option(
            session_name,
            "pane-active-border-style",
            Some(theme.primary),
            None,
        ),
        tmux_style_option(
            session_name,
            "status-style",
            Some(theme.text),
            Some(theme.surface),
        ),
        tmux_style_option(
            session_name,
            "message-style",
            Some(theme.text),
            Some(theme.surface),
        ),
        tmux_style_option(
            session_name,
            "mode-style",
            Some(theme.selection_fg),
            Some(theme.selection_bg),
        ),
        tmux_option(session_name, "display-panes-colour", tmux_hex(theme.border)),
        tmux_option(
            session_name,
            "display-panes-active-colour",
            tmux_hex(theme.primary),
        ),
    ]
}

pub fn grove_managed_tmux_sessions(metadata_rows: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    metadata_rows
        .lines()
        .filter_map(|row| row.split('\t').next())
        .map(str::trim)
        .filter(|session| {
            session.starts_with("grove-ws-")
                || session.starts_with("grove-wt-")
                || session.starts_with("grove-task-")
        })
        .filter(|session| seen.insert((*session).to_string()))
        .map(ToOwned::to_owned)
        .collect()
}

fn tmux_option(session_name: &str, option: &str, value: String) -> Vec<String> {
    vec![
        "tmux".to_string(),
        "set-option".to_string(),
        "-t".to_string(),
        session_name.to_string(),
        option.to_string(),
        value,
    ]
}

fn tmux_style_option(
    session_name: &str,
    option: &str,
    foreground: Option<Color>,
    background: Option<Color>,
) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(color) = background {
        parts.push(format!("bg={}", tmux_hex(color)));
    }
    if let Some(color) = foreground {
        parts.push(format!("fg={}", tmux_hex(color)));
    }
    tmux_option(session_name, option, parts.join(","))
}

fn tmux_hex(color: Color) -> String {
    let rgb = color.to_rgb();
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grove_managed_tmux_sessions_filters_and_deduplicates_known_prefixes() {
        let rows = "grove-ws-main\t\t\t\t\t\t\ngrove-wt-main-grove\t\t\t\t\t\t\ngrove-task-feature-a\t\t\t\t\t\t\ngrove-ws-main\t\t\t\t\t\t\nscratch\t\t\t\t\t\t\n";

        assert_eq!(
            grove_managed_tmux_sessions(rows),
            vec![
                "grove-ws-main".to_string(),
                "grove-wt-main-grove".to_string(),
                "grove-task-feature-a".to_string(),
            ]
        );
    }
}
