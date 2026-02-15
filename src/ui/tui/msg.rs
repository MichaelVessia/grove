use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Msg {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(PasteEvent),
    Tick,
    Resize { width: u16, height: u16 },
    PreviewPollCompleted(PreviewPollCompletion),
    RefreshWorkspacesCompleted(RefreshWorkspacesCompletion),
    DeleteWorkspaceCompleted(DeleteWorkspaceCompletion),
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
pub(super) struct CursorCapture {
    pub(super) session: String,
    pub(super) capture_ms: u64,
    pub(super) result: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceStatusPollTarget {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) session_name: String,
    pub(super) supported_agent: bool,
}

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
