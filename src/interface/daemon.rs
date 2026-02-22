use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::application::commands::{
    AgentStartRequest, AgentStopRequest, ErrorCode as CommandErrorCode,
    InProcessLifecycleCommandService, LifecycleCommandService, RepoContext, WorkspaceCreateRequest,
    WorkspaceDeleteRequest, WorkspaceEditRequest, WorkspaceListRequest, WorkspaceMergeRequest,
    WorkspaceSelector, WorkspaceUpdateRequest,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};

const GROVE_DIR: &str = ".grove";
const DEFAULT_SOCKET_FILE: &str = "groved.sock";
pub const PROTOCOL_VERSION: u32 = 2;
const DAEMON_CLIENT_LOG_PATH_ENV: &str = "GROVE_DAEMON_CLIENT_LOG_PATH";
static DAEMON_CLIENT_LOG_PATH_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static DAEMON_CLIENT_LOG_WRITER: OnceLock<Mutex<DaemonClientLogWriter>> = OnceLock::new();

#[derive(Default)]
struct DaemonClientLogWriter {
    path: Option<PathBuf>,
    writer: Option<BufWriter<File>>,
}

pub fn set_daemon_client_log_path(path: Option<PathBuf>) {
    let state = DAEMON_CLIENT_LOG_PATH_OVERRIDE.get_or_init(|| Mutex::new(None));
    let Ok(mut state) = state.lock() else {
        return;
    };
    *state = path;
}

fn daemon_client_log_path() -> Option<PathBuf> {
    let override_state = DAEMON_CLIENT_LOG_PATH_OVERRIDE.get_or_init(|| Mutex::new(None));
    if let Ok(override_path) = override_state.lock()
        && override_path.is_some()
    {
        return override_path.clone();
    }
    daemon_client_log_path_from_env(std::env::var_os(DAEMON_CLIENT_LOG_PATH_ENV).as_deref())
}

fn daemon_client_log_path_from_env(value: Option<&OsStr>) -> Option<PathBuf> {
    let value = value.and_then(OsStr::to_str)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn open_daemon_client_log_writer(path: &Path) -> Option<BufWriter<File>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    Some(BufWriter::new(file))
}

fn daemon_write_client_log_line(text: &str) {
    let log_path = daemon_client_log_path();
    let state =
        DAEMON_CLIENT_LOG_WRITER.get_or_init(|| Mutex::new(DaemonClientLogWriter::default()));
    let Ok(mut state) = state.lock() else {
        return;
    };

    if state.path != log_path {
        state.path = log_path.clone();
        state.writer = log_path.as_deref().and_then(open_daemon_client_log_writer);
    }
    let Some(writer) = state.writer.as_mut() else {
        return;
    };

    if writer.write_all(text.as_bytes()).is_err() {
        return;
    }
    if writer.write_all(b"\n").is_err() {
        return;
    }
    let _ = writer.flush();
}

fn daemon_log_event(event: &str, kind: &str, fields: impl IntoIterator<Item = (String, Value)>) {
    let mut data = Map::new();
    for (key, value) in fields {
        data.insert(key, value);
    }
    let line = json!({
        "ts": crate::infrastructure::event_log::now_millis(),
        "event": event,
        "kind": kind,
        "data": data,
    });
    let Ok(text) = serde_json::to_string(&line) else {
        return;
    };
    if kind.starts_with("client_") {
        daemon_write_client_log_line(&text);
        return;
    }
    eprintln!("{text}");
}

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
    WorkspaceEdit {
        payload: DaemonWorkspaceEditPayload,
    },
    WorkspaceDelete {
        payload: DaemonWorkspaceDeletePayload,
    },
    WorkspaceMerge {
        payload: DaemonWorkspaceMergePayload,
    },
    WorkspaceUpdate {
        payload: DaemonWorkspaceUpdatePayload,
    },
    AgentStart {
        payload: DaemonAgentStartPayload,
    },
    AgentStop {
        payload: DaemonAgentStopPayload,
    },
    SessionLaunch {
        payload: DaemonSessionLaunchPayload,
    },
    SessionCapture {
        payload: DaemonSessionCapturePayload,
    },
    SessionCursorMetadata {
        payload: DaemonSessionCursorMetadataPayload,
    },
    SessionResize {
        payload: DaemonSessionResizePayload,
    },
    SessionSendKeys {
        payload: DaemonSessionSendKeysPayload,
    },
    SessionPasteBuffer {
        payload: DaemonSessionPasteBufferPayload,
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
    WorkspaceEditOk {
        result: DaemonWorkspaceMutationResult,
    },
    WorkspaceEditErr {
        error: DaemonCommandError,
    },
    WorkspaceDeleteOk {
        result: DaemonWorkspaceMutationResult,
    },
    WorkspaceDeleteErr {
        error: DaemonCommandError,
    },
    WorkspaceMergeOk {
        result: DaemonWorkspaceMutationResult,
    },
    WorkspaceMergeErr {
        error: DaemonCommandError,
    },
    WorkspaceUpdateOk {
        result: DaemonWorkspaceMutationResult,
    },
    WorkspaceUpdateErr {
        error: DaemonCommandError,
    },
    AgentStartOk {
        result: DaemonWorkspaceMutationResult,
    },
    AgentStartErr {
        error: DaemonCommandError,
    },
    AgentStopOk {
        result: DaemonWorkspaceMutationResult,
    },
    AgentStopErr {
        error: DaemonCommandError,
    },
    SessionLaunchOk,
    SessionLaunchErr {
        error: DaemonCommandError,
    },
    SessionCaptureOk {
        output: String,
    },
    SessionCaptureErr {
        error: DaemonCommandError,
    },
    SessionCursorMetadataOk {
        metadata: String,
    },
    SessionCursorMetadataErr {
        error: DaemonCommandError,
    },
    SessionResizeOk,
    SessionResizeErr {
        error: DaemonCommandError,
    },
    SessionSendKeysOk,
    SessionSendKeysErr {
        error: DaemonCommandError,
    },
    SessionPasteBufferOk,
    SessionPasteBufferErr {
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
pub struct DaemonWorkspaceEditPayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub agent: Option<String>,
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceDeletePayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub delete_branch: bool,
    pub force_stop: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceMergePayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub cleanup_workspace: bool,
    pub cleanup_branch: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWorkspaceUpdatePayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonAgentStartPayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub prompt: Option<String>,
    pub pre_launch_command: Option<String>,
    pub skip_permissions: bool,
    pub dry_run: bool,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonAgentStopPayload {
    pub repo_root: String,
    pub workspace: Option<String>,
    pub workspace_path: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionLaunchPayload {
    pub session_name: String,
    pub workspace_path: String,
    pub command: String,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionCapturePayload {
    pub session_name: String,
    pub scrollback_lines: u16,
    pub include_escape_sequences: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionCursorMetadataPayload {
    pub session_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionResizePayload {
    pub session_name: String,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionSendKeysPayload {
    pub command: Vec<String>,
    #[serde(default)]
    pub fire_and_forget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionPasteBufferPayload {
    pub session_name: String,
    pub text: String,
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
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace list",
        )),
    }
}

pub fn ping_via_socket(socket_path: &Path) -> std::io::Result<u32> {
    let response = send_request(socket_path, &DaemonRequest::Ping)?;

    match response {
        DaemonResponse::Pong { protocol_version } => Ok(protocol_version),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for ping",
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
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace create",
        )),
    }
}

pub fn workspace_edit_via_socket(
    socket_path: &Path,
    payload: DaemonWorkspaceEditPayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceEdit { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceEditOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceEditErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace edit",
        )),
    }
}

pub fn workspace_delete_via_socket(
    socket_path: &Path,
    payload: DaemonWorkspaceDeletePayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceDelete { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceDeleteOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceDeleteErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace delete",
        )),
    }
}

pub fn workspace_merge_via_socket(
    socket_path: &Path,
    payload: DaemonWorkspaceMergePayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceMerge { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceMergeOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceMergeErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace merge",
        )),
    }
}

pub fn workspace_update_via_socket(
    socket_path: &Path,
    payload: DaemonWorkspaceUpdatePayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::WorkspaceUpdate { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::WorkspaceUpdateOk { result } => Ok(Ok(result)),
        DaemonResponse::WorkspaceUpdateErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for workspace update",
        )),
    }
}

pub fn agent_start_via_socket(
    socket_path: &Path,
    payload: DaemonAgentStartPayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::AgentStart { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::AgentStartOk { result } => Ok(Ok(result)),
        DaemonResponse::AgentStartErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for agent start",
        )),
    }
}

pub fn agent_stop_via_socket(
    socket_path: &Path,
    payload: DaemonAgentStopPayload,
) -> std::io::Result<Result<DaemonWorkspaceMutationResult, DaemonCommandError>> {
    let request = DaemonRequest::AgentStop { payload };
    let response = send_request(socket_path, &request)?;

    match response {
        DaemonResponse::AgentStopOk { result } => Ok(Ok(result)),
        DaemonResponse::AgentStopErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for agent stop",
        )),
    }
}

pub fn session_launch_via_socket(
    socket_path: &Path,
    payload: DaemonSessionLaunchPayload,
) -> std::io::Result<Result<(), DaemonCommandError>> {
    let request = DaemonRequest::SessionLaunch { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionLaunchOk => Ok(Ok(())),
        DaemonResponse::SessionLaunchErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session launch",
        )),
    }
}

pub fn session_capture_via_socket(
    socket_path: &Path,
    payload: DaemonSessionCapturePayload,
) -> std::io::Result<Result<String, DaemonCommandError>> {
    let request = DaemonRequest::SessionCapture { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionCaptureOk { output } => Ok(Ok(output)),
        DaemonResponse::SessionCaptureErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session capture",
        )),
    }
}

pub fn session_cursor_metadata_via_socket(
    socket_path: &Path,
    payload: DaemonSessionCursorMetadataPayload,
) -> std::io::Result<Result<String, DaemonCommandError>> {
    let request = DaemonRequest::SessionCursorMetadata { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionCursorMetadataOk { metadata } => Ok(Ok(metadata)),
        DaemonResponse::SessionCursorMetadataErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session cursor metadata",
        )),
    }
}

pub fn session_resize_via_socket(
    socket_path: &Path,
    payload: DaemonSessionResizePayload,
) -> std::io::Result<Result<(), DaemonCommandError>> {
    let request = DaemonRequest::SessionResize { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionResizeOk => Ok(Ok(())),
        DaemonResponse::SessionResizeErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session resize",
        )),
    }
}

pub fn session_send_keys_via_socket(
    socket_path: &Path,
    payload: DaemonSessionSendKeysPayload,
) -> std::io::Result<Result<(), DaemonCommandError>> {
    let request = DaemonRequest::SessionSendKeys { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionSendKeysOk => Ok(Ok(())),
        DaemonResponse::SessionSendKeysErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session send keys",
        )),
    }
}

pub fn session_paste_buffer_via_socket(
    socket_path: &Path,
    payload: DaemonSessionPasteBufferPayload,
) -> std::io::Result<Result<(), DaemonCommandError>> {
    let request = DaemonRequest::SessionPasteBuffer { payload };
    let response = send_request(socket_path, &request)?;
    match response {
        DaemonResponse::SessionPasteBufferOk => Ok(Ok(())),
        DaemonResponse::SessionPasteBufferErr { error } => Ok(Err(error)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected daemon response for session paste buffer",
        )),
    }
}

fn daemon_request_kind(request: &DaemonRequest) -> &'static str {
    match request {
        DaemonRequest::Ping => "ping",
        DaemonRequest::WorkspaceList { .. } => "workspace_list",
        DaemonRequest::WorkspaceCreate { .. } => "workspace_create",
        DaemonRequest::WorkspaceEdit { .. } => "workspace_edit",
        DaemonRequest::WorkspaceDelete { .. } => "workspace_delete",
        DaemonRequest::WorkspaceMerge { .. } => "workspace_merge",
        DaemonRequest::WorkspaceUpdate { .. } => "workspace_update",
        DaemonRequest::AgentStart { .. } => "agent_start",
        DaemonRequest::AgentStop { .. } => "agent_stop",
        DaemonRequest::SessionLaunch { .. } => "session_launch",
        DaemonRequest::SessionCapture { .. } => "session_capture",
        DaemonRequest::SessionCursorMetadata { .. } => "session_cursor_metadata",
        DaemonRequest::SessionResize { .. } => "session_resize",
        DaemonRequest::SessionSendKeys { .. } => "session_send_keys",
        DaemonRequest::SessionPasteBuffer { .. } => "session_paste_buffer",
    }
}

fn daemon_response_kind(response: &DaemonResponse) -> &'static str {
    match response {
        DaemonResponse::Pong { .. } => "pong",
        DaemonResponse::WorkspaceListOk { .. } => "workspace_list_ok",
        DaemonResponse::WorkspaceListErr { .. } => "workspace_list_err",
        DaemonResponse::WorkspaceCreateOk { .. } => "workspace_create_ok",
        DaemonResponse::WorkspaceCreateErr { .. } => "workspace_create_err",
        DaemonResponse::WorkspaceEditOk { .. } => "workspace_edit_ok",
        DaemonResponse::WorkspaceEditErr { .. } => "workspace_edit_err",
        DaemonResponse::WorkspaceDeleteOk { .. } => "workspace_delete_ok",
        DaemonResponse::WorkspaceDeleteErr { .. } => "workspace_delete_err",
        DaemonResponse::WorkspaceMergeOk { .. } => "workspace_merge_ok",
        DaemonResponse::WorkspaceMergeErr { .. } => "workspace_merge_err",
        DaemonResponse::WorkspaceUpdateOk { .. } => "workspace_update_ok",
        DaemonResponse::WorkspaceUpdateErr { .. } => "workspace_update_err",
        DaemonResponse::AgentStartOk { .. } => "agent_start_ok",
        DaemonResponse::AgentStartErr { .. } => "agent_start_err",
        DaemonResponse::AgentStopOk { .. } => "agent_stop_ok",
        DaemonResponse::AgentStopErr { .. } => "agent_stop_err",
        DaemonResponse::SessionLaunchOk => "session_launch_ok",
        DaemonResponse::SessionLaunchErr { .. } => "session_launch_err",
        DaemonResponse::SessionCaptureOk { .. } => "session_capture_ok",
        DaemonResponse::SessionCaptureErr { .. } => "session_capture_err",
        DaemonResponse::SessionCursorMetadataOk { .. } => "session_cursor_metadata_ok",
        DaemonResponse::SessionCursorMetadataErr { .. } => "session_cursor_metadata_err",
        DaemonResponse::SessionResizeOk => "session_resize_ok",
        DaemonResponse::SessionResizeErr { .. } => "session_resize_err",
        DaemonResponse::SessionSendKeysOk => "session_send_keys_ok",
        DaemonResponse::SessionSendKeysErr { .. } => "session_send_keys_err",
        DaemonResponse::SessionPasteBufferOk => "session_paste_buffer_ok",
        DaemonResponse::SessionPasteBufferErr { .. } => "session_paste_buffer_err",
    }
}

fn send_request(socket_path: &Path, request: &DaemonRequest) -> std::io::Result<DaemonResponse> {
    let request_kind = daemon_request_kind(request);
    let total_started_at = Instant::now();
    let connect_started_at = Instant::now();
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(error) => {
            daemon_log_event(
                "daemon_request",
                "client_failed",
                [
                    ("request".to_string(), Value::from(request_kind)),
                    ("stage".to_string(), Value::from("connect")),
                    ("error".to_string(), Value::from(error.to_string())),
                    (
                        "duration_ms".to_string(),
                        Value::from(
                            u64::try_from(
                                Instant::now()
                                    .saturating_duration_since(total_started_at)
                                    .as_millis(),
                            )
                            .unwrap_or(u64::MAX),
                        ),
                    ),
                ],
            );
            return Err(error);
        }
    };
    let connect_ms = u64::try_from(
        Instant::now()
            .saturating_duration_since(connect_started_at)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let request_json =
        serde_json::to_string(request).map_err(|error| std::io::Error::other(error.to_string()))?;

    let write_started_at = Instant::now();
    if let Err(error) = stream.write_all(request_json.as_bytes()) {
        daemon_log_event(
            "daemon_request",
            "client_failed",
            [
                ("request".to_string(), Value::from(request_kind)),
                ("stage".to_string(), Value::from("write_request")),
                ("error".to_string(), Value::from(error.to_string())),
            ],
        );
        return Err(error);
    }
    if let Err(error) = stream.write_all(b"\n") {
        daemon_log_event(
            "daemon_request",
            "client_failed",
            [
                ("request".to_string(), Value::from(request_kind)),
                ("stage".to_string(), Value::from("write_newline")),
                ("error".to_string(), Value::from(error.to_string())),
            ],
        );
        return Err(error);
    }
    if let Err(error) = stream.flush() {
        daemon_log_event(
            "daemon_request",
            "client_failed",
            [
                ("request".to_string(), Value::from(request_kind)),
                ("stage".to_string(), Value::from("flush_request")),
                ("error".to_string(), Value::from(error.to_string())),
            ],
        );
        return Err(error);
    }
    let write_ms = u64::try_from(
        Instant::now()
            .saturating_duration_since(write_started_at)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);

    let mut response_line = String::new();
    let mut reader = BufReader::new(stream);
    let read_started_at = Instant::now();
    let bytes_read = match reader.read_line(&mut response_line) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            daemon_log_event(
                "daemon_request",
                "client_failed",
                [
                    ("request".to_string(), Value::from(request_kind)),
                    ("stage".to_string(), Value::from("read_response")),
                    ("error".to_string(), Value::from(error.to_string())),
                ],
            );
            return Err(error);
        }
    };
    let read_ms = u64::try_from(
        Instant::now()
            .saturating_duration_since(read_started_at)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    if bytes_read == 0 {
        let error = std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "daemon closed socket before writing a response",
        );
        daemon_log_event(
            "daemon_request",
            "client_failed",
            [
                ("request".to_string(), Value::from(request_kind)),
                ("stage".to_string(), Value::from("response_eof")),
                ("error".to_string(), Value::from(error.to_string())),
            ],
        );
        return Err(error);
    }

    let parse_started_at = Instant::now();
    let response = serde_json::from_str(response_line.trim())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    let parse_ms = u64::try_from(
        Instant::now()
            .saturating_duration_since(parse_started_at)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    let total_ms = u64::try_from(
        Instant::now()
            .saturating_duration_since(total_started_at)
            .as_millis(),
    )
    .unwrap_or(u64::MAX);
    daemon_log_event(
        "daemon_request",
        "client_completed",
        [
            ("request".to_string(), Value::from(request_kind)),
            (
                "response".to_string(),
                Value::from(daemon_response_kind(&response)),
            ),
            ("connect_ms".to_string(), Value::from(connect_ms)),
            ("write_ms".to_string(), Value::from(write_ms)),
            ("read_ms".to_string(), Value::from(read_ms)),
            ("parse_ms".to_string(), Value::from(parse_ms)),
            ("total_ms".to_string(), Value::from(total_ms)),
        ],
    );
    Ok(response)
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

    if args.once {
        let service = InProcessLifecycleCommandService::new();
        for stream in listener.incoming() {
            let stream = stream?;
            let handled_request = handle_connection(stream, &service)?;
            if handled_request {
                break;
            }
        }
        remove_socket_if_exists(&args.socket_path)?;
    } else {
        for stream in listener.incoming() {
            let stream = stream?;
            std::thread::spawn(move || {
                let service = InProcessLifecycleCommandService::new();
                if let Err(error) = handle_connection(stream, &service) {
                    daemon_log_event(
                        "daemon_request",
                        "server_connection_handler_failed",
                        [("error".to_string(), Value::from(error.to_string()))],
                    );
                }
            });
        }
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

/// Returns `(session, text)` for a literal send-keys command (6-element vec
/// with `-l` at index 2). Returns `None` for named keys or malformed commands.
fn parse_literal_send_keys(command: &[String]) -> Option<(&str, &str)> {
    if command.len() == 6
        && command[0] == "tmux"
        && command[1] == "send-keys"
        && command[2] == "-l"
        && command[3] == "-t"
    {
        Some((&command[4], &command[5]))
    } else {
        None
    }
}

fn write_response(writer: &mut UnixStream, response: &DaemonResponse) -> std::io::Result<()> {
    let payload = serde_json::to_string(response)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    writer.write_all(payload.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Reads buffered lines from `reader` while they are literal send-keys to the
/// same `session`, appending text to `combined_text`. Returns the first
/// non-coalescable request as overflow (or `None` if the buffer is exhausted).
fn drain_coalescable_literals(
    session: &str,
    combined_text: &mut String,
    reader: &mut BufReader<UnixStream>,
    line_buf: &mut String,
) -> std::io::Result<(Option<DaemonRequest>, u64)> {
    let mut merged_followups = 0u64;
    while reader.buffer().contains(&b'\n') {
        line_buf.clear();
        let bytes_read = reader.read_line(line_buf)?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<DaemonRequest>(trimmed) else {
            daemon_log_event(
                "daemon_request",
                "server_invalid_dropped",
                [
                    ("stage".to_string(), Value::from("coalesce")),
                    (
                        "request_preview".to_string(),
                        Value::from(trimmed.to_string()),
                    ),
                ],
            );
            continue;
        };
        if let DaemonRequest::SessionSendKeys { payload } = &request
            && let Some((sess, text)) = parse_literal_send_keys(&payload.command)
            && sess == session
        {
            combined_text.push_str(text);
            merged_followups = merged_followups.saturating_add(1);
            continue;
        }
        return Ok((Some(request), merged_followups));
    }
    Ok((None, merged_followups))
}

/// Attempts to coalesce a send-keys payload with subsequent buffered literals.
/// Returns the response and an optional overflow request that was read ahead
/// but could not be coalesced.
fn coalesce_send_keys(
    payload: DaemonSessionSendKeysPayload,
    reader: &mut BufReader<UnixStream>,
    line_buf: &mut String,
    service: &impl LifecycleCommandService,
) -> std::io::Result<(DaemonResponse, Option<DaemonRequest>, u64)> {
    if let Some((session, text)) = parse_literal_send_keys(&payload.command) {
        let session_owned = session.to_string();
        let mut combined_text = text.to_string();
        let (overflow, merged_followups) =
            drain_coalescable_literals(&session_owned, &mut combined_text, reader, line_buf)?;
        let coalesced_command = vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            session_owned,
            combined_text,
        ];
        let coalesced_payload = DaemonSessionSendKeysPayload {
            command: coalesced_command,
            fire_and_forget: payload.fire_and_forget,
        };
        let response = dispatch_request(
            DaemonRequest::SessionSendKeys {
                payload: coalesced_payload,
            },
            service,
        );
        Ok((response, overflow, merged_followups))
    } else {
        let response = dispatch_request(DaemonRequest::SessionSendKeys { payload }, service);
        Ok((response, None, 0))
    }
}

fn handle_connection(
    stream: UnixStream,
    service: &impl LifecycleCommandService,
) -> std::io::Result<bool> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut handled_any = false;
    let mut line_buf = String::new();
    let mut pending_request: Option<DaemonRequest> = None;

    loop {
        let request_started_at = Instant::now();
        let request = if let Some(overflow) = pending_request.take() {
            overflow
        } else {
            line_buf.clear();
            let bytes_read = reader.read_line(&mut line_buf)?;
            if bytes_read == 0 {
                break;
            }
            let trimmed = line_buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            handled_any = true;
            match serde_json::from_str::<DaemonRequest>(trimmed) {
                Ok(request) => request,
                Err(error) => {
                    daemon_log_event(
                        "daemon_request",
                        "server_invalid_dropped",
                        [
                            ("stage".to_string(), Value::from("read")),
                            ("error".to_string(), Value::from(error.to_string())),
                            (
                                "request_preview".to_string(),
                                Value::from(trimmed.to_string()),
                            ),
                        ],
                    );
                    continue;
                }
            }
        };

        handled_any = true;
        let request_kind = daemon_request_kind(&request);

        match request {
            DaemonRequest::SessionSendKeys { payload } => {
                let fire_and_forget = payload.fire_and_forget;
                let (response, overflow, merged_followups) =
                    coalesce_send_keys(payload, &mut reader, &mut line_buf, service)?;
                pending_request = overflow;
                let dispatch_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(request_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                let write_started_at = Instant::now();
                let mut write_error: Option<String> = None;
                if !fire_and_forget && let Err(error) = write_response(&mut writer, &response) {
                    write_error = Some(error.to_string());
                }
                let write_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(write_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                let total_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(request_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                let mut fields = vec![
                    ("request".to_string(), Value::from(request_kind)),
                    (
                        "response".to_string(),
                        Value::from(daemon_response_kind(&response)),
                    ),
                    ("dispatch_ms".to_string(), Value::from(dispatch_ms)),
                    ("write_ms".to_string(), Value::from(write_ms)),
                    ("total_ms".to_string(), Value::from(total_ms)),
                    ("fire_and_forget".to_string(), Value::from(fire_and_forget)),
                    (
                        "coalesced_count".to_string(),
                        Value::from(merged_followups.saturating_add(1)),
                    ),
                ];
                if let Some(error) = write_error {
                    fields.push(("write_error".to_string(), Value::from(error)));
                    daemon_log_event(
                        "daemon_request",
                        "server_completed_with_write_error",
                        fields,
                    );
                    break;
                }
                daemon_log_event("daemon_request", "server_completed", fields);
            }
            other => {
                let response = dispatch_request(other, service);
                let dispatch_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(request_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                let write_started_at = Instant::now();
                let write_result = write_response(&mut writer, &response);
                let write_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(write_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                let total_ms = u64::try_from(
                    Instant::now()
                        .saturating_duration_since(request_started_at)
                        .as_millis(),
                )
                .unwrap_or(u64::MAX);
                match write_result {
                    Ok(()) => daemon_log_event(
                        "daemon_request",
                        "server_completed",
                        [
                            ("request".to_string(), Value::from(request_kind)),
                            (
                                "response".to_string(),
                                Value::from(daemon_response_kind(&response)),
                            ),
                            ("dispatch_ms".to_string(), Value::from(dispatch_ms)),
                            ("write_ms".to_string(), Value::from(write_ms)),
                            ("total_ms".to_string(), Value::from(total_ms)),
                            ("fire_and_forget".to_string(), Value::from(false)),
                            ("coalesced_count".to_string(), Value::from(0)),
                        ],
                    ),
                    Err(error) => {
                        daemon_log_event(
                            "daemon_request",
                            "server_completed_with_write_error",
                            [
                                ("request".to_string(), Value::from(request_kind)),
                                (
                                    "response".to_string(),
                                    Value::from(daemon_response_kind(&response)),
                                ),
                                ("dispatch_ms".to_string(), Value::from(dispatch_ms)),
                                ("write_ms".to_string(), Value::from(write_ms)),
                                ("total_ms".to_string(), Value::from(total_ms)),
                                ("fire_and_forget".to_string(), Value::from(false)),
                                ("coalesced_count".to_string(), Value::from(0)),
                                ("write_error".to_string(), Value::from(error.to_string())),
                            ],
                        );
                        break;
                    }
                }
            }
        }
    }

    Ok(handled_any)
}

fn dispatch_request(
    request: DaemonRequest,
    service: &impl LifecycleCommandService,
) -> DaemonResponse {
    match request {
        DaemonRequest::Ping => DaemonResponse::Pong {
            protocol_version: PROTOCOL_VERSION,
        },
        DaemonRequest::WorkspaceList { repo_root } => {
            handle_workspace_list_request(service, PathBuf::from(repo_root))
        }
        DaemonRequest::WorkspaceCreate { payload } => {
            handle_workspace_create_request(service, payload)
        }
        DaemonRequest::WorkspaceEdit { payload } => handle_workspace_edit_request(service, payload),
        DaemonRequest::WorkspaceDelete { payload } => {
            handle_workspace_delete_request(service, payload)
        }
        DaemonRequest::WorkspaceMerge { payload } => {
            handle_workspace_merge_request(service, payload)
        }
        DaemonRequest::WorkspaceUpdate { payload } => {
            handle_workspace_update_request(service, payload)
        }
        DaemonRequest::AgentStart { payload } => handle_agent_start_request(service, payload),
        DaemonRequest::AgentStop { payload } => handle_agent_stop_request(service, payload),
        DaemonRequest::SessionLaunch { payload } => handle_session_launch_request(payload),
        DaemonRequest::SessionCapture { payload } => handle_session_capture_request(payload),
        DaemonRequest::SessionCursorMetadata { payload } => {
            handle_session_cursor_metadata_request(payload)
        }
        DaemonRequest::SessionResize { payload } => handle_session_resize_request(payload),
        DaemonRequest::SessionSendKeys { payload } => handle_session_send_keys_request(payload),
        DaemonRequest::SessionPasteBuffer { payload } => {
            handle_session_paste_buffer_request(payload)
        }
    }
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

fn handle_workspace_edit_request(
    service: &impl LifecycleCommandService,
    payload: DaemonWorkspaceEditPayload,
) -> DaemonResponse {
    let parsed_agent = match parse_agent_from_request(payload.agent) {
        Ok(agent) => agent,
        Err(error) => {
            return DaemonResponse::WorkspaceEditErr { error };
        }
    };
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::WorkspaceEditErr { error };
            }
        };

    let request = WorkspaceEditRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        agent: parsed_agent,
        base_branch: payload.base_branch,
    };

    match service.workspace_edit(request) {
        Ok(response) => DaemonResponse::WorkspaceEditOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::WorkspaceEditErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_workspace_delete_request(
    service: &impl LifecycleCommandService,
    payload: DaemonWorkspaceDeletePayload,
) -> DaemonResponse {
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::WorkspaceDeleteErr { error };
            }
        };

    let request = WorkspaceDeleteRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        delete_branch: payload.delete_branch,
        force_stop: payload.force_stop,
        dry_run: payload.dry_run,
    };

    match service.workspace_delete(request) {
        Ok(response) => DaemonResponse::WorkspaceDeleteOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::WorkspaceDeleteErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_workspace_merge_request(
    service: &impl LifecycleCommandService,
    payload: DaemonWorkspaceMergePayload,
) -> DaemonResponse {
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::WorkspaceMergeErr { error };
            }
        };

    let request = WorkspaceMergeRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        cleanup_workspace: payload.cleanup_workspace,
        cleanup_branch: payload.cleanup_branch,
        dry_run: payload.dry_run,
    };

    match service.workspace_merge(request) {
        Ok(response) => DaemonResponse::WorkspaceMergeOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::WorkspaceMergeErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_workspace_update_request(
    service: &impl LifecycleCommandService,
    payload: DaemonWorkspaceUpdatePayload,
) -> DaemonResponse {
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::WorkspaceUpdateErr { error };
            }
        };

    let request = WorkspaceUpdateRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        dry_run: payload.dry_run,
    };

    match service.workspace_update(request) {
        Ok(response) => DaemonResponse::WorkspaceUpdateOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::WorkspaceUpdateErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_agent_start_request(
    service: &impl LifecycleCommandService,
    payload: DaemonAgentStartPayload,
) -> DaemonResponse {
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::AgentStartErr { error };
            }
        };

    let request = AgentStartRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        workspace_hint: None,
        prompt: payload.prompt,
        pre_launch_command: payload.pre_launch_command,
        skip_permissions: payload.skip_permissions,
        capture_cols: payload.capture_cols,
        capture_rows: payload.capture_rows,
        dry_run: payload.dry_run,
    };

    match service.agent_start(request) {
        Ok(response) => DaemonResponse::AgentStartOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::AgentStartErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_agent_stop_request(
    service: &impl LifecycleCommandService,
    payload: DaemonAgentStopPayload,
) -> DaemonResponse {
    let selector =
        match parse_workspace_selector_from_request(payload.workspace, payload.workspace_path) {
            Ok(selector) => selector,
            Err(error) => {
                return DaemonResponse::AgentStopErr { error };
            }
        };

    let request = AgentStopRequest {
        context: RepoContext {
            repo_root: PathBuf::from(payload.repo_root),
        },
        selector,
        workspace_hint: None,
        dry_run: payload.dry_run,
    };

    match service.agent_stop(request) {
        Ok(response) => DaemonResponse::AgentStopOk {
            result: DaemonWorkspaceMutationResult {
                workspace: DaemonWorkspaceView::from_workspace(response.workspace),
                warnings: response.warnings,
            },
        },
        Err(error) => DaemonResponse::AgentStopErr {
            error: DaemonCommandError::from_command_error(error.code, error.message),
        },
    }
}

fn handle_session_launch_request(payload: DaemonSessionLaunchPayload) -> DaemonResponse {
    let request = crate::application::agent_runtime::ShellLaunchRequest {
        session_name: payload.session_name,
        workspace_path: PathBuf::from(payload.workspace_path),
        command: payload.command,
        capture_cols: payload.capture_cols,
        capture_rows: payload.capture_rows,
    };
    let (_, result) = crate::application::agent_runtime::execute_shell_launch_request_for_mode(
        &request,
        crate::application::agent_runtime::CommandExecutionMode::Process,
    );
    match result {
        Ok(()) => DaemonResponse::SessionLaunchOk,
        Err(error) => DaemonResponse::SessionLaunchErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error,
            },
        },
    }
}

fn handle_session_capture_request(payload: DaemonSessionCapturePayload) -> DaemonResponse {
    match crate::infrastructure::tmux::capture_session_output(
        &payload.session_name,
        usize::from(payload.scrollback_lines),
        payload.include_escape_sequences,
    ) {
        Ok(output) => DaemonResponse::SessionCaptureOk { output },
        Err(error) => DaemonResponse::SessionCaptureErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error.to_string(),
            },
        },
    }
}

fn handle_session_cursor_metadata_request(
    payload: DaemonSessionCursorMetadataPayload,
) -> DaemonResponse {
    match crate::infrastructure::tmux::capture_cursor_metadata(&payload.session_name) {
        Ok(metadata) => DaemonResponse::SessionCursorMetadataOk { metadata },
        Err(error) => DaemonResponse::SessionCursorMetadataErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error.to_string(),
            },
        },
    }
}

fn handle_session_resize_request(payload: DaemonSessionResizePayload) -> DaemonResponse {
    match crate::infrastructure::tmux::resize_session(
        &payload.session_name,
        payload.width,
        payload.height,
    ) {
        Ok(()) => DaemonResponse::SessionResizeOk,
        Err(error) => DaemonResponse::SessionResizeErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error.to_string(),
            },
        },
    }
}

fn handle_session_send_keys_request(payload: DaemonSessionSendKeysPayload) -> DaemonResponse {
    match crate::infrastructure::tmux::execute_command(&payload.command) {
        Ok(()) => DaemonResponse::SessionSendKeysOk,
        Err(error) => DaemonResponse::SessionSendKeysErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error.to_string(),
            },
        },
    }
}

fn handle_session_paste_buffer_request(payload: DaemonSessionPasteBufferPayload) -> DaemonResponse {
    match crate::infrastructure::tmux::paste_buffer(&payload.session_name, &payload.text) {
        Ok(()) => DaemonResponse::SessionPasteBufferOk,
        Err(error) => DaemonResponse::SessionPasteBufferErr {
            error: DaemonCommandError {
                code: "runtime_failure".to_string(),
                message: error.to_string(),
            },
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

fn parse_workspace_selector_from_request(
    workspace_name: Option<String>,
    workspace_path: Option<String>,
) -> Result<WorkspaceSelector, DaemonCommandError> {
    match (workspace_name, workspace_path) {
        (Some(name), Some(path)) => Ok(WorkspaceSelector::NameAndPath {
            name,
            path: PathBuf::from(path),
        }),
        (Some(name), None) => Ok(WorkspaceSelector::Name(name)),
        (None, Some(path)) => Ok(WorkspaceSelector::Path(PathBuf::from(path))),
        (None, None) => Err(DaemonCommandError {
            code: command_error_code_label(CommandErrorCode::InvalidArgument).to_string(),
            message: "workspace selector is required (--workspace or --workspace-path)".to_string(),
        }),
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
    use std::ffi::OsStr;
    use std::io::Read;
    use std::net::Shutdown;
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
    fn daemon_client_log_path_from_env_returns_none_for_missing_or_blank_values() {
        assert_eq!(daemon_client_log_path_from_env(None), None);
        assert_eq!(daemon_client_log_path_from_env(Some(OsStr::new(""))), None);
        assert_eq!(
            daemon_client_log_path_from_env(Some(OsStr::new("   "))),
            None
        );
    }

    #[test]
    fn daemon_client_log_path_from_env_trims_whitespace() {
        assert_eq!(
            daemon_client_log_path_from_env(Some(OsStr::new(" /tmp/grove-daemon.jsonl "))),
            Some(PathBuf::from("/tmp/grove-daemon.jsonl"))
        );
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

    #[test]
    fn handle_connection_ping_request_writes_pong_response() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        client
            .write_all(br#"{"type":"ping"}"#)
            .expect("request should write");
        client
            .write_all(b"\n")
            .expect("request newline should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown write should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("request should be handled");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        assert!(
            response.contains(r#""type":"pong""#),
            "expected pong response, got: {response}"
        );
    }

    #[test]
    fn handle_connection_invalid_request_is_non_fatal() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        client
            .write_all(br#"{"type":"ping"}x"#)
            .expect("invalid request should write");
        client
            .write_all(b"\n")
            .expect("request newline should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown write should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service)
            .expect("invalid request should not fail the daemon");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response read should succeed");
        assert!(
            response.is_empty(),
            "invalid request should not get a response, got: {response}"
        );
    }

    #[test]
    fn handle_connection_multiple_requests_on_one_stream() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        client
            .write_all(b"{\"type\":\"ping\"}\n{\"type\":\"ping\"}\n")
            .expect("requests should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown write should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("requests should be handled");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        let lines: Vec<&str> = response.trim().split('\n').collect();
        assert_eq!(
            lines.len(),
            2,
            "expected two response lines, got: {response}"
        );
        for line in &lines {
            assert!(
                line.contains(r#""type":"pong""#),
                "expected pong response, got: {line}"
            );
        }
    }

    #[test]
    fn handle_connection_invalid_then_valid_request() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        client
            .write_all(b"not-valid-json\n{\"type\":\"ping\"}\n")
            .expect("requests should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown write should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("requests should be handled");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        let lines: Vec<&str> = response.trim().split('\n').collect();
        assert_eq!(
            lines.len(),
            1,
            "expected one response line (invalid request gets no response), got: {response}"
        );
        assert!(
            lines[0].contains(r#""type":"pong""#),
            "expected pong response, got: {}",
            lines[0]
        );
    }

    #[test]
    fn handle_connection_empty_lines_are_skipped() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        client
            .write_all(b"\n\n{\"type\":\"ping\"}\n\n")
            .expect("requests should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown write should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("requests should be handled");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        let lines: Vec<&str> = response.trim().split('\n').collect();
        assert_eq!(lines.len(), 1, "expected one response, got: {response}");
        assert!(
            lines[0].contains(r#""type":"pong""#),
            "expected pong, got: {}",
            lines[0]
        );
    }

    #[test]
    fn parse_literal_send_keys_identifies_literal_form() {
        let command = vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "grove-ws-auth".to_string(),
            "a".to_string(),
        ];
        let result = parse_literal_send_keys(&command);
        assert_eq!(result, Some(("grove-ws-auth", "a")));
    }

    #[test]
    fn parse_literal_send_keys_returns_none_for_named_keys() {
        let command = vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-auth".to_string(),
            "Enter".to_string(),
        ];
        assert_eq!(parse_literal_send_keys(&command), None);
    }

    #[test]
    fn parse_literal_send_keys_rejects_wrong_structure() {
        assert_eq!(parse_literal_send_keys(&[]), None);

        let wrong_binary = vec![
            "notmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "sess".to_string(),
            "x".to_string(),
        ];
        assert_eq!(parse_literal_send_keys(&wrong_binary), None);

        let too_short = vec!["tmux".to_string(), "send-keys".to_string()];
        assert_eq!(parse_literal_send_keys(&too_short), None);
    }

    #[test]
    fn drain_coalescable_literals_merges_same_session() {
        let (client, server) = UnixStream::pair().expect("unix stream pair should create");

        let req_a = make_literal_send_keys_json("sess1", "a");
        let req_b = make_literal_send_keys_json("sess1", "b");
        let req_c = make_literal_send_keys_json("sess1", "c");
        let mut writer = client;
        writer
            .write_all(format!("{req_a}\n{req_b}\n{req_c}\n").as_bytes())
            .expect("write should succeed");
        writer
            .shutdown(Shutdown::Write)
            .expect("shutdown should succeed");

        let mut reader = BufReader::new(server);
        // Prime the BufReader (reads a chunk into its internal buffer)
        let mut prime = String::new();
        reader
            .read_line(&mut prime)
            .expect("prime read should succeed");

        let mut combined = "a".to_string();
        let mut line_buf = String::new();
        let (overflow, merged_followups) =
            drain_coalescable_literals("sess1", &mut combined, &mut reader, &mut line_buf)
                .expect("drain should succeed");

        assert_eq!(combined, "abc");
        assert!(overflow.is_none());
        assert_eq!(merged_followups, 2);
    }

    #[test]
    fn drain_coalescable_literals_stops_at_named_key() {
        let (client, server) = UnixStream::pair().expect("unix stream pair should create");

        let req_a = make_literal_send_keys_json("sess1", "a");
        let req_b = make_literal_send_keys_json("sess1", "b");
        let enter = make_named_send_keys_json("sess1", "Enter");
        let mut writer = client;
        writer
            .write_all(format!("{req_a}\n{req_b}\n{enter}\n").as_bytes())
            .expect("write should succeed");
        writer
            .shutdown(Shutdown::Write)
            .expect("shutdown should succeed");

        let mut reader = BufReader::new(server);
        let mut prime = String::new();
        reader
            .read_line(&mut prime)
            .expect("prime read should succeed");

        let mut combined = "a".to_string();
        let mut line_buf = String::new();
        let (overflow, merged_followups) =
            drain_coalescable_literals("sess1", &mut combined, &mut reader, &mut line_buf)
                .expect("drain should succeed");

        assert_eq!(combined, "ab");
        assert!(
            overflow.is_some(),
            "named key should be returned as overflow"
        );
        assert_eq!(merged_followups, 1);
    }

    #[test]
    fn drain_coalescable_literals_stops_at_different_session() {
        let (client, server) = UnixStream::pair().expect("unix stream pair should create");

        let req_a = make_literal_send_keys_json("sess1", "a");
        let req_b = make_literal_send_keys_json("sess1", "b");
        let req_other = make_literal_send_keys_json("sess2", "x");
        let mut writer = client;
        writer
            .write_all(format!("{req_a}\n{req_b}\n{req_other}\n").as_bytes())
            .expect("write should succeed");
        writer
            .shutdown(Shutdown::Write)
            .expect("shutdown should succeed");

        let mut reader = BufReader::new(server);
        let mut prime = String::new();
        reader
            .read_line(&mut prime)
            .expect("prime read should succeed");

        let mut combined = "a".to_string();
        let mut line_buf = String::new();
        let (overflow, merged_followups) =
            drain_coalescable_literals("sess1", &mut combined, &mut reader, &mut line_buf)
                .expect("drain should succeed");

        assert_eq!(combined, "ab");
        assert!(
            overflow.is_some(),
            "different session should be returned as overflow"
        );
        assert_eq!(merged_followups, 1);
    }

    #[test]
    fn handle_connection_fire_and_forget_suppresses_response() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        let send_keys = make_literal_send_keys_json_with_faf("sess1", "a", true);
        let ping = r#"{"type":"ping"}"#;
        client
            .write_all(format!("{send_keys}\n{ping}\n").as_bytes())
            .expect("requests should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("should handle");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        let lines: Vec<&str> = response.trim().split('\n').collect();
        assert_eq!(
            lines.len(),
            1,
            "expected only pong (send-keys response suppressed), got: {response}"
        );
        assert!(lines[0].contains(r#""type":"pong""#));
    }

    #[test]
    fn handle_connection_backward_compat_missing_fire_and_forget() {
        let (mut client, server) = UnixStream::pair().expect("unix stream pair should create");
        // JSON without fire_and_forget field (backward compat: defaults to false)
        let send_keys = make_literal_send_keys_json("sess1", "a");
        client
            .write_all(format!("{send_keys}\n").as_bytes())
            .expect("requests should write");
        client
            .shutdown(Shutdown::Write)
            .expect("shutdown should succeed");

        let service = InProcessLifecycleCommandService::new();
        let handled = handle_connection(server, &service).expect("should handle");
        assert!(handled);

        let mut response = String::new();
        client
            .read_to_string(&mut response)
            .expect("response should read");
        let lines: Vec<&str> = response.trim().split('\n').collect();
        assert_eq!(
            lines.len(),
            1,
            "expected one response (fire_and_forget defaults false), got: {response}"
        );
        assert!(
            lines[0].contains(r#""type":"session_send_keys"#),
            "expected send_keys response, got: {}",
            lines[0]
        );
    }

    fn make_literal_send_keys_json(session: &str, text: &str) -> String {
        serde_json::to_string(&DaemonRequest::SessionSendKeys {
            payload: DaemonSessionSendKeysPayload {
                command: vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-l".to_string(),
                    "-t".to_string(),
                    session.to_string(),
                    text.to_string(),
                ],
                fire_and_forget: false,
            },
        })
        .expect("serialization should succeed")
    }

    fn make_literal_send_keys_json_with_faf(
        session: &str,
        text: &str,
        fire_and_forget: bool,
    ) -> String {
        serde_json::to_string(&DaemonRequest::SessionSendKeys {
            payload: DaemonSessionSendKeysPayload {
                command: vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-l".to_string(),
                    "-t".to_string(),
                    session.to_string(),
                    text.to_string(),
                ],
                fire_and_forget,
            },
        })
        .expect("serialization should succeed")
    }

    fn make_named_send_keys_json(session: &str, key_name: &str) -> String {
        serde_json::to_string(&DaemonRequest::SessionSendKeys {
            payload: DaemonSessionSendKeysPayload {
                command: vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    session.to_string(),
                    key_name.to_string(),
                ],
                fire_and_forget: false,
            },
        })
        .expect("serialization should succeed")
    }
}
