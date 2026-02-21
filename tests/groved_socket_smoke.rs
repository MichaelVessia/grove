use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

fn unique_socket_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "groved-socket-smoke-{label}-{}-{timestamp}.sock",
        std::process::id()
    ))
}

fn wait_for_socket_ready(path: &Path) {
    for _ in 0..200 {
        if path.exists() && UnixStream::connect(path).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("socket was not ready at {}", path.display());
}

fn spawn_groved(socket_path: &Path, stale_file: bool) -> Child {
    if stale_file {
        std::fs::write(socket_path, "stale").expect("stale socket marker should be created");
    }

    Command::new(env!("CARGO_BIN_EXE_groved"))
        .arg("--socket")
        .arg(socket_path)
        .arg("--once")
        .spawn()
        .expect("groved should start")
}

fn ping_socket(socket_path: &Path) -> serde_json::Value {
    let mut stream = UnixStream::connect(socket_path).expect("should connect to groved socket");
    let request = json!({"type": "ping"});
    let request_json = serde_json::to_string(&request).expect("request should serialize");

    stream
        .write_all(format!("{request_json}\n").as_bytes())
        .expect("request should be sent");
    stream.flush().expect("request flush should succeed");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader
        .read_line(&mut line)
        .expect("response should be readable");

    serde_json::from_str(line.trim()).expect("response should be valid JSON")
}

fn await_success(child: &mut Child) {
    let status = child.wait().expect("groved should exit");
    assert!(status.success(), "groved exited with non-zero status");
}

#[test]
fn groved_replies_to_ping_request() {
    let socket_path = unique_socket_path("ping");
    let mut child = spawn_groved(&socket_path, false);
    wait_for_socket_ready(&socket_path);

    let response = ping_socket(&socket_path);

    await_success(&mut child);
    assert_eq!(
        response["type"],
        serde_json::Value::String("pong".to_string())
    );
    assert_eq!(response["protocol_version"], serde_json::Value::from(1));
}

#[test]
fn groved_recovers_from_stale_socket_file() {
    let socket_path = unique_socket_path("stale");
    let mut child = spawn_groved(&socket_path, true);
    wait_for_socket_ready(&socket_path);

    let response = ping_socket(&socket_path);

    await_success(&mut child);
    assert_eq!(
        response["type"],
        serde_json::Value::String("pong".to_string())
    );
    assert_eq!(response["protocol_version"], serde_json::Value::from(1));
}
