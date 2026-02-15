use std::time::{Duration, Instant};

use super::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload, is_paste_event,
    render_cursor_overlay, render_cursor_overlay_ansi, tmux_send_keys_command,
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
fn ctrl_backslash_exits_immediately() {
    let now = Instant::now();
    let mut state =
        InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);

    assert_eq!(
        state.handle_key(InteractiveKey::CtrlBackslash, now),
        InteractiveAction::ExitInteractive
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
fn split_mouse_fragment_filter_drops_sequence_after_mouse_event() {
    let now = Instant::now();
    let mut state =
        InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);
    state.note_mouse_event(now);

    assert!(state.should_drop_split_mouse_fragment('[', now));
    assert!(state.should_drop_split_mouse_fragment('<', now + Duration::from_millis(1)));
    assert!(state.should_drop_split_mouse_fragment('3', now + Duration::from_millis(2)));
    assert!(state.should_drop_split_mouse_fragment('5', now + Duration::from_millis(3)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(4)));
    assert!(state.should_drop_split_mouse_fragment('1', now + Duration::from_millis(5)));
    assert!(state.should_drop_split_mouse_fragment('0', now + Duration::from_millis(6)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(7)));
    assert!(state.should_drop_split_mouse_fragment('5', now + Duration::from_millis(8)));
    assert!(state.should_drop_split_mouse_fragment('M', now + Duration::from_millis(9)));
}

#[test]
fn split_mouse_fragment_filter_drops_sequence_when_prefix_bracket_is_missing() {
    let now = Instant::now();
    let mut state =
        InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);
    state.note_mouse_event(now);

    assert!(state.should_drop_split_mouse_fragment('<', now));
    assert!(state.should_drop_split_mouse_fragment('6', now + Duration::from_millis(1)));
    assert!(state.should_drop_split_mouse_fragment('5', now + Duration::from_millis(2)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(3)));
    assert!(state.should_drop_split_mouse_fragment('1', now + Duration::from_millis(4)));
    assert!(state.should_drop_split_mouse_fragment('0', now + Duration::from_millis(5)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(6)));
    assert!(state.should_drop_split_mouse_fragment('4', now + Duration::from_millis(7)));
    assert!(state.should_drop_split_mouse_fragment('M', now + Duration::from_millis(8)));
}

#[test]
fn split_mouse_fragment_filter_drops_boundary_marker_then_sequence() {
    let now = Instant::now();
    let mut state =
        InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);
    state.note_mouse_event(now);

    assert!(state.should_drop_split_mouse_fragment('M', now));
    assert!(state.should_drop_split_mouse_fragment('[', now + Duration::from_millis(1)));
    assert!(state.should_drop_split_mouse_fragment('<', now + Duration::from_millis(2)));
    assert!(state.should_drop_split_mouse_fragment('3', now + Duration::from_millis(3)));
    assert!(state.should_drop_split_mouse_fragment('5', now + Duration::from_millis(4)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(5)));
    assert!(state.should_drop_split_mouse_fragment('1', now + Duration::from_millis(6)));
    assert!(state.should_drop_split_mouse_fragment('0', now + Duration::from_millis(7)));
    assert!(state.should_drop_split_mouse_fragment(';', now + Duration::from_millis(8)));
    assert!(state.should_drop_split_mouse_fragment('5', now + Duration::from_millis(9)));
    assert!(state.should_drop_split_mouse_fragment('M', now + Duration::from_millis(10)));
}

#[test]
fn split_mouse_fragment_filter_allows_normal_bracket_typing() {
    let now = Instant::now();
    let mut state =
        InteractiveState::new("%1".to_string(), "grove-ws-auth".to_string(), now, 40, 120);

    assert!(!state.should_drop_split_mouse_fragment('[', now));
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
