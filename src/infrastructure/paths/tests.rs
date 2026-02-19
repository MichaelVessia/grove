use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::refer_to_same_location;

#[derive(Debug)]
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "grove-paths-{label}-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test dir should be created");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn canonicalized_paths_compare_equal() {
    let temp = TestDir::new("canonicalized");
    let direct = temp.path.join("target");
    let nested = temp.path.join("nested");
    fs::create_dir_all(&direct).expect("target directory should exist");
    fs::create_dir_all(&nested).expect("nested directory should exist");
    let with_parent_segment = temp.path.join("nested/../target");

    assert!(refer_to_same_location(&direct, &with_parent_segment));
}

#[test]
fn raw_paths_compare_equal_when_canonicalize_fails() {
    let missing = PathBuf::from("/tmp/grove-paths-does-not-exist-a");
    let same_missing = PathBuf::from("/tmp/grove-paths-does-not-exist-a");

    assert!(refer_to_same_location(&missing, &same_missing));
}

#[test]
fn different_paths_compare_not_equal() {
    let left = PathBuf::from("/tmp/grove-paths-left");
    let right = PathBuf::from("/tmp/grove-paths-right");

    assert!(!refer_to_same_location(&left, &right));
}
