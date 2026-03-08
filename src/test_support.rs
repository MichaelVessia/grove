use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates a unique temporary directory for test isolation.
///
/// Each call produces a distinct path by combining the given `prefix`, the
/// current process ID, and a nanosecond timestamp. The directory is created
/// on disk before returning.
pub fn unique_test_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", process::id()));
    std::fs::create_dir_all(&path).expect("test directory should be created");
    path
}
