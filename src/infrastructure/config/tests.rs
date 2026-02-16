use super::{GroveConfig, MultiplexerKind, ProjectConfig, load_from_path, save_to_path};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("grove-config-{label}-{pid}-{timestamp}.toml"))
}

#[test]
fn missing_config_defaults_to_tmux() {
    let path = unique_temp_path("missing");
    let config = load_from_path(&path).expect("missing path should default");
    assert_eq!(
        config,
        GroveConfig {
            multiplexer: MultiplexerKind::Tmux,
            projects: Vec::new(),
        }
    );
}

#[test]
fn save_and_load_round_trip() {
    let path = unique_temp_path("roundtrip");
    let config = GroveConfig {
        multiplexer: MultiplexerKind::Tmux,
        projects: vec![ProjectConfig {
            name: "grove".to_string(),
            path: PathBuf::from("/repos/grove"),
        }],
    };
    save_to_path(&path, &config).expect("config should save");

    let loaded = load_from_path(&path).expect("config should load");
    assert_eq!(loaded, config);

    let _ = fs::remove_file(path);
}

#[test]
fn load_old_config_without_projects_defaults_to_empty_projects() {
    let path = unique_temp_path("legacy");
    fs::write(&path, "multiplexer = \"tmux\"\n").expect("fixture should write");

    let loaded = load_from_path(&path).expect("legacy config should load");
    assert_eq!(loaded.projects, Vec::<ProjectConfig>::new());

    let _ = fs::remove_file(path);
}
