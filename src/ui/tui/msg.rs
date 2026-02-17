use super::*;
use crate::application::agent_runtime::{SessionExecutionResult, WorkspaceStatusTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Msg {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(PasteEvent),
    Tick,
    Resize { width: u16, height: u16 },
    PreviewPollCompleted(PreviewPollCompletion),
    LazygitLaunchCompleted(LazygitLaunchCompletion),
    RefreshWorkspacesCompleted(RefreshWorkspacesCompletion),
    DeleteProjectCompleted(DeleteProjectCompletion),
    DeleteWorkspaceCompleted(DeleteWorkspaceCompletion),
    MergeWorkspaceCompleted(MergeWorkspaceCompletion),
    UpdateWorkspaceFromBaseCompleted(UpdateWorkspaceFromBaseCompletion),
    CreateWorkspaceCompleted(CreateWorkspaceCompletion),
    StartAgentCompleted(StartAgentCompletion),
    StopAgentCompleted(StopAgentCompletion),
    InteractiveSendCompleted(InteractiveSendCompletion),
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreviewPollCompletion {
    pub(super) generation: u64,
    pub(super) live_capture: Option<LivePreviewCapture>,
    pub(super) cursor_capture: Option<CursorCapture>,
    pub(super) workspace_status_captures: Vec<WorkspaceStatusCapture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LivePreviewCapture {
    pub(super) session: String,
    pub(super) include_escape_sequences: bool,
    pub(super) capture_ms: u64,
    pub(super) total_ms: u64,
    pub(super) result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LazygitLaunchCompletion {
    pub(super) session_name: String,
    pub(super) duration_ms: u64,
    pub(super) result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CursorCapture {
    pub(super) session: String,
    pub(super) capture_ms: u64,
    pub(super) result: Result<String, String>,
}

pub(super) type WorkspaceStatusPollTarget = WorkspaceStatusTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceStatusCapture {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) session_name: String,
    pub(super) supported_agent: bool,
    pub(super) capture_ms: u64,
    pub(super) result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RefreshWorkspacesCompletion {
    pub(super) preferred_workspace_path: Option<PathBuf>,
    pub(super) bootstrap: BootstrapData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteWorkspaceCompletion {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) result: Result<(), String>,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteProjectCompletion {
    pub(super) project_name: String,
    pub(super) project_path: PathBuf,
    pub(super) projects: Vec<ProjectConfig>,
    pub(super) result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MergeWorkspaceCompletion {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) workspace_branch: String,
    pub(super) base_branch: String,
    pub(super) result: Result<(), String>,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UpdateWorkspaceFromBaseCompletion {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) workspace_branch: String,
    pub(super) base_branch: String,
    pub(super) result: Result<(), String>,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateWorkspaceCompletion {
    pub(super) request: CreateWorkspaceRequest,
    pub(super) result: Result<CreateWorkspaceResult, WorkspaceLifecycleError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartAgentCompletion {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) session_name: String,
    pub(super) result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StopAgentCompletion {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) session_name: String,
    pub(super) result: Result<(), String>,
}

impl From<SessionExecutionResult> for StartAgentCompletion {
    fn from(result: SessionExecutionResult) -> Self {
        Self {
            workspace_name: result.workspace_name,
            workspace_path: result.workspace_path,
            session_name: result.session_name,
            result: result.result,
        }
    }
}

impl From<SessionExecutionResult> for StopAgentCompletion {
    fn from(result: SessionExecutionResult) -> Self {
        Self {
            workspace_name: result.workspace_name,
            workspace_path: result.workspace_path,
            session_name: result.session_name,
            result: result.result,
        }
    }
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key_event) => Self::Key(key_event),
            Event::Mouse(mouse_event) => Self::Mouse(mouse_event),
            Event::Paste(paste_event) => Self::Paste(paste_event),
            Event::Tick => Self::Tick,
            Event::Resize { width, height } => Self::Resize { width, height },
            _ => Self::Noop,
        }
    }
}
