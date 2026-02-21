use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const GROVE_DIR: &str = ".grove";
const DEFAULT_SOCKET_FILE: &str = "groved.sock";
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonArgs {
    pub socket_path: PathBuf,
    pub once: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Pong { protocol_version: u32 },
}

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let parsed = parse_args(args)?;
    serve(parsed)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> std::io::Result<DaemonArgs> {
    let mut socket_path: Option<PathBuf> = None;
    let mut once = false;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--socket" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--socket requires a path",
                    ));
                };
                socket_path = Some(PathBuf::from(path));
            }
            "--once" => {
                once = true;
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown groved argument: {argument}"),
                ));
            }
        }
    }

    Ok(DaemonArgs {
        socket_path: socket_path.unwrap_or(default_socket_path()?),
        once,
    })
}

pub fn serve(args: DaemonArgs) -> std::io::Result<()> {
    ensure_socket_parent(&args.socket_path)?;
    let listener = bind_listener(&args.socket_path)?;

    for stream in listener.incoming() {
        let stream = stream?;
        let handled_request = handle_connection(stream)?;
        if args.once && handled_request {
            break;
        }
    }

    if args.once {
        remove_socket_if_exists(&args.socket_path)?;
    }

    Ok(())
}

fn ensure_socket_parent(socket_path: &Path) -> std::io::Result<()> {
    let Some(parent) = socket_path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
}

fn default_socket_path() -> std::io::Result<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "unable to resolve home directory for default socket path",
        ));
    };
    Ok(home.join(GROVE_DIR).join(DEFAULT_SOCKET_FILE))
}

fn bind_listener(socket_path: &Path) -> std::io::Result<UnixListener> {
    match UnixListener::bind(socket_path) {
        Ok(listener) => Ok(listener),
        Err(bind_error) if bind_error.kind() == std::io::ErrorKind::AddrInUse => {
            if !socket_path.exists() {
                return Err(bind_error);
            }

            if UnixStream::connect(socket_path).is_ok() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!("daemon already running at {}", socket_path.display()),
                ));
            }

            remove_socket_if_exists(socket_path)?;
            UnixListener::bind(socket_path)
        }
        Err(bind_error) => Err(bind_error),
    }
}

fn remove_socket_if_exists(socket_path: &Path) -> std::io::Result<()> {
    match fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn handle_connection(mut stream: UnixStream) -> std::io::Result<bool> {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(stream.try_clone()?);
        let bytes_read = reader.read_line(&mut request_line)?;
        if bytes_read == 0 {
            return Ok(false);
        }
    }

    let request: DaemonRequest = serde_json::from_str(request_line.trim()).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid request: {error}"),
        )
    })?;

    let response = match request {
        DaemonRequest::Ping => DaemonResponse::Pong {
            protocol_version: PROTOCOL_VERSION,
        },
    };

    let payload = serde_json::to_string(&response)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    stream.write_all(payload.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_socket_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "groved-test-{label}-{}-{timestamp}.sock",
            process::id()
        ))
    }

    #[test]
    fn parse_args_reads_socket_path_and_once_flag() {
        let parsed = parse_args([
            "--socket".to_string(),
            "/tmp/groved.sock".to_string(),
            "--once".to_string(),
        ])
        .expect("args should parse");

        assert_eq!(parsed.socket_path, PathBuf::from("/tmp/groved.sock"));
        assert!(parsed.once);
    }

    #[test]
    fn parse_args_rejects_unknown_flag() {
        let error = parse_args(["--unknown".to_string()]).expect_err("parse should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn bind_listener_replaces_stale_socket_path() {
        let socket_path = unique_temp_socket_path("stale");
        fs::write(&socket_path, "stale").expect("stale socket marker should be written");

        let listener =
            bind_listener(&socket_path).expect("listener should bind after stale cleanup");
        drop(listener);
        remove_socket_if_exists(&socket_path).expect("socket file cleanup should succeed");
    }

    #[test]
    fn bind_listener_keeps_active_socket_intact() {
        let socket_path = unique_temp_socket_path("active");
        let active_listener = UnixListener::bind(&socket_path).expect("first listener should bind");

        let error = bind_listener(&socket_path).expect_err("second bind should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);

        drop(active_listener);
        remove_socket_if_exists(&socket_path).expect("socket file cleanup should succeed");
    }
}
