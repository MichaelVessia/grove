#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayBootstrapSnapshot {
    repo_name: String,
    discovery_state: ReplayDiscoveryState,
    projects: Vec<ProjectConfig>,
    workspaces: Vec<ReplayWorkspace>,
    selected_index: usize,
    focus: ReplayFocus,
    mode: ReplayMode,
    preview_tab: ReplayPreviewTab,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_hidden: bool,
    mouse_capture_enabled: bool,
    launch_skip_permissions: bool,
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
    Agent,
    Shell,
    Git,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayWorkspace {
    name: String,
    path: PathBuf,
    project_name: Option<String>,
    project_path: Option<PathBuf>,
    branch: String,
    base_branch: Option<String>,
    last_activity_unix_secs: Option<i64>,
    agent: ReplayAgentType,
    status: ReplayWorkspaceStatus,
    is_main: bool,
    is_orphaned: bool,
    supported_agent: bool,
    pull_requests: Vec<ReplayPullRequest>,
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
            workspaces: app
                .state
                .workspaces
                .iter()
                .map(ReplayWorkspace::from_workspace)
                .collect(),
            selected_index: app.state.selected_index,
            focus: ReplayFocus::from_focus(app.state.focus),
            mode: ReplayMode::from_mode(app.state.mode),
            preview_tab: ReplayPreviewTab::from_preview_tab(app.preview_tab),
            viewport_width: app.viewport_width,
            viewport_height: app.viewport_height,
            sidebar_width_pct: app.sidebar_width_pct,
            sidebar_hidden: app.sidebar_hidden,
            mouse_capture_enabled: app.mouse_capture_enabled,
            launch_skip_permissions: app.launch_skip_permissions,
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

impl ReplayFocus {
    fn from_focus(focus: PaneFocus) -> Self {
        match focus {
            PaneFocus::WorkspaceList => Self::WorkspaceList,
            PaneFocus::Preview => Self::Preview,
        }
    }

    fn to_focus(self) -> PaneFocus {
        match self {
            Self::WorkspaceList => PaneFocus::WorkspaceList,
            Self::Preview => PaneFocus::Preview,
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
            PreviewTab::Agent => Self::Agent,
            PreviewTab::Shell => Self::Shell,
            PreviewTab::Git => Self::Git,
        }
    }

    fn to_preview_tab(self) -> PreviewTab {
        match self {
            Self::Agent => PreviewTab::Agent,
            Self::Shell => PreviewTab::Shell,
            Self::Git => PreviewTab::Git,
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

impl ReplayWorkspace {
    fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            name: workspace.name.clone(),
            path: workspace.path.clone(),
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            branch: workspace.branch.clone(),
            base_branch: workspace.base_branch.clone(),
            last_activity_unix_secs: workspace.last_activity_unix_secs,
            agent: ReplayAgentType::from_agent_type(workspace.agent),
            status: ReplayWorkspaceStatus::from_workspace_status(workspace.status),
            is_main: workspace.is_main,
            is_orphaned: workspace.is_orphaned,
            supported_agent: workspace.supported_agent,
            pull_requests: workspace
                .pull_requests
                .iter()
                .map(ReplayPullRequest::from_pull_request)
                .collect(),
        }
    }

    fn to_workspace(&self) -> Workspace {
        Workspace {
            name: self.name.clone(),
            path: self.path.clone(),
            project_name: self.project_name.clone(),
            project_path: self.project_path.clone(),
            branch: self.branch.clone(),
            base_branch: self.base_branch.clone(),
            last_activity_unix_secs: self.last_activity_unix_secs,
            agent: self.agent.to_agent_type(),
            status: self.status.to_workspace_status(),
            is_main: self.is_main,
            is_orphaned: self.is_orphaned,
            supported_agent: self.supported_agent,
            pull_requests: self
                .pull_requests
                .iter()
                .map(ReplayPullRequest::to_pull_request)
                .collect(),
        }
    }
}
