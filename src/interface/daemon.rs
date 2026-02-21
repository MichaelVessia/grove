use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::application::commands::{
    ErrorCode as CommandErrorCode, InProcessLifecycleCommandService, LifecycleCommandService,
    RepoContext, WorkspaceCreateRequest, WorkspaceListRequest,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};

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
    WorkspaceList {
        repo_root: String,
    },
    WorkspaceCreate {
        payload: DaemonWorkspaceCreatePayload,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Pong {
        protocol_version: u32,
    },
    WorkspaceListOk {
        result: DaemonWorkspaceListResult,
    },
    WorkspaceListErr {
        error: DaemonCommandError,
    },
    WorkspaceCreateOk {
        result: DaemonWorkspaceMutationResult,
    },
    WorkspaceCreateErr {
        error: DaemonCommandError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceListResult {
    pub repo_root: String,
    pub workspaces: Vec<DaemonWorkspaceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceMutationResult {
    pub workspace: DaemonWorkspaceView,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceCreatePayload {
    pub repo_root: String,
    pub name: String,
    pub base_branch: Option<String>,
    pub existing_branch: Option<String>,
    pub agent: Option<String>,
    pub start: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceView {
    pub name: String,
    pub path: String,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
    pub branch: String,
    pub base_branch: Option<String>,
    pub last_activity_unix_secs: Option<i64>,
    pub agent: String,
    pub status: String,
    pub is_main: bool,
    pub is_orphaned: bool,
    pub supported_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonCommandError {
    pub code: String,
    pub message: String,
}

impl DaemonWorkspaceView {
    fn from_workspace(workspace: Workspace) -> Self {
        Self {
            name: workspace.name,
            path: workspace.path.display().to_string(),
            project_name: workspace.project_name,
            project_path: workspace
                .project_path
                .map(|path| path.display().to_string()),
            branch: workspace.branch,
            base_branch: workspace.base_branch,
            last_activity_unix_secs: workspace.last_activity_unix_secs,
            agent: agent_label(workspace.agent).to_string(),
            status: workspace_status_label(workspace.status).to_string(),
            is_main: workspace.is_main,
            is_orphaned: workspace.is_orphaned,
            supported_agent: workspace.supported_agent,
        }
    }
}

impl DaemonCommandError {
    fn from_command_error(code: CommandErrorCode, message: String) -> Self {
        Self {
            code: command_error_code_label(code).to_string(),
            message,
        }
    }
}

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let parsed = parse_args(args)?;
    serve(parsed)
}

pub fn workspace_list_via_socket(
    socket_path: &Path,
    repo_root: &Path,
) -> std::io::Result<Result<DaemonWorkspaceListResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceList {
        repo_root: repo_root.display().to_string(),
    };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceListOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceListErr { error } => Ok(Err(error)),
        DaemonResponse::Pong { .. }
        | DaemonResponse::WorkspaceCreateOk { .. }
        | DaemonResponse::WorkspaceCreateErr { .. } => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace list",
        )),
    }
}

pub fn workspace_create_via_socket(
    socket_path: &Path,
    payload: DaemonWorkspaceCreatePayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceCreate { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceCreateOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceCreateErr { error } => Ok(Err(error)),
        DaemonResponse::Pong { .. }
        | DaemonResponse::WorkspaceListOk { .. }
        | DaemonResponse::WorkspaceListErr { .. } => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace create",
        )),
    }
}

fn send_request(socket_path: &Path, request: &DaemonRequest) -> std::io::Result<DaemonResponse> {
    let mut stream = UnixStream::connect(socket_path)?;
    let request_json =
        serde_json::to_string(request).map_err(|error| std::io::Error::other(error.to_string()))?;

    stream.write_all(request_json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response_line = String::new();
    let mut reader = BufReader::new(stream);
    let bytes_read = reader.read_line(&mut response_line)?;
    if bytes_read == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "daemon closed socket before writing a response",
        ));
    }

    serde_json::from_str(response_line.trim())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
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
    let service = InProcessLifecycleCommandService::new();

    for stream in listener.incoming() {
        let stream = stream?;
        let handled_request = handle_connection(stream, &service)?;
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

fn handle_connection(
    mut stream: UnixStream,
    service: &impl LifecycleCommandService,
) -> std::io::Result<bool> {
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
        DaemonRequest::WorkspaceList { repo_root } => {
            handle_workspace_list_request(service, PathBuf::from(repo_root))
        }
        DaemonRequest::WorkspaceCreate { payload } => {
            handle_workspace_create_request(service, payload)
        }
    };

    let payload = serde_json::to_string(&response)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    stream.write_all(payload.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(true)
}

fn handle_workspace_list_request(
    service: &impl LifecycleCommandService,
    repo_root: PathBuf,
) -> DaemonResponse {
    let repo_root_display = repo_root.display().to_string();
    let request = WorkspaceListRequest {
        context: RepoContext { repo_root },
    };

    match service.workspace_list(request) {
        Ok(response) => DaemonResponse::WorkspaceListOk {
            result: DaemonWorkspaceListResult {
                repo_root: repo_root_display,
                workspaces: response
                    .workspaces
                    .into_iter()
                    .map(DaemonWorkspaceView::from_workspace)
                    .collect(),
            },
        },
        Err(error) => DaemonResponse::WorkspaceListErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_workspace_create_request(
    service: &impl LifecycleCommandService,
    payload: DaemonWorkspaceCreatePayload,
) -> DaemonResponse {
    let parsed_agent = match parse_agent_from_request(payload.agent) {
        Ok(agent) => agent,
        Err(error) => {
            return DaemonResponse::WorkspaceCreateErr { error };
        }
    };

    let request = WorkspaceCreateRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        name: payload.name,
        base_branch: payload.base_branch,
        existing_branch: payload.existing_branch,
        agent: parsed_agent,
        start: payload.start,
        dry_run: payload.dry_run,
        setup_template: None,
    };

    match service.workspace_create(request) {
        Ok(response) => DaemonResponse::WorkspaceCreateOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::WorkspaceCreateErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn parse_agent_from_request(
    agent: Option<String>,
) -> Result<Option<AgentType>, DaemonCommandError> {
    match agent {
        None => Ok(None),
        Some(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "claude" => Ok(Some(AgentType::Claude)),
                "codex" => Ok(Some(AgentType::Codex)),
                _ => Err(DaemonCommandError {
                    code: command_error_code_label(CommandErrorCode::InvalidArgument).to_string(),
                    message: "--agent must be one of: claude, codex".to_string(),
                }),
            }
        }
    }
}

fn command_error_code_label(code: CommandErrorCode) -> &'static str {
    match code {
        CommandErrorCode::InvalidArgument => "invalid_argument",
        CommandErrorCode::NotFound => "not_found",
        CommandErrorCode::Conflict => "conflict",
        CommandErrorCode::RuntimeFailure => "runtime_failure",
        CommandErrorCode::Internal => "internal",
    }
}

fn agent_label(agent: AgentType) -> &'static str {
    match agent {
        AgentType::Claude => "claude",
        AgentType::Codex => "codex",
    }
}

fn workspace_status_label(status: WorkspaceStatus) -> &'static str {
    match status {
        WorkspaceStatus::Main => "main",
        WorkspaceStatus::Idle => "idle",
        WorkspaceStatus::Active => "active",
        WorkspaceStatus::Thinking => "thinking",
        WorkspaceStatus::Waiting => "waiting",
        WorkspaceStatus::Done => "done",
        WorkspaceStatus::Error => "error",
        WorkspaceStatus::Unknown => "unknown",
        WorkspaceStatus::Unsupported => "unsupported",
    }
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
