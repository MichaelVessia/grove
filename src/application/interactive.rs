use std::time::{Duration, Instant};

use crate::application::agent_runtime::zellij_config_path;
use crate::config::MultiplexerKind;

const DOUBLE_ESCAPE_WINDOW_MS: u64 = 150;
const SPLIT_MOUSE_FRAGMENT_START_WINDOW_MS: u64 = 10;
const SPLIT_MOUSE_FRAGMENT_MAX_AGE_MS: u64 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractiveState {
    pub active: bool,
    pub target_pane: String,
    pub target_session: String,
    pub last_key_time: Instant,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    pub pane_height: u16,
    pub pane_width: u16,
    pub bracketed_paste: bool,
    last_mouse_event_time: Option<Instant>,
    mouse_fragment_started_at: Option<Instant>,
    last_escape_time: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveKey {
    Enter,
    Tab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Escape,
    CtrlBackslash,
    Ctrl(char),
    Function(u8),
    Char(char),
    AltC,
    AltV,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveAction {
    SendNamed(String),
    SendLiteral(String),
    ExitInteractive,
    CopySelection,
    PasteClipboard,
    Noop,
}

impl InteractiveState {
    pub fn new(
        target_pane: String,
        target_session: String,
        now: Instant,
        pane_height: u16,
        pane_width: u16,
    ) -> Self {
        Self {
            active: true,
            target_pane,
            target_session,
            last_key_time: now,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            pane_height,
            pane_width,
            bracketed_paste: false,
            last_mouse_event_time: None,
            mouse_fragment_started_at: None,
            last_escape_time: None,
        }
    }

    pub fn handle_key(&mut self, key: InteractiveKey, now: Instant) -> InteractiveAction {
        self.last_key_time = now;

        match key {
            InteractiveKey::CtrlBackslash => {
                self.last_escape_time = None;
                InteractiveAction::ExitInteractive
            }
            InteractiveKey::Escape => {
                let should_exit = self.last_escape_time.is_some_and(|last_escape_time| {
                    now.saturating_duration_since(last_escape_time)
                        <= Duration::from_millis(DOUBLE_ESCAPE_WINDOW_MS)
                });

                if should_exit {
                    self.last_escape_time = None;
                    InteractiveAction::ExitInteractive
                } else {
                    self.last_escape_time = Some(now);
                    InteractiveAction::SendNamed("Escape".to_string())
                }
            }
            InteractiveKey::AltC => {
                self.last_escape_time = None;
                InteractiveAction::CopySelection
            }
            InteractiveKey::AltV => {
                self.last_escape_time = None;
                InteractiveAction::PasteClipboard
            }
            other => {
                self.last_escape_time = None;
                key_to_action(other)
            }
        }
    }

    pub fn update_cursor(
        &mut self,
        cursor_row: u16,
        cursor_col: u16,
        cursor_visible: bool,
        pane_height: u16,
        pane_width: u16,
    ) -> bool {
        if self.cursor_row == cursor_row
            && self.cursor_col == cursor_col
            && self.cursor_visible == cursor_visible
            && self.pane_height == pane_height
            && self.pane_width == pane_width
        {
            return false;
        }

        self.cursor_row = cursor_row;
        self.cursor_col = cursor_col;
        self.cursor_visible = cursor_visible;
        self.pane_height = pane_height;
        self.pane_width = pane_width;
        true
    }

    pub fn note_mouse_event(&mut self, now: Instant) {
        self.last_mouse_event_time = Some(now);
    }

    pub fn should_drop_split_mouse_fragment(&mut self, character: char, now: Instant) -> bool {
        if let Some(started_at) = self.mouse_fragment_started_at {
            if now.saturating_duration_since(started_at)
                > Duration::from_millis(SPLIT_MOUSE_FRAGMENT_MAX_AGE_MS)
            {
                self.mouse_fragment_started_at = None;
            } else if is_mouse_fragment_character(character) {
                if matches!(character, 'M' | 'm') {
                    self.mouse_fragment_started_at = None;
                }
                return true;
            } else {
                self.mouse_fragment_started_at = None;
            }
        }

        if is_mouse_fragment_start(character)
            && self
                .last_mouse_event_time
                .is_some_and(|last_mouse_event_time| {
                    now.saturating_duration_since(last_mouse_event_time)
                        <= Duration::from_millis(SPLIT_MOUSE_FRAGMENT_START_WINDOW_MS)
                })
        {
            self.mouse_fragment_started_at = Some(now);
            return true;
        }

        false
    }
}

fn is_mouse_fragment_start(character: char) -> bool {
    matches!(character, '[' | '<' | 'M' | 'm')
}

fn is_mouse_fragment_character(character: char) -> bool {
    matches!(character, '[' | '<' | ';' | 'M' | 'm') || character.is_ascii_digit()
}

fn key_to_action(key: InteractiveKey) -> InteractiveAction {
    match key {
        InteractiveKey::Enter => InteractiveAction::SendNamed("Enter".to_string()),
        InteractiveKey::Tab => InteractiveAction::SendNamed("Tab".to_string()),
        InteractiveKey::Backspace => InteractiveAction::SendNamed("BSpace".to_string()),
        InteractiveKey::Delete => InteractiveAction::SendNamed("DC".to_string()),
        InteractiveKey::Up => InteractiveAction::SendNamed("Up".to_string()),
        InteractiveKey::Down => InteractiveAction::SendNamed("Down".to_string()),
        InteractiveKey::Left => InteractiveAction::SendNamed("Left".to_string()),
        InteractiveKey::Right => InteractiveAction::SendNamed("Right".to_string()),
        InteractiveKey::Home => InteractiveAction::SendNamed("Home".to_string()),
        InteractiveKey::End => InteractiveAction::SendNamed("End".to_string()),
        InteractiveKey::PageUp => InteractiveAction::SendNamed("PPage".to_string()),
        InteractiveKey::PageDown => InteractiveAction::SendNamed("NPage".to_string()),
        InteractiveKey::Ctrl(character) if character.is_ascii_alphabetic() => {
            InteractiveAction::SendNamed(format!("C-{}", character.to_ascii_lowercase()))
        }
        InteractiveKey::Function(index) if (1..=12).contains(&index) => {
            InteractiveAction::SendNamed(format!("F{index}"))
        }
        InteractiveKey::Char(character) => InteractiveAction::SendLiteral(character.to_string()),
        _ => InteractiveAction::Noop,
    }
}

pub fn tmux_send_keys_command(
    target_session: &str,
    action: &InteractiveAction,
) -> Option<Vec<String>> {
    multiplexer_send_input_command(MultiplexerKind::Tmux, target_session, action)
}

pub fn multiplexer_send_input_command(
    multiplexer: MultiplexerKind,
    target_session: &str,
    action: &InteractiveAction,
) -> Option<Vec<String>> {
    match multiplexer {
        MultiplexerKind::Tmux => tmux_send_keys_command_impl(target_session, action),
        MultiplexerKind::Zellij => zellij_send_input_command(target_session, action),
    }
}

fn tmux_send_keys_command_impl(
    target_session: &str,
    action: &InteractiveAction,
) -> Option<Vec<String>> {
    match action {
        InteractiveAction::SendNamed(key) => Some(vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            target_session.to_string(),
            key.clone(),
        ]),
        InteractiveAction::SendLiteral(text) => Some(vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            target_session.to_string(),
            text.clone(),
        ]),
        _ => None,
    }
}

fn zellij_send_input_command(
    target_session: &str,
    action: &InteractiveAction,
) -> Option<Vec<String>> {
    let config_path = zellij_config_path().to_string_lossy().to_string();
    match action {
        InteractiveAction::SendLiteral(text) => Some(vec![
            "zellij".to_string(),
            "--config".to_string(),
            config_path.clone(),
            "--session".to_string(),
            target_session.to_string(),
            "action".to_string(),
            "write-chars".to_string(),
            text.clone(),
        ]),
        InteractiveAction::SendNamed(key) => {
            let bytes = zellij_named_key_bytes(key)?;
            let mut command = vec![
                "zellij".to_string(),
                "--config".to_string(),
                config_path,
                "--session".to_string(),
                target_session.to_string(),
                "action".to_string(),
                "write".to_string(),
            ];
            command.extend(bytes.into_iter().map(|value| value.to_string()));
            Some(command)
        }
        _ => None,
    }
}

fn zellij_named_key_bytes(key: &str) -> Option<Vec<u8>> {
    if let Some(character) = key.strip_prefix("C-") {
        let mut chars = character.chars();
        let value = chars.next()?;
        if chars.next().is_some() || !value.is_ascii_alphabetic() {
            return None;
        }
        let lower = value.to_ascii_lowercase();
        let lower_u8 = u8::try_from(u32::from(lower)).ok()?;
        let control = lower_u8 & 0x1f;
        return Some(vec![control]);
    }

    if let Some(index_text) = key.strip_prefix('F') {
        let index = index_text.parse::<u8>().ok()?;
        let sequence = match index {
            1 => "\u{1b}OP",
            2 => "\u{1b}OQ",
            3 => "\u{1b}OR",
            4 => "\u{1b}OS",
            5 => "\u{1b}[15~",
            6 => "\u{1b}[17~",
            7 => "\u{1b}[18~",
            8 => "\u{1b}[19~",
            9 => "\u{1b}[20~",
            10 => "\u{1b}[21~",
            11 => "\u{1b}[23~",
            12 => "\u{1b}[24~",
            _ => return None,
        };
        return Some(sequence.as_bytes().to_vec());
    }

    let sequence = match key {
        "Enter" => "\r",
        "Tab" => "\t",
        "BSpace" => "\u{7f}",
        "DC" => "\u{1b}[3~",
        "Up" => "\u{1b}[A",
        "Down" => "\u{1b}[B",
        "Left" => "\u{1b}[D",
        "Right" => "\u{1b}[C",
        "Home" => "\u{1b}[H",
        "End" => "\u{1b}[F",
        "PPage" => "\u{1b}[5~",
        "NPage" => "\u{1b}[6~",
        "Escape" => "\u{1b}",
        _ => return None,
    };
    Some(sequence.as_bytes().to_vec())
}

fn is_paste_event(text: &str) -> bool {
    text.contains('\n') || text.chars().count() > 10
}

pub fn encode_paste_payload(text: &str, bracketed_paste: bool) -> String {
    if bracketed_paste && is_paste_event(text) {
        return format!("\u{1b}[200~{text}\u{1b}[201~");
    }

    text.to_string()
}

pub fn render_cursor_overlay(line: &str, cursor_col: usize, cursor_visible: bool) -> String {
    if !cursor_visible {
        return line.to_string();
    }

    let characters: Vec<char> = line.chars().collect();
    if cursor_col >= characters.len() {
        let padding = " ".repeat(cursor_col.saturating_sub(characters.len()));
        return format!("{line}{padding}|");
    }

    let mut rendered = String::with_capacity(line.len().saturating_add(1));
    for (index, character) in characters.iter().enumerate() {
        if index == cursor_col {
            rendered.push('|');
        }
        rendered.push(*character);
    }

    rendered
}

pub fn render_cursor_overlay_ansi(
    line: &str,
    plain_line: &str,
    cursor_col: usize,
    cursor_visible: bool,
) -> String {
    if !cursor_visible {
        return line.to_string();
    }

    let plain_len = plain_line.chars().count();
    if cursor_col >= plain_len {
        let padding = " ".repeat(cursor_col.saturating_sub(plain_len));
        return format!("{line}{padding}|");
    }

    let mut rendered = String::with_capacity(line.len().saturating_add(1));
    let mut chars = line.chars().peekable();
    let mut visible_index = 0usize;
    let mut inserted = false;

    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            rendered.push(character);
            if let Some(next) = chars.next() {
                rendered.push(next);
                match next {
                    '[' => {
                        for value in chars.by_ref() {
                            rendered.push(value);
                            if ('\u{40}'..='\u{7e}').contains(&value) {
                                break;
                            }
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
            }
            continue;
        }

        if !inserted && visible_index == cursor_col {
            rendered.push('|');
            inserted = true;
        }

        rendered.push(character);
        visible_index = visible_index.saturating_add(1);
    }

    if !inserted {
        rendered.push('|');
    }

    rendered
}

#[cfg(test)]
mod tests;
