use super::{
    CliArgs, debug_record_path, ensure_event_log_parent_directory, parse_cli_args,
    resolve_event_log_path,
};
use std::path::PathBuf;

#[test]
fn cli_parser_reads_event_log_and_print_hello() {
    let parsed = parse_cli_args(vec![
        "--event-log".to_string(),
        "/tmp/events.jsonl".to_string(),
        "--print-hello".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        parsed,
        CliArgs {
            print_hello: true,
            event_log_path: Some(PathBuf::from("/tmp/events.jsonl")),
            debug_record: false,
            replay_trace_path: None,
            replay_snapshot_path: None,
            replay_emit_test_name: None,
            replay_invariant_only: false,
        }
    );
}

#[test]
fn cli_parser_requires_event_log_path() {
    let error = parse_cli_args(vec!["--event-log".to_string()])
        .expect_err("missing event log path should fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn cli_parser_reads_debug_record_flag() {
    let parsed =
        parse_cli_args(vec!["--debug-record".to_string()]).expect("debug flag should parse");
    assert_eq!(
        parsed,
        CliArgs {
            print_hello: false,
            event_log_path: None,
            debug_record: true,
            replay_trace_path: None,
            replay_snapshot_path: None,
            replay_emit_test_name: None,
            replay_invariant_only: false,
        }
    );
}

#[test]
fn cli_parser_reads_replay_options() {
    let parsed = parse_cli_args(vec![
        "replay".to_string(),
        "/tmp/debug-record.jsonl".to_string(),
        "--snapshot".to_string(),
        "/tmp/replay-snapshot.json".to_string(),
        "--emit-test".to_string(),
        "flow-a".to_string(),
        "--invariant-only".to_string(),
    ])
    .expect("replay arguments should parse");

    assert_eq!(
        parsed,
        CliArgs {
            print_hello: false,
            event_log_path: None,
            debug_record: false,
            replay_trace_path: Some(PathBuf::from("/tmp/debug-record.jsonl")),
            replay_snapshot_path: Some(PathBuf::from("/tmp/replay-snapshot.json")),
            replay_emit_test_name: Some("flow-a".to_string()),
            replay_invariant_only: true,
        }
    );
}

#[test]
fn cli_parser_rejects_replay_flags_without_replay_subcommand() {
    let error = parse_cli_args(vec!["--snapshot".to_string(), "/tmp/out.json".to_string()])
        .expect_err("replay-only flags without replay should fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn debug_record_path_uses_grove_directory_and_timestamp_prefix() {
    let app_start_ts = 1_771_023_000_555u64;
    let path = debug_record_path(app_start_ts).expect("path should resolve");
    let path_text = path.to_string_lossy();
    assert!(path_text.contains(".grove/"));
    assert!(path_text.contains(&format!("debug-record-{app_start_ts}")));
    let _ = std::fs::remove_file(path);
}

#[test]
fn resolve_event_log_path_places_relative_paths_under_grove_directory() {
    assert_eq!(
        resolve_event_log_path(PathBuf::from("events.jsonl")),
        PathBuf::from(".grove/events.jsonl")
    );
}

#[test]
fn resolve_event_log_path_keeps_absolute_paths_unchanged() {
    assert_eq!(
        resolve_event_log_path(PathBuf::from("/tmp/events.jsonl")),
        PathBuf::from("/tmp/events.jsonl")
    );
}

#[test]
fn resolve_event_log_path_keeps_grove_prefixed_relative_paths() {
    assert_eq!(
        resolve_event_log_path(PathBuf::from(".grove/custom/events.jsonl")),
        PathBuf::from(".grove/custom/events.jsonl")
    );
}

#[test]
fn ensure_event_log_parent_directory_creates_missing_directories() {
    let root = std::env::temp_dir().join(format!(
        "grove-main-tests-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos()
    ));
    let path = root.join(".grove/nested/events.jsonl");

    ensure_event_log_parent_directory(&path).expect("parent directory should be created");
    assert!(root.join(".grove/nested").exists());

    let _ = std::fs::remove_dir_all(root);
}
