use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-debug-record-cli-{label}-{}-{timestamp}",
        std::process::id()
    ))
}

#[test]
fn print_hello_with_debug_record_creates_timestamped_file_in_dot_grove() {
    let cwd = unique_temp_dir("print-hello");
    fs::create_dir_all(&cwd).expect("temp cwd should exist");

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--print-hello")
        .arg("--debug-record")
        .current_dir(&cwd)
        .output()
        .expect("grove binary should run");

    assert!(output.status.success(), "binary exited non-zero");

    let debug_dir = cwd.join(".grove");
    assert!(debug_dir.is_dir(), ".grove directory should exist");

    let entries: Vec<_> = fs::read_dir(&debug_dir)
        .expect("debug dir should be readable")
        .filter_map(Result::ok)
        .collect();

    assert_eq!(entries.len(), 1, "expected exactly one debug record file");
    let file_name = entries[0].file_name();
    let file_name = file_name.to_string_lossy();
    assert!(
        file_name.starts_with("debug-record-") && file_name.ends_with(".jsonl"),
        "unexpected debug record filename: {file_name}"
    );

    let _ = fs::remove_dir_all(cwd);
}
