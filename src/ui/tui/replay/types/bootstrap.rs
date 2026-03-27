#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayBootstrapSnapshot {
    repo_name: String,
    discovery_state: ReplayDiscoveryState,
    projects: Vec<ProjectConfig>,
    tasks: Vec<ReplayTask>,
    selected_task_index: usize,
    selected_worktree_index: usize,
    focus: ReplayFocus,
    mode: ReplayMode,
    preview_tab: ReplayPreviewTab,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_permission_mode: PermissionMode,
    #[serde(default)]
    theme_name: ThemeName,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ReplayDiscoveryState {
    Ready,
    Empty,
    Error { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayFocus {
    WorkspaceList,
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayMode {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayPreviewTab {
    Home,
    Agent,
    Shell,
    Git,
    Diff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPullRequest {
    number: u64,
    url: String,
    status: ReplayPullRequestStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayPullRequestStatus {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayAgentType {
    Claude,
    Codex,
    Opencode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayWorkspaceStatus {
    Main,
    Idle,
    Active,
    Thinking,
    Waiting,
    Done,
    Error,
    Unknown,
    Unsupported,
}

impl ReplayBootstrapSnapshot {
    fn from_app(app: &GroveApp) -> Self {
        Self {
            repo_name: app.repo_name.clone(),
            discovery_state: ReplayDiscoveryState::from_discovery_state(&app.discovery_state),
            projects: app.projects.clone(),
            tasks: app.state.tasks.iter().map(ReplayTask::from_task).collect(),
            selected_task_index: app.state.selected_task_index,
            selected_worktree_index: app.state.selected_worktree_index,
            focus: ReplayFocus::from_mode(app.active_main_pane_mode()),
            mode: ReplayMode::from_mode(app.active_main_pane_mode()),
            preview_tab: ReplayPreviewTab::from_preview_tab(app.preview_tab),
            viewport_width: app.viewport_width,
            viewport_height: app.viewport_height,
            sidebar_width_pct: app.sidebar_width_pct,
            sidebar_hidden: app.sidebar_hidden,
            mouse_capture_enabled: app.mouse_capture_enabled,
            launch_permission_mode: app.launch_permission_mode,
            theme_name: app.theme_name,
        }
    }

    fn to_tasks(&self) -> Vec<Task> {
        self.tasks.iter().map(ReplayTask::to_task).collect()
    }
}

impl ReplayFocus {
    fn from_mode(mode: UiMode) -> Self {
        match mode {
            UiMode::List => Self::WorkspaceList,
            UiMode::Preview => Self::Preview,
        }
    }

    fn to_focus_id(self) -> u64 {
        match self {
            Self::WorkspaceList => FOCUS_ID_WORKSPACE_LIST,
            Self::Preview => FOCUS_ID_PREVIEW,
        }
    }
}

impl ReplayMode {
    fn from_mode(mode: UiMode) -> Self {
        match mode {
            UiMode::List => Self::List,
            UiMode::Preview => Self::Preview,
        }
    }

    fn to_mode(self) -> UiMode {
        match self {
            Self::List => UiMode::List,
            Self::Preview => UiMode::Preview,
        }
    }
}

impl ReplayPreviewTab {
    fn from_preview_tab(tab: PreviewTab) -> Self {
        match tab {
            PreviewTab::Home => Self::Home,
            PreviewTab::Agent => Self::Agent,
            PreviewTab::Shell => Self::Shell,
            PreviewTab::Git => Self::Git,
            PreviewTab::Diff => Self::Diff,
        }
    }

    fn to_preview_tab(self) -> PreviewTab {
        match self {
            Self::Home => PreviewTab::Home,
            Self::Agent => PreviewTab::Agent,
            Self::Shell => PreviewTab::Shell,
            Self::Git => PreviewTab::Git,
            Self::Diff => PreviewTab::Diff,
        }
    }
}

impl ReplayDiscoveryState {
    fn from_discovery_state(state: &DiscoveryState) -> Self {
        match state {
            DiscoveryState::Ready => Self::Ready,
            DiscoveryState::Empty => Self::Empty,
            DiscoveryState::Error(message) => Self::Error {
                message: message.clone(),
            },
        }
    }

    fn to_discovery_state(&self) -> DiscoveryState {
        match self {
            Self::Ready => DiscoveryState::Ready,
            Self::Empty => DiscoveryState::Empty,
            Self::Error { message } => DiscoveryState::Error(message.clone()),
        }
    }
}

impl ReplayAgentType {
    fn from_agent_type(agent: AgentType) -> Self {
        match agent {
            AgentType::Claude => Self::Claude,
            AgentType::Codex => Self::Codex,
            AgentType::OpenCode => Self::Opencode,
        }
    }

    fn to_agent_type(self) -> AgentType {
        match self {
            Self::Claude => AgentType::Claude,
            Self::Codex => AgentType::Codex,
            Self::Opencode => AgentType::OpenCode,
        }
    }
}

impl ReplayWorkspaceStatus {
    fn from_workspace_status(status: WorkspaceStatus) -> Self {
        match status {
            WorkspaceStatus::Main => Self::Main,
            WorkspaceStatus::Idle => Self::Idle,
            WorkspaceStatus::Active => Self::Active,
            WorkspaceStatus::Thinking => Self::Thinking,
            WorkspaceStatus::Waiting => Self::Waiting,
            WorkspaceStatus::Done => Self::Done,
            WorkspaceStatus::Error => Self::Error,
            WorkspaceStatus::Unknown => Self::Unknown,
            WorkspaceStatus::Unsupported => Self::Unsupported,
        }
    }

    fn to_workspace_status(self) -> WorkspaceStatus {
        match self {
            Self::Main => WorkspaceStatus::Main,
            Self::Idle => WorkspaceStatus::Idle,
            Self::Active => WorkspaceStatus::Active,
            Self::Thinking => WorkspaceStatus::Thinking,
            Self::Waiting => WorkspaceStatus::Waiting,
            Self::Done => WorkspaceStatus::Done,
            Self::Error => WorkspaceStatus::Error,
            Self::Unknown => WorkspaceStatus::Unknown,
            Self::Unsupported => WorkspaceStatus::Unsupported,
        }
    }
}

impl ReplayPullRequestStatus {
    fn from_pull_request_status(status: PullRequestStatus) -> Self {
        match status {
            PullRequestStatus::Open => Self::Open,
            PullRequestStatus::Merged => Self::Merged,
            PullRequestStatus::Closed => Self::Closed,
        }
    }

    fn to_pull_request_status(self) -> PullRequestStatus {
        match self {
            Self::Open => PullRequestStatus::Open,
            Self::Merged => PullRequestStatus::Merged,
            Self::Closed => PullRequestStatus::Closed,
        }
    }
}

impl ReplayPullRequest {
    fn from_pull_request(pull_request: &PullRequest) -> Self {
        Self {
            number: pull_request.number,
            url: pull_request.url.clone(),
            status: ReplayPullRequestStatus::from_pull_request_status(pull_request.status),
        }
    }

    fn to_pull_request(&self) -> PullRequest {
        PullRequest {
            number: self.number,
            url: self.url.clone(),
            status: self.status.to_pull_request_status(),
        }
    }
}
