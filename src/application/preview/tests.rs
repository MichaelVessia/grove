use std::time::{Duration, Instant};

use super::{PreviewState, split_output_lines};

#[test]
fn split_output_lines_trims_final_newline() {
    assert_eq!(
        split_output_lines("a\nb\n"),
        vec!["a".to_string(), "b".to_string()]
    );
    assert!(split_output_lines("\n").is_empty());
}

#[test]
fn capture_ignores_mouse_noise_in_clean_diff() {
    let mut state = PreviewState::new();

    let first = state.apply_capture("hello\u{1b}[?1000h\u{1b}[<35;192;47M");
    assert!(first.changed_raw);
    assert!(first.changed_cleaned);
    assert_eq!(state.lines, vec!["hello".to_string()]);
    assert_eq!(state.render_lines, vec!["hello".to_string()]);

    let second = state.apply_capture("hello\u{1b}[?1000l");
    assert!(second.changed_raw);
    assert!(!second.changed_cleaned);
    assert_eq!(state.lines, vec!["hello".to_string()]);
    assert_eq!(state.render_lines, vec!["hello".to_string()]);
}

#[test]
fn scroll_up_pauses_autoscroll_and_scroll_down_resumes_at_bottom() {
    let mut state = PreviewState::new();
    state.lines = vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
        "4".to_string(),
    ];

    let base = Instant::now();
    assert!(state.scroll(-2, base, 2));
    assert!(!state.auto_scroll);
    assert_eq!(state.offset, 2);

    assert!(state.scroll(1, base + Duration::from_millis(200), 2));
    assert!(!state.auto_scroll);
    assert_eq!(state.offset, 1);

    assert!(state.scroll(1, base + Duration::from_millis(400), 2));
    assert!(state.auto_scroll);
    assert_eq!(state.offset, 0);
}

#[test]
fn scroll_up_clamps_offset_to_available_lines() {
    let mut state = PreviewState::new();
    state.lines = vec!["1".to_string(), "2".to_string()];

    assert!(state.scroll(-10, Instant::now(), 1));
    assert_eq!(state.offset, 1);
}

#[test]
fn apply_capture_clamps_existing_offset_when_output_shrinks() {
    let mut state = PreviewState::new();
    state.lines = vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
        "4".to_string(),
    ];
    state.offset = 3;
    state.auto_scroll = false;

    state.apply_capture("line-a\nline-b");
    assert_eq!(state.lines.len(), 2);
    assert_eq!(state.offset, 2);
}

#[test]
fn scroll_burst_guard_drops_rapid_bursts() {
    let mut state = PreviewState::new();
    state.lines = (1..=20).map(|value| value.to_string()).collect();
    let base = Instant::now();

    assert!(state.scroll(-1, base, 5));
    assert!(!state.scroll(-1, base + Duration::from_millis(1), 5));
    assert!(!state.scroll(-1, base + Duration::from_millis(2), 5));
    assert!(!state.scroll(-1, base + Duration::from_millis(3), 5));
    assert!(!state.scroll(-1, base + Duration::from_millis(4), 5));
    assert!(state.scroll(-1, base + Duration::from_millis(50), 5));
    assert!(state.scroll(-1, base + Duration::from_millis(130), 5));
}

#[test]
fn scroll_is_noop_when_content_fits_viewport() {
    let mut state = PreviewState::new();
    state.lines = vec!["1".to_string(), "2".to_string()];

    assert!(!state.scroll(-1, Instant::now(), 4));
    assert_eq!(state.offset, 0);
    assert!(state.auto_scroll);
}

#[test]
fn visible_lines_respects_offset_from_bottom() {
    let mut state = PreviewState::new();
    state.lines = vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
        "4".to_string(),
        "5".to_string(),
    ];
    state.offset = 1;

    let visible = state.visible_lines(2);
    assert_eq!(visible, vec!["3".to_string(), "4".to_string()]);
}

#[test]
fn capture_record_ring_buffer_caps_at_10() {
    let mut state = PreviewState::new();

    for i in 0..12 {
        state.apply_capture(&format!("output-{i}"));
    }

    assert_eq!(state.recent_captures.len(), 10);
    assert!(
        state
            .recent_captures
            .front()
            .unwrap()
            .raw_output
            .contains("output-2")
    );
    assert!(
        state
            .recent_captures
            .back()
            .unwrap()
            .raw_output
            .contains("output-11")
    );
}

#[test]
fn capture_record_contains_expected_fields() {
    let mut state = PreviewState::new();
    state.apply_capture("hello world");

    assert_eq!(state.recent_captures.len(), 1);
    let record = state.recent_captures.front().unwrap();
    assert_eq!(record.raw_output, "hello world");
    assert!(record.changed_raw);
    assert!(record.changed_cleaned);
    assert!(record.ts > 0);
    assert!(record.digest.raw_len > 0);
}
