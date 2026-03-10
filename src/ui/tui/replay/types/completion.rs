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
    tasks: Vec<ReplayTask>,
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
    requested_workspace_paths: Vec<PathBuf>,
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
struct ReplayPullUpstreamCompletion {
    workspace_name: String,
    workspace_path: PathBuf,
    base_branch: String,
    result: ReplayUnitResult,
    propagate_target_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceCompletion {
    request: ReplayCreateWorkspaceRequest,
    result: ReplayCreateWorkspaceResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayCreateWorkspaceRequest {
    task_name: String,
    repositories: Vec<ProjectConfig>,
    agent: ReplayAgentType,
    branch_source: ReplayTaskBranchSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ReplayTaskBranchSource {
    BaseBranch,
    PullRequest { number: u64, branch_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ReplayCreateWorkspaceResult {
    Ok {
        task_root: PathBuf,
        task: ReplayTask,
        warnings: Vec<String>,
    },
    Err {
        error: ReplayTaskLifecycleError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
enum ReplayTaskLifecycleError {
    EmptyTaskName,
    InvalidTaskName,
    EmptyRepositories,
    HomeDirectoryUnavailable,
    RepositoryNameUnavailable,
    BaseBranchDetectionFailed(String),
    TaskInvalid(String),
    TaskManifest(String),
    GitCommandFailed(String),
    Io(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayTask {
    name: String,
    slug: String,
    root_path: PathBuf,
    branch: String,
    worktrees: Vec<ReplayWorktree>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorktree {
    repository_name: String,
    repository_path: PathBuf,
    path: PathBuf,
    branch: String,
    base_branch: Option<String>,
    last_activity_unix_secs: Option<i64>,
    agent: ReplayAgentType,
    status: ReplayWorkspaceStatus,
    is_orphaned: bool,
    supported_agent: bool,
    pull_requests: Vec<ReplayPullRequest>,
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
            bootstrap: ReplayBootstrapData {
                repo_name: completion.repo_name.clone(),
                tasks: completion
                    .tasks
                    .iter()
                    .map(ReplayTask::from_task)
                    .collect(),
                discovery_state: ReplayDiscoveryState::from_discovery_state(
                    &completion.discovery_state,
                ),
            },
        }
    }

    fn to_completion(&self) -> RefreshWorkspacesCompletion {
        RefreshWorkspacesCompletion {
            preferred_workspace_path: self.preferred_workspace_path.clone(),
            repo_name: self.bootstrap.repo_name.clone(),
            discovery_state: self.bootstrap.discovery_state.to_discovery_state(),
            tasks: self.bootstrap.tasks.iter().map(ReplayTask::to_task).collect(),
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
            requested_workspace_paths: completion.requested_workspace_paths.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            warnings: completion.warnings.clone(),
        }
    }

    fn to_completion(&self) -> DeleteWorkspaceCompletion {
        DeleteWorkspaceCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            requested_workspace_paths: self.requested_workspace_paths.clone(),
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

impl ReplayPullUpstreamCompletion {
    fn from_completion(completion: &PullUpstreamCompletion) -> Self {
        Self {
            workspace_name: completion.workspace_name.clone(),
            workspace_path: completion.workspace_path.clone(),
            base_branch: completion.base_branch.clone(),
            result: ReplayUnitResult::from_result(&completion.result),
            propagate_target_count: completion.propagate_target_count,
        }
    }

    fn to_completion(&self) -> PullUpstreamCompletion {
        PullUpstreamCompletion {
            workspace_name: self.workspace_name.clone(),
            workspace_path: self.workspace_path.clone(),
            base_branch: self.base_branch.clone(),
            result: self.result.to_result(),
            propagate_target_count: self.propagate_target_count,
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
    fn from_request(request: &CreateTaskRequest) -> Self {
        Self {
            task_name: request.task_name.clone(),
            repositories: request.repositories.clone(),
            agent: ReplayAgentType::from_agent_type(request.agent),
            branch_source: ReplayTaskBranchSource::from_branch_source(&request.branch_source),
        }
    }

    fn to_request(&self) -> CreateTaskRequest {
        CreateTaskRequest {
            task_name: self.task_name.clone(),
            repositories: self.repositories.clone(),
            agent: self.agent.to_agent_type(),
            branch_source: self.branch_source.to_branch_source(),
        }
    }
}

impl ReplayTaskBranchSource {
    fn from_branch_source(branch_source: &TaskBranchSource) -> Self {
        match branch_source {
            TaskBranchSource::BaseBranch => Self::BaseBranch,
            TaskBranchSource::PullRequest {
                number,
                branch_name,
            } => Self::PullRequest {
                number: *number,
                branch_name: branch_name.clone(),
            },
        }
    }

    fn to_branch_source(&self) -> TaskBranchSource {
        match self {
            Self::BaseBranch => TaskBranchSource::BaseBranch,
            Self::PullRequest {
                number,
                branch_name,
            } => TaskBranchSource::PullRequest {
                number: *number,
                branch_name: branch_name.clone(),
            },
        }
    }
}

impl ReplayCreateWorkspaceResult {
    fn from_result(result: &Result<CreateTaskResult, TaskLifecycleError>) -> Self {
        match result {
            Ok(value) => Self::Ok {
                task_root: value.task_root.clone(),
                task: ReplayTask::from_task(&value.task),
                warnings: value.warnings.clone(),
            },
            Err(error) => Self::Err {
                error: ReplayTaskLifecycleError::from_error(error),
            },
        }
    }

    fn to_result(&self) -> Result<CreateTaskResult, TaskLifecycleError> {
        match self {
            Self::Ok { task_root, task, warnings } => Ok(CreateTaskResult {
                task_root: task_root.clone(),
                task: task.to_task(),
                warnings: warnings.clone(),
            }),
            Self::Err { error } => Err(error.to_error()),
        }
    }
}

impl ReplayTaskLifecycleError {
    fn from_error(error: &TaskLifecycleError) -> Self {
        match error {
            TaskLifecycleError::EmptyTaskName => Self::EmptyTaskName,
            TaskLifecycleError::InvalidTaskName => Self::InvalidTaskName,
            TaskLifecycleError::EmptyRepositories => Self::EmptyRepositories,
            TaskLifecycleError::HomeDirectoryUnavailable => Self::HomeDirectoryUnavailable,
            TaskLifecycleError::RepositoryNameUnavailable => Self::RepositoryNameUnavailable,
            TaskLifecycleError::BaseBranchDetectionFailed(message) => {
                Self::BaseBranchDetectionFailed(message.clone())
            }
            TaskLifecycleError::TaskInvalid(message) => Self::TaskInvalid(message.clone()),
            TaskLifecycleError::TaskManifest(message) => Self::TaskManifest(message.clone()),
            TaskLifecycleError::GitCommandFailed(message) => {
                Self::GitCommandFailed(message.clone())
            }
            TaskLifecycleError::Io(message) => Self::Io(message.clone()),
        }
    }

    fn to_error(&self) -> TaskLifecycleError {
        match self {
            Self::EmptyTaskName => TaskLifecycleError::EmptyTaskName,
            Self::InvalidTaskName => TaskLifecycleError::InvalidTaskName,
            Self::EmptyRepositories => TaskLifecycleError::EmptyRepositories,
            Self::HomeDirectoryUnavailable => TaskLifecycleError::HomeDirectoryUnavailable,
            Self::RepositoryNameUnavailable => TaskLifecycleError::RepositoryNameUnavailable,
            Self::BaseBranchDetectionFailed(message) => {
                TaskLifecycleError::BaseBranchDetectionFailed(message.clone())
            }
            Self::TaskInvalid(message) => TaskLifecycleError::TaskInvalid(message.clone()),
            Self::TaskManifest(message) => TaskLifecycleError::TaskManifest(message.clone()),
            Self::GitCommandFailed(message) => {
                TaskLifecycleError::GitCommandFailed(message.clone())
            }
            Self::Io(message) => TaskLifecycleError::Io(message.clone()),
        }
    }
}

impl ReplayTask {
    fn from_task(task: &Task) -> Self {
        Self {
            name: task.name.clone(),
            slug: task.slug.clone(),
            root_path: task.root_path.clone(),
            branch: task.branch.clone(),
            worktrees: task
                .worktrees
                .iter()
                .map(ReplayWorktree::from_worktree)
                .collect(),
        }
    }

    fn to_task(&self) -> Task {
        Task::try_new(
            self.name.clone(),
            self.slug.clone(),
            self.root_path.clone(),
            self.branch.clone(),
            self.worktrees
                .iter()
                .map(ReplayWorktree::to_worktree)
                .collect(),
        )
        .expect("replay task should decode")
    }
}

impl ReplayWorktree {
    fn from_worktree(worktree: &Worktree) -> Self {
        Self {
            repository_name: worktree.repository_name.clone(),
            repository_path: worktree.repository_path.clone(),
            path: worktree.path.clone(),
            branch: worktree.branch.clone(),
            base_branch: worktree.base_branch.clone(),
            last_activity_unix_secs: worktree.last_activity_unix_secs,
            agent: ReplayAgentType::from_agent_type(worktree.agent),
            status: ReplayWorkspaceStatus::from_workspace_status(worktree.status),
            is_orphaned: worktree.is_orphaned,
            supported_agent: worktree.supported_agent,
            pull_requests: worktree
                .pull_requests
                .iter()
                .map(ReplayPullRequest::from_pull_request)
                .collect(),
        }
    }

    fn to_worktree(&self) -> Worktree {
        Worktree::try_new(
            self.repository_name.clone(),
            self.repository_path.clone(),
            self.path.clone(),
            self.branch.clone(),
            self.agent.to_agent_type(),
            self.status.to_workspace_status(),
        )
        .expect("replay worktree should decode")
        .with_base_branch(self.base_branch.clone())
        .with_last_activity_unix_secs(self.last_activity_unix_secs)
        .with_supported_agent(self.supported_agent)
        .with_orphaned(self.is_orphaned)
        .with_pull_requests(
            self.pull_requests
                .iter()
                .map(ReplayPullRequest::to_pull_request)
                .collect(),
        )
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
