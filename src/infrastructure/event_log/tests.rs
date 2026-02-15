use super::{Event, EventLogger, FileEventLogger, NullEventLogger};
use serde_json::Value;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_path(label: &str) -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-event-log-{label}-{}-{timestamp}.jsonl",
        std::process::id()
    ))
}

#[test]
fn file_event_logger_writes_ndjson() {
    let path = unique_path("writer");
    let logger = FileEventLogger::open(&path).expect("event log file should open");
    logger.log(Event::new("state_change", "selection_changed").with_data("index", Value::from(1)));

    let raw = fs::read_to_string(&path).expect("event log should be readable");
    assert!(!raw.trim().is_empty());
    let first_line = raw.lines().next().expect("first event line should exist");
    let json: Value = serde_json::from_str(first_line).expect("event line should be valid json");
    assert_eq!(json["event"], Value::from("state_change"));
    assert_eq!(json["kind"], Value::from("selection_changed"));
    assert_eq!(json["data"]["index"], Value::from(1));

    let _ = fs::remove_file(path);
}

#[test]
fn null_event_logger_is_noop() {
    let logger = NullEventLogger;
    logger.log(Event::new("test", "noop"));
}
