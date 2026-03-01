#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPreviewPollCompletion {
    generation: u64,
    live_capture: Option<ReplayLivePreviewCapture>,
    cursor_capture: Option<ReplayCursorCapture>,
    workspace_status_captures: Vec<ReplayWorkspaceStatusCapture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayLivePreviewCapture {
    session: String,
    #[serde(default = "default_live_preview_scrollback_lines")]
    scrollback_lines: usize,
    include_escape_sequences: bool,
    capture_ms: u64,
    total_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCursorCapture {
    session: String,
    capture_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspaceStatusCapture {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    supported_agent: bool,
    capture_ms: u64,
    result: ReplayStringResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayLazygitLaunchCompletion {
    session_name: String,
    duration_ms: u64,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspaceShellLaunchCompletion {
    session_name: String,
    duration_ms: u64,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayRefreshWorkspacesCompletion {
    preferred_workspace_path: Option<PathBuf>,
    bootstrap: ReplayBootstrapData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayBootstrapData {
    repo_name: String,
    workspaces: Vec<ReplayWorkspace>,
    discovery_state: ReplayDiscoveryState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayDeleteProjectCompletion {
    project_name: String,
    project_path: PathBuf,
    projects: Vec<ProjectConfig>,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayDeleteWorkspaceCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayMergeWorkspaceCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    workspace_branch: String,
    base_branch: String,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayUpdateWorkspaceFromBaseCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    workspace_branch: String,
    base_branch: String,
    result: ReplayUnitResult,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceCompletion {
    request: ReplayCreateWorkspaceRequest,
    result: ReplayCreateWorkspaceResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceRequest {
    workspace_name: String,
    branch_mode: ReplayBranchMode,
    agent: ReplayAgentType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayBranchMode {
    NewBranch { base_branch: String },
    ExistingBranch { existing_branch: String },
    PullRequest { number: u64, base_branch: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayCreateWorkspaceResult {
    Ok {
        workspace_path: PathBuf,
        branch: String,
        warnings: Vec<String>,
    },
    Err {
        error: ReplayWorkspaceLifecycleError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
enum ReplayWorkspaceLifecycleError {
    EmptyWorkspaceName,
    InvalidWorkspaceName,
    EmptyBaseBranch,
    EmptyExistingBranch,
    InvalidPullRequestNumber,
    RepoNameUnavailable,
    HomeDirectoryUnavailable,
    GitCommandFailed(String),
    Io(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySessionCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    session_name: String,
    result: ReplayUnitResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayInteractiveSendCompletion {
    send: ReplayQueuedInteractiveSend,
    tmux_send_ms: u64,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayQueuedInteractiveSend {
    command: Vec<String>,
    target_session: String,
    attention_ack_workspace_path: Option<PathBuf>,
    action_kind: String,
    trace_context: Option<ReplayInputTraceContext>,
    literal_chars: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayInputTraceContext {
    seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayStringResult {
    Ok { output: String },
    Err { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayUnitResult {
    Ok,
    Err { error: String },
}

impl ReplayPreviewPollCompletion {
    fn from_completion(completion: &PreviewPollCompletion) -> Self {
        Self {
            generation: completion.generation,
            live_capture: completion
                .live_capture
                .as_ref()
                .map(ReplayLivePreviewCapture::from_capture),
            cursor_capture: completion
                .cursor_capture
                .as_ref()
                .map(ReplayCursorCapture::from_capture),
            workspace_status_captures: completion
                .workspace_status_captures
                .iter()
                .map(ReplayWorkspaceStatusCapture::from_capture)
                .collect(),
        }
    }

    fn to_completion(&self) -> PreviewPollCompletion {
        PreviewPollCompletion {
            generation: self.generation,
            live_capture: self
                .live_capture
                .as_ref()
                .map(ReplayLivePreviewCapture::to_capture),
            cursor_capture: self
                .cursor_capture
                .as_ref()
                .map(ReplayCursorCapture::to_capture),
            workspace_status_captures: self
                .workspace_status_captures
                .iter()
                .map(ReplayWorkspaceStatusCapture::to_capture)
                .collect(),
        }
    }
}

fn default_live_preview_scrollback_lines() -> usize {
    LIVE_PREVIEW_SCROLLBACK_LINES
}

impl ReplayLivePreviewCapture {
    fn from_capture(capture: &LivePreviewCapture) -> Self {
        Self {
            session: capture.session.clone(),
            scrollback_lines: capture.scrollback_lines,
            include_escape_sequences: capture.include_escape_sequences,
            capture_ms: capture.capture_ms,
            total_ms: capture.total_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> LivePreviewCapture {
        LivePreviewCapture {
            session: self.session.clone(),
            scrollback_lines: self.scrollback_lines,
            include_escape_sequences: self.include_escape_sequences,
            capture_ms: self.capture_ms,
            total_ms: self.total_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayCursorCapture {
    fn from_capture(capture: &CursorCapture) -> Self {
        Self {
            session: capture.session.clone(),
            capture_ms: capture.capture_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> CursorCapture {
        CursorCapture {
            session: self.session.clone(),
            capture_ms: self.capture_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayWorkspaceStatusCapture {
    fn from_capture(capture: &WorkspaceStatusCapture) -> Self {
        Self {
            workspace_name: capture.workspace_name.clone(),
            workspace_path: capture.workspace_path.clone(),
            session_name: capture.session_name.clone(),
            supported_agent: capture.supported_agent,
            capture_ms: capture.capture_ms,
            result: ReplayStringResult::from_result(&capture.result),
        }
    }

    fn to_capture(&self) -> WorkspaceStatusCapture {
        WorkspaceStatusCapture {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            supported_agent: self.supported_agent,
            capture_ms: self.capture_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayLazygitLaunchCompletion {
    fn from_completion(completion: &LazygitLaunchCompletion) -> Self {
        Self {
            session_name: completion.session_name.clone(),
            duration_ms: completion.duration_ms,
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> LazygitLaunchCompletion {
        LazygitLaunchCompletion {
            session_name: self.session_name.clone(),
            duration_ms: self.duration_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayWorkspaceShellLaunchCompletion {
    fn from_completion(completion: &WorkspaceShellLaunchCompletion) -> Self {
        Self {
            session_name: completion.session_name.clone(),
            duration_ms: completion.duration_ms,
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> WorkspaceShellLaunchCompletion {
        WorkspaceShellLaunchCompletion {
            session_name: self.session_name.clone(),
            duration_ms: self.duration_ms,
            result: self.result.to_result(),
        }
    }
}

impl ReplayRefreshWorkspacesCompletion {
    fn from_completion(completion: &RefreshWorkspacesCompletion) -> Self {
        Self {
            preferred_workspace_path: completion.preferred_workspace_path.clone(),
            bootstrap: ReplayBootstrapData::from_bootstrap_data(&completion.bootstrap),
        }
    }

    fn to_completion(&self) -> RefreshWorkspacesCompletion {
        RefreshWorkspacesCompletion {
            preferred_workspace_path: self.preferred_workspace_path.clone(),
            bootstrap: self.bootstrap.to_bootstrap_data(),
        }
    }
}

impl ReplayBootstrapData {
    fn from_bootstrap_data(data: &BootstrapData) -> Self {
        Self {
            repo_name: data.repo_name.clone(),
            workspaces: data
                .workspaces
                .iter()
                .map(ReplayWorkspace::from_workspace)
                .collect(),
            discovery_state: ReplayDiscoveryState::from_discovery_state(&data.discovery_state),
        }
    }

    fn to_bootstrap_data(&self) -> BootstrapData {
        BootstrapData {
            repo_name: self.repo_name.clone(),
            workspaces: self
                .workspaces
                .iter()
                .map(ReplayWorkspace::to_workspace)
                .collect(),
            discovery_state: self.discovery_state.to_discovery_state(),
        }
    }
}

impl ReplayDeleteProjectCompletion {
    fn from_completion(completion: &DeleteProjectCompletion) -> Self {
        Self {
            project_name: completion.project_name.clone(),
            project_path: completion.project_path.clone(),
            projects: completion.projects.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> DeleteProjectCompletion {
        DeleteProjectCompletion {
            project_name: self.project_name.clone(),
            project_path: self.project_path.clone(),
            projects: self.projects.clone(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayDeleteWorkspaceCompletion {
    fn from_completion(completion: &DeleteWorkspaceCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> DeleteWorkspaceCompletion {
        DeleteWorkspaceCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayMergeWorkspaceCompletion {
    fn from_completion(completion: &MergeWorkspaceCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            workspace_branch: completion.workspace_branch.clone(),
            base_branch: completion.base_branch.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> MergeWorkspaceCompletion {
        MergeWorkspaceCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            workspace_branch: self.workspace_branch.clone(),
            base_branch: self.base_branch.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayUpdateWorkspaceFromBaseCompletion {
    fn from_completion(completion: &UpdateWorkspaceFromBaseCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            workspace_branch: completion.workspace_branch.clone(),
            base_branch: completion.base_branch.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> UpdateWorkspaceFromBaseCompletion {
        UpdateWorkspaceFromBaseCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            workspace_branch: self.workspace_branch.clone(),
            base_branch: self.base_branch.clone(),
            result: self.result.to_result(),
            warnings: self.warnings.clone(),
        }
    }
}

impl ReplayCreateWorkspaceCompletion {
    fn from_completion(completion: &CreateWorkspaceCompletion) -> Self {
        Self {
            request: ReplayCreateWorkspaceRequest::from_request(&completion.request),
            result: ReplayCreateWorkspaceResult::from_result(&completion.result),
        }
    }

    fn to_completion(&self) -> CreateWorkspaceCompletion {
        CreateWorkspaceCompletion {
            request: self.request.to_request(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayCreateWorkspaceRequest {
    fn from_request(request: &CreateWorkspaceRequest) -> Self {
        Self {
            workspace_name: request.workspace_name.clone(),
            branch_mode: ReplayBranchMode::from_branch_mode(&request.branch_mode),
            agent: ReplayAgentType::from_agent_type(request.agent),
        }
    }

    fn to_request(&self) -> CreateWorkspaceRequest {
        CreateWorkspaceRequest {
            workspace_name: self.workspace_name.clone(),
            branch_mode: self.branch_mode.to_branch_mode(),
            agent: self.agent.to_agent_type(),
        }
    }
}

impl ReplayBranchMode {
    fn from_branch_mode(mode: &BranchMode) -> Self {
        match mode {
            BranchMode::NewBranch { base_branch } => Self::NewBranch {
                base_branch: base_branch.clone(),
            },
            BranchMode::ExistingBranch { existing_branch } => Self::ExistingBranch {
                existing_branch: existing_branch.clone(),
            },
            BranchMode::PullRequest {
                number,
                base_branch,
            } => Self::PullRequest {
                number: *number,
                base_branch: base_branch.clone(),
            },
        }
    }

    fn to_branch_mode(&self) -> BranchMode {
        match self {
            Self::NewBranch { base_branch } => BranchMode::NewBranch {
                base_branch: base_branch.clone(),
            },
            Self::ExistingBranch { existing_branch } => BranchMode::ExistingBranch {
                existing_branch: existing_branch.clone(),
            },
            Self::PullRequest {
                number,
                base_branch,
            } => BranchMode::PullRequest {
                number: *number,
                base_branch: base_branch.clone(),
            },
        }
    }
}

impl ReplayCreateWorkspaceResult {
    fn from_result(result: &Result<CreateWorkspaceResult, WorkspaceLifecycleError>) -> Self {
        match result {
            Ok(value) => Self::Ok {
                workspace_path: value.workspace_path.clone(),
                branch: value.branch.clone(),
                warnings: value.warnings.clone(),
            },
            Err(error) => Self::Err {
                error: ReplayWorkspaceLifecycleError::from_error(error),
            },
        }
    }

    fn to_result(&self) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
        match self {
            Self::Ok {
                workspace_path,
                branch,
                warnings,
            } => Ok(CreateWorkspaceResult {
                workspace_path: workspace_path.clone(),
                branch: branch.clone(),
                warnings: warnings.clone(),
            }),
            Self::Err { error } => Err(error.to_error()),
        }
    }
}

impl ReplayWorkspaceLifecycleError {
    fn from_error(error: &WorkspaceLifecycleError) -> Self {
        match error {
            WorkspaceLifecycleError::EmptyWorkspaceName => Self::EmptyWorkspaceName,
            WorkspaceLifecycleError::InvalidWorkspaceName => Self::InvalidWorkspaceName,
            WorkspaceLifecycleError::EmptyBaseBranch => Self::EmptyBaseBranch,
            WorkspaceLifecycleError::EmptyExistingBranch => Self::EmptyExistingBranch,
            WorkspaceLifecycleError::InvalidPullRequestNumber => Self::InvalidPullRequestNumber,
            WorkspaceLifecycleError::RepoNameUnavailable => Self::RepoNameUnavailable,
            WorkspaceLifecycleError::HomeDirectoryUnavailable => Self::HomeDirectoryUnavailable,
            WorkspaceLifecycleError::GitCommandFailed(message) => {
                Self::GitCommandFailed(message.clone())
            }
            WorkspaceLifecycleError::Io(message) => Self::Io(message.clone()),
        }
    }

    fn to_error(&self) -> WorkspaceLifecycleError {
        match self {
            Self::EmptyWorkspaceName => WorkspaceLifecycleError::EmptyWorkspaceName,
            Self::InvalidWorkspaceName => WorkspaceLifecycleError::InvalidWorkspaceName,
            Self::EmptyBaseBranch => WorkspaceLifecycleError::EmptyBaseBranch,
            Self::EmptyExistingBranch => WorkspaceLifecycleError::EmptyExistingBranch,
            Self::InvalidPullRequestNumber => WorkspaceLifecycleError::InvalidPullRequestNumber,
            Self::RepoNameUnavailable => WorkspaceLifecycleError::RepoNameUnavailable,
            Self::HomeDirectoryUnavailable => WorkspaceLifecycleError::HomeDirectoryUnavailable,
            Self::GitCommandFailed(message) => {
                WorkspaceLifecycleError::GitCommandFailed(message.clone())
            }
            Self::Io(message) => WorkspaceLifecycleError::Io(message.clone()),
        }
    }
}

impl ReplaySessionCompletion {
    fn from_start_completion(completion: &StartAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn from_stop_completion(completion: &StopAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn from_restart_completion(completion: &RestartAgentCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            session_name: completion.session_name.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
        }
    }

    fn to_start_completion(&self) -> StartAgentCompletion {
        StartAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }

    fn to_stop_completion(&self) -> StopAgentCompletion {
        StopAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }

    fn to_restart_completion(&self) -> RestartAgentCompletion {
        RestartAgentCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            session_name: self.session_name.clone(),
            result: self.result.to_result(),
        }
    }
}

impl ReplayInteractiveSendCompletion {
    fn from_completion(completion: &InteractiveSendCompletion) -> Self {
        Self {
            send: ReplayQueuedInteractiveSend::from_send(&completion.send),
            tmux_send_ms: completion.tmux_send_ms,
            error: completion.error.clone(),
        }
    }

    fn to_completion(&self) -> InteractiveSendCompletion {
        InteractiveSendCompletion {
            send: self.send.to_send(),
            tmux_send_ms: self.tmux_send_ms,
            error: self.error.clone(),
        }
    }
}

impl ReplayQueuedInteractiveSend {
    fn from_send(send: &QueuedInteractiveSend) -> Self {
        Self {
            command: send.command.clone(),
            target_session: send.target_session.clone(),
            attention_ack_workspace_path: send.attention_ack_workspace_path.clone(),
            action_kind: send.action_kind.clone(),
            trace_context: send
                .trace_context
                .as_ref()
                .map(ReplayInputTraceContext::from_trace_context),
            literal_chars: send.literal_chars,
        }
    }

    fn to_send(&self) -> QueuedInteractiveSend {
        QueuedInteractiveSend {
            command: self.command.clone(),
            target_session: self.target_session.clone(),
            attention_ack_workspace_path: self.attention_ack_workspace_path.clone(),
            action_kind: self.action_kind.clone(),
            trace_context: self
                .trace_context
                .as_ref()
                .map(ReplayInputTraceContext::to_trace_context),
            literal_chars: self.literal_chars,
        }
    }
}

impl ReplayInputTraceContext {
    fn from_trace_context(trace_context: &InputTraceContext) -> Self {
        Self {
            seq: trace_context.seq,
        }
    }

    fn to_trace_context(&self) -> InputTraceContext {
        InputTraceContext {
            seq: self.seq,
            received_at: std::time::Instant::now(),
        }
    }
}

impl ReplayStringResult {
    fn from_result(result: &Result<String, String>) -> Self {
        match result {
            Ok(output) => Self::Ok {
                output: output.clone(),
            },
            Err(error) => Self::Err {
                error: error.clone(),
            },
        }
    }

    fn to_result(&self) -> Result<String, String> {
        match self {
            Self::Ok { output } => Ok(output.clone()),
            Self::Err { error } => Err(error.clone()),
        }
    }
}

impl ReplayUnitResult {
    fn from_result(result: &Result<(), String>) -> Self {
        match result {
            Ok(()) => Self::Ok,
            Err(error) => Self::Err {
                error: error.clone(),
            },
        }
    }

    fn to_result(&self) -> Result<(), String> {
        match self {
            Self::Ok => Ok(()),
            Self::Err { error } => Err(error.clone()),
        }
    }
}
