use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_socket_path(label: &str) -> PathBuf {
    let compact_label = label.chars().take(8).collect::<String>();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "gwsd-{compact_label}-{}-{timestamp}.sock",
        std::process::id()
    ))
}

fn wait_for_socket_ready(path: &Path) {
    for _ in 0..200 {
        if path.exists() && std::os::unix::net::UnixStream::connect(path).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("socket was not ready at {}", path.display());
}

fn spawn_groved(socket_path: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_groved"))
        .arg("--socket")
        .arg(socket_path)
        .arg("--once")
        .spawn()
        .expect("groved should start")
}

#[test]
fn workspace_list_can_use_daemon_socket_transport() {
    let socket_path = unique_socket_path("workspace-list");
    let mut daemon = spawn_groved(&socket_path);
    wait_for_socket_ready(&socket_path);

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--socket")
        .arg(&socket_path)
        .arg("workspace")
        .arg("list")
        .arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("grove should run");

    let status = daemon.wait().expect("daemon should exit");
    assert!(status.success(), "groved exited non-zero");
    assert!(output.status.success(), "grove exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace list".to_string())
    );
    assert!(value["result"]["workspaces"].is_array());
}

#[test]
fn socket_transport_rejects_unsupported_commands_for_now() {
    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--socket")
        .arg("/tmp/non-existent-groved.sock")
        .arg("workspace")
        .arg("delete")
        .arg("--workspace")
        .arg("feature-a")
        .arg("--dry-run")
        .output()
        .expect("grove should run");

    assert!(output.status.success(), "grove exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("INVALID_ARGUMENT".to_string())
    );
}

#[test]
fn workspace_edit_missing_workspace_can_use_daemon_socket_transport() {
    let socket_path = unique_socket_path("workspace-edit");
    let mut daemon = spawn_groved(&socket_path);
    wait_for_socket_ready(&socket_path);

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--socket")
        .arg(&socket_path)
        .arg("workspace")
        .arg("edit")
        .arg("--workspace")
        .arg("definitely-missing-workspace")
        .arg("--agent")
        .arg("codex")
        .arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("grove should run");

    let status = daemon.wait().expect("daemon should exit");
    assert!(status.success(), "groved exited non-zero");
    assert!(output.status.success(), "grove exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace edit".to_string())
    );
    assert_eq!(
        value["error"]["code"],
        serde_json::Value::String("WORKSPACE_NOT_FOUND".to_string())
    );
}

#[test]
fn workspace_create_dry_run_can_use_daemon_socket_transport() {
    let socket_path = unique_socket_path("workspace-create");
    let mut daemon = spawn_groved(&socket_path);
    wait_for_socket_ready(&socket_path);

    let output = Command::new(env!("CARGO_BIN_EXE_grove"))
        .arg("--socket")
        .arg(&socket_path)
        .arg("workspace")
        .arg("create")
        .arg("--name")
        .arg("feature-daemon-create")
        .arg("--base")
        .arg("main")
        .arg("--dry-run")
        .arg("--repo")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("grove should run");

    let status = daemon.wait().expect("daemon should exit");
    assert!(status.success(), "groved exited non-zero");
    assert!(output.status.success(), "grove exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");
    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    assert_eq!(
        value["command"],
        serde_json::Value::String("grove workspace create".to_string())
    );
    assert_eq!(value["result"]["dry_run"], serde_json::Value::Bool(true));
}
