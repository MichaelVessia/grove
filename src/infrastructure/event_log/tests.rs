use super::{Event, EventLogger, FileEventLogger, NullEventLogger};
use serde_json::Value;
use std::collections::BTreeSet;
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
    drop(logger);

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

#[test]
fn event_schema_document_lists_required_common_fields_and_key_events() {
    let raw = fs::read_to_string("docs/observability/event-schema.json")
        .expect("event schema should exist");
    let parsed: Value = serde_json::from_str(&raw).expect("event schema should parse");
    assert_eq!(
        parsed["schema_version"].as_str(),
        Some("grove-event-schema-v1")
    );
    let common_fields = parsed["required_common_fields"]
        .as_array()
        .expect("required_common_fields should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<&str>>();
    assert!(common_fields.contains("run_id"));
    assert!(common_fields.contains("mono_ms"));
    assert!(common_fields.contains("event_seq"));
    assert!(common_fields.contains("msg_seq"));
    assert!(common_fields.contains("poll_generation"));
    assert!(common_fields.contains("frame_seq"));

    let events = parsed["events"]
        .as_array()
        .expect("events should be an array")
        .iter()
        .map(|entry| {
            let event = entry["event"]
                .as_str()
                .expect("event should be a string")
                .to_string();
            let kind = entry["kind"]
                .as_str()
                .expect("kind should be a string")
                .to_string();
            (event, kind)
        })
        .collect::<BTreeSet<(String, String)>>();
    assert!(events.contains(&(String::from("app"), String::from("session_started"))));
    assert!(events.contains(&(String::from("preview_poll"), String::from("cycle_started"))));
    assert!(events.contains(&(String::from("workspace_status"), String::from("transition"))));
    assert!(events.contains(&(
        String::from("input"),
        String::from("interactive_input_to_preview")
    )));
    assert!(events.contains(&(String::from("ui_command"), String::from("execute"))));
}
