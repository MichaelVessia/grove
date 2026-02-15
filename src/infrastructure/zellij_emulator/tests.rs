use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{ZellijPreviewEmulator, parse_script_header_size, sanitize_log_chunk};

fn unique_log_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-zellij-emulator-{label}-{}-{timestamp}.log",
        std::process::id()
    ))
}

#[test]
fn sanitize_log_chunk_strips_script_wrappers_and_nul() {
    let sanitized = sanitize_log_chunk(
        b"Script started on now\n\0\x1b[31mred\x1b[0m\nScript done on now\n",
        true,
    );
    assert_eq!(sanitized, b"\x1b[31mred\x1b[0m");
}

#[test]
fn sanitize_log_chunk_includes_exit_sentinel_from_script_footer() {
    let sanitized = sanitize_log_chunk(
        b"Script started on now\nline one\nScript done on now [COMMAND_EXIT_CODE=\"0\"]\n",
        true,
    );
    assert_eq!(sanitized, b"line one\nexited with code 0\n");
}

#[test]
fn parse_script_header_size_reads_columns_and_lines() {
    let source =
        b"Script started on now [COMMAND=\"x\" COLUMNS=\"132\" LINES=\"48\"]\nrest of file\n";
    assert_eq!(parse_script_header_size(source), Some((132, 48)));
}

#[test]
fn parse_script_header_size_rejects_missing_or_invalid_dimensions() {
    assert_eq!(
        parse_script_header_size(b"Script started on now [COMMAND=\"x\"]\nrest\n"),
        None
    );
    assert_eq!(
        parse_script_header_size(
            b"Script started on now [COMMAND=\"x\" COLUMNS=\"-1\" LINES=\"40\"]\nrest\n"
        ),
        None
    );
}

#[test]
fn emulator_capture_is_incremental_and_resets_after_truncate() {
    let path = unique_log_path("incremental");
    fs::write(
        &path,
        b"Script started on now\n\x1b[32mhello\x1b[0m\nScript done on now\n",
    )
    .expect("log should write");

    let mut emulator = ZellijPreviewEmulator::default();
    let first = emulator
        .capture_from_log("grove-ws-test", &path, Some((80, 24)), 200)
        .expect("capture should succeed");
    assert!(first.contains("hello"));
    assert!(first.contains("\u{1b}[0;32m"));

    fs::write(
        &path,
        b"Script started on now\n\x1b[31mreset\x1b[0m\nScript done on now\n",
    )
    .expect("truncated log should write");
    let second = emulator
        .capture_from_log("grove-ws-test", &path, Some((80, 24)), 200)
        .expect("capture should succeed after truncate");
    assert!(second.contains("reset"));
    assert!(!second.contains("hello"));

    let _ = fs::remove_file(path);
}

#[test]
fn emulator_capture_reads_only_appended_bytes() {
    let path = unique_log_path("append");
    fs::write(&path, b"Script started on now\nline one\n").expect("initial log should write");

    let mut emulator = ZellijPreviewEmulator::default();
    let first = emulator
        .capture_from_log("grove-ws-append", &path, Some((80, 24)), 200)
        .expect("capture should succeed");
    assert!(first.contains("line one"));

    fs::write(&path, b"Script started on now\nline one\nline two\n")
        .expect("appended log should write");
    let second = emulator
        .capture_from_log("grove-ws-append", &path, Some((80, 24)), 200)
        .expect("capture should succeed");
    assert!(second.contains("line one"));
    assert!(second.contains("line two"));

    let _ = fs::remove_file(path);
}
