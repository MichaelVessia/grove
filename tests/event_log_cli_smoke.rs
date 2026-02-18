use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_event_log_path(label: &str) -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-event-log-cli-smoke-{label}-{}-{timestamp}.jsonl",
        std::process::id()
    ))
}

fn unique_relative_event_log_name(label: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    format!(
        "event-log-cli-smoke-{label}-{}-{timestamp}.jsonl",
        std::process::id()
    )
}

#[test]
fn print_hello_with_event_log_creates_log_file() {
    let log_path = unique_event_log_path("print-hello");
    let _ = fs::remove_file(&log_path);

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--print-hello")
        .arg("--event-log")
        .arg(&log_path)
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");
    assert!(
        log_path.exists(),
        "event log file should be created when --event-log is provided"
    );

    let _ = fs::remove_file(log_path);
}

#[test]
fn print_hello_with_relative_event_log_creates_file_under_grove_directory() {
    let relative_name = unique_relative_event_log_name("relative");
    let log_path = std::path::PathBuf::from(".grove").join(&relative_name);
    let _ = fs::remove_file(&log_path);

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--print-hello")
        .arg("--event-log")
        .arg(&relative_name)
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");
    assert!(
        log_path.exists(),
        "relative --event-log should resolve under .grove/"
    );

    let _ = fs::remove_file(log_path);
}
