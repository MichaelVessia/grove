use std::time::{Duration, Instant};

const DOUBLE_ESCAPE_WINDOW_MS: u64 = 150;

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
            last_escape_time: None,
        }
    }

    pub fn handle_key(&mut self, key: InteractiveKey, now: Instant) -> InteractiveAction {
        self.last_key_time = now;

        match key {
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
    ) {
        self.cursor_row = cursor_row;
        self.cursor_col = cursor_col;
        self.cursor_visible = cursor_visible;
        self.pane_height = pane_height;
        self.pane_width = pane_width;
    }
}

pub fn key_to_action(key: InteractiveKey) -> InteractiveAction {
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

pub fn is_paste_event(text: &str) -> bool {
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
                        while let Some(value) = chars.next() {
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

pub fn looks_like_mouse_fragment(fragment: &str) -> bool {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return false;
    }

    trimmed.starts_with("[<")
        || trimmed.starts_with("\u{1b}[<")
        || (trimmed.ends_with('M') || trimmed.ends_with('m'))
            && trimmed.chars().all(|character| {
                matches!(character, '[' | '<' | ';' | 'M' | 'm') || character.is_ascii_digit()
            })
}

pub fn should_snap_back_for_input(fragment: &str) -> bool {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed == "Escape" || looks_like_mouse_fragment(trimmed) {
        return false;
    }
    if trimmed == "[" {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload, is_paste_event,
        looks_like_mouse_fragment, render_cursor_overlay, render_cursor_overlay_ansi,
        should_snap_back_for_input, tmux_send_keys_command,
    };

    #[test]
    fn double_escape_exits_within_window() {
        let now = Instant::now();
        let mut state =
            InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);

        assert_eq!(
            state.handle_key(InteractiveKey::Escape, now),
            InteractiveAction::SendNamed("Escape".to_string())
        );
        assert_eq!(
            state.handle_key(InteractiveKey::Escape, now + Duration::from_millis(120)),
            InteractiveAction::ExitInteractive
        );
    }

    #[test]
    fn escape_outside_window_is_forwarded_again() {
        let now = Instant::now();
        let mut state =
            InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);

        assert_eq!(
            state.handle_key(InteractiveKey::Escape, now),
            InteractiveAction::SendNamed("Escape".to_string())
        );
        assert_eq!(
            state.handle_key(InteractiveKey::Escape, now + Duration::from_millis(200)),
            InteractiveAction::SendNamed("Escape".to_string())
        );
    }

    #[test]
    fn key_mapping_covers_named_and_literal_tmux_forms() {
        assert_eq!(
            tmux_send_keys_command(
                "grove-ws-auth",
                &InteractiveAction::SendNamed("Enter".to_string())
            ),
            Some(vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-auth".to_string(),
                "Enter".to_string(),
            ])
        );

        assert_eq!(
            tmux_send_keys_command(
                "grove-ws-auth",
                &InteractiveAction::SendLiteral("x".to_string())
            ),
            Some(vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-auth".to_string(),
                "x".to_string(),
            ])
        );
    }

    #[test]
    fn paste_payload_wraps_only_when_bracketed_mode_and_large_input() {
        assert!(!is_paste_event("short"));
        assert!(is_paste_event("line 1\nline 2"));

        assert_eq!(encode_paste_payload("short", true), "short");
        assert_eq!(
            encode_paste_payload("line 1\nline 2", true),
            "\u{1b}[200~line 1\nline 2\u{1b}[201~"
        );
    }

    #[test]
    fn cursor_overlay_marks_current_column() {
        assert_eq!(render_cursor_overlay("abcd", 1, true), "a|bcd");
        assert_eq!(render_cursor_overlay("ab", 4, true), "ab  |");
        assert_eq!(render_cursor_overlay("ab", 1, false), "ab");
    }

    #[test]
    fn ansi_cursor_overlay_preserves_ansi_and_inserts_marker() {
        let line = "A\u{1b}[31mBC\u{1b}[0mD";
        let plain = "ABCD";
        assert_eq!(
            render_cursor_overlay_ansi(line, plain, 2, true),
            "A\u{1b}[31mB|C\u{1b}[0mD"
        );
        assert_eq!(
            render_cursor_overlay_ansi(line, plain, 6, true),
            "A\u{1b}[31mBC\u{1b}[0mD  |"
        );
    }

    #[test]
    fn mouse_fragment_guard_blocks_suspicious_inputs() {
        assert!(looks_like_mouse_fragment("[<35;192;47M"));
        assert!(looks_like_mouse_fragment("\u{1b}[<65;10;5m"));
        assert!(!looks_like_mouse_fragment("hello"));

        assert!(!should_snap_back_for_input("Escape"));
        assert!(!should_snap_back_for_input("[<35;192;47M"));
        assert!(!should_snap_back_for_input("["));
        assert!(should_snap_back_for_input("a"));
    }

    #[test]
    fn alt_copy_and_paste_map_to_special_actions() {
        let now = Instant::now();
        let mut state =
            InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);

        assert_eq!(
            state.handle_key(InteractiveKey::AltC, now),
            InteractiveAction::CopySelection
        );
        assert_eq!(
            state.handle_key(InteractiveKey::AltV, now),
            InteractiveAction::PasteClipboard
        );
    }
}
