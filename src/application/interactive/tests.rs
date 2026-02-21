use std::io::{BufRead, BufReader, BufWriter};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use crate::interface::daemon::DaemonSessionSendKeysPayload;

use super::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload, is_paste_event,
    multiplexer_send_input_command, render_cursor_overlay, render_cursor_overlay_ansi,
};

#[test]
fn double_escape_exits_within_window() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    assert_eq!(
        state.handle_key(InteractiveKey::CtrlBackslash, now),
        InteractiveAction::ExitInteractive
    );
}

#[test]
fn key_mapping_covers_named_and_literal_tmux_forms() {
    assert_eq!(
        multiplexer_send_input_command(
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
        multiplexer_send_input_command(
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
fn back_tab_maps_to_tmux_btab() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    assert_eq!(
        state.handle_key(InteractiveKey::BackTab, now),
        InteractiveAction::SendNamed("BTab".to_string())
    );
}

#[test]
fn modified_enter_maps_to_kitty_csi_u_sequence() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    assert_eq!(
        state.handle_key(
            InteractiveKey::ModifiedEnter {
                shift: true,
                alt: false,
                ctrl: false,
            },
            now
        ),
        InteractiveAction::SendLiteral("\u{1b}[13;2u".to_string())
    );

    assert_eq!(
        state.handle_key(
            InteractiveKey::ModifiedEnter {
                shift: true,
                alt: true,
                ctrl: true,
            },
            now
        ),
        InteractiveAction::SendLiteral("\u{1b}[13;8u".to_string())
    );
}

#[test]
fn modified_enter_forwards_via_tmux_literal_send_keys() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
    let action = state.handle_key(
        InteractiveKey::ModifiedEnter {
            shift: true,
            alt: false,
            ctrl: false,
        },
        now,
    );

    assert_eq!(
        multiplexer_send_input_command("grove-ws-auth", &action),
        Some(vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "grove-ws-auth".to_string(),
            "\u{1b}[13;2u".to_string(),
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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
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
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    assert!(!state.should_drop_split_mouse_fragment('[', now));
}

#[test]
fn alt_copy_and_paste_map_to_special_actions() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    assert_eq!(
        state.handle_key(InteractiveKey::AltC, now),
        InteractiveAction::CopySelection
    );
    assert_eq!(
        state.handle_key(InteractiveKey::AltV, now),
        InteractiveAction::PasteClipboard
    );
}

#[test]
fn pipeline_send_keys_writes_json_newline_to_stream() {
    let (reader_stream, writer_stream) =
        UnixStream::pair().expect("unix stream pair should create");
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
    state.daemon_stream = Some(BufWriter::new(writer_stream));

    let payload = DaemonSessionSendKeysPayload {
        command: vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-auth".to_string(),
            "Enter".to_string(),
        ],
    };
    state
        .pipeline_send_keys(payload)
        .expect("pipeline send should succeed");

    drop(state);

    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("should read pipeline output");
    let parsed: serde_json::Value =
        serde_json::from_str(line.trim()).expect("pipeline output should be valid JSON");
    assert_eq!(parsed["type"], "session_send_keys");
}

#[test]
fn close_daemon_stream_clears_field() {
    let (_reader_stream, writer_stream) =
        UnixStream::pair().expect("unix stream pair should create");
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );
    state.daemon_stream = Some(BufWriter::new(writer_stream));
    assert!(state.daemon_stream.is_some());

    state.close_daemon_stream();
    assert!(state.daemon_stream.is_none());
}

#[test]
fn pipeline_send_keys_returns_error_when_no_stream() {
    let now = Instant::now();
    let mut state = InteractiveState::new(
        "%1".to_string(),
        "grove-ws-auth".to_string(),
        now,
        40,
        120,
        None,
    );

    let payload = DaemonSessionSendKeysPayload {
        command: vec!["tmux".to_string(), "send-keys".to_string()],
    };
    let result = state.pipeline_send_keys(payload);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotConnected);
}
