mod support;

use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use support::EventLogReader;

fn unique_event_log_path(label: &str) -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-event-log-reader-{label}-{}-{timestamp}.jsonl",
        std::process::id()
    ))
}

#[test]
fn event_log_reader_waits_and_asserts_sequence() {
    let path = unique_event_log_path("sequence");
    fs::write(
        &path,
        "{\"kind\":\"dialog_opened\"}\n{\"kind\":\"dialog_confirmed\"}\n",
    )
    .expect("event log fixture should be writable");

    let reader = EventLogReader::open(path.clone());
    let found = reader
        .wait_for("dialog_confirmed", Duration::from_millis(100))
        .expect("reader should find kind");
    assert_eq!(found["kind"], serde_json::Value::from("dialog_confirmed"));
    reader
        .assert_sequence(&["dialog_opened", "dialog_confirmed"])
        .expect("sequence should match");

    let _ = fs::remove_file(path);
}
