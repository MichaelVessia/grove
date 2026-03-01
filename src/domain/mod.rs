use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Claude,
    Codex,
    OpenCode,
}

impl AgentType {
    pub const ALL: [Self; 3] = [Self::Claude, Self::Codex, Self::OpenCode];

    pub const fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
        }
    }

    pub const fn marker(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }

    pub const fn command_override_env_var(self) -> &'static str {
        match self {
            Self::Claude => "GROVE_CLAUDE_CMD",
            Self::Codex => "GROVE_CODEX_CMD",
            Self::OpenCode => "GROVE_OPENCODE_CMD",
        }
    }

    pub fn from_marker(value: &str) -> Option<Self> {
        match value {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Claude => Self::Codex,
            Self::Codex => Self::OpenCode,
            Self::OpenCode => Self::Claude,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Claude => Self::OpenCode,
            Self::Codex => Self::Claude,
            Self::OpenCode => Self::Codex,
        }
    }

    pub const fn allows_cursor_overlay(self) -> bool {
        !matches!(self, Self::Codex)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceStatus {
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

impl WorkspaceStatus {
    pub const fn has_session(self) -> bool {
        matches!(
            self,
            Self::Active | Self::Thinking | Self::Waiting | Self::Done | Self::Error
        )
    }

    pub const fn is_running(self) -> bool {
        matches!(self, Self::Active | Self::Thinking | Self::Waiting)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestStatus {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
    pub status: PullRequestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub project_name: Option<String>,
    pub project_path: Option<PathBuf>,
    pub branch: String,
    pub base_branch: Option<String>,
    pub last_activity_unix_secs: Option<i64>,
    pub agent: AgentType,
    pub status: WorkspaceStatus,
    pub is_main: bool,
    pub is_orphaned: bool,
    pub supported_agent: bool,
    pub pull_requests: Vec<PullRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceValidationError {
    EmptyName,
    EmptyPath,
    EmptyBranch,
    MainWorkspaceMustUseMainStatus,
}

impl Workspace {
    pub fn try_new(
        name: String,
        path: PathBuf,
        branch: String,
        last_activity_unix_secs: Option<i64>,
        agent: AgentType,
        status: WorkspaceStatus,
        is_main: bool,
    ) -> Result<Self, WorkspaceValidationError> {
        if name.trim().is_empty() {
            return Err(WorkspaceValidationError::EmptyName);
        }
        if path.as_os_str().is_empty() {
            return Err(WorkspaceValidationError::EmptyPath);
        }
        if branch.trim().is_empty() {
            return Err(WorkspaceValidationError::EmptyBranch);
        }
        if is_main && status != WorkspaceStatus::Main {
            return Err(WorkspaceValidationError::MainWorkspaceMustUseMainStatus);
        }

        Ok(Self {
            name,
            path,
            project_name: None,
            project_path: None,
            branch,
            base_branch: None,
            last_activity_unix_secs,
            agent,
            status,
            is_main,
            is_orphaned: false,
            supported_agent: true,
            pull_requests: Vec::new(),
        })
    }

    pub fn with_base_branch(mut self, base_branch: Option<String>) -> Self {
        self.base_branch = base_branch;
        self
    }

    pub fn with_project_context(mut self, project_name: String, project_path: PathBuf) -> Self {
        self.project_name = Some(project_name);
        self.project_path = Some(project_path);
        self
    }

    pub fn with_supported_agent(mut self, supported_agent: bool) -> Self {
        self.supported_agent = supported_agent;
        self
    }

    pub fn with_orphaned(mut self, is_orphaned: bool) -> Self {
        self.is_orphaned = is_orphaned;
        self
    }

    pub fn with_pull_requests(mut self, pull_requests: Vec<PullRequest>) -> Self {
        self.pull_requests = pull_requests;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentType, PullRequest, PullRequestStatus, Workspace, WorkspaceStatus,
        WorkspaceValidationError,
    };
    use std::path::PathBuf;

    #[test]
    fn main_workspace_requires_main_status() {
        let workspace = Workspace::try_new(
            "grove".to_string(),
            PathBuf::from("/repos/grove"),
            "main".to_string(),
            Some(1_700_000_000),
            AgentType::Claude,
            WorkspaceStatus::Idle,
            true,
        );
        assert_eq!(
            workspace,
            Err(WorkspaceValidationError::MainWorkspaceMustUseMainStatus)
        );
    }

    #[test]
    fn workspace_requires_non_empty_name_and_branch() {
        assert_eq!(
            Workspace::try_new(
                "".to_string(),
                PathBuf::from("/repos/grove"),
                "main".to_string(),
                Some(1_700_000_000),
                AgentType::Claude,
                WorkspaceStatus::Idle,
                false
            ),
            Err(WorkspaceValidationError::EmptyName)
        );
        assert_eq!(
            Workspace::try_new(
                "feature-x".to_string(),
                PathBuf::from("/repos/grove-feature-x"),
                "".to_string(),
                Some(1_700_000_000),
                AgentType::Claude,
                WorkspaceStatus::Idle,
                false
            ),
            Err(WorkspaceValidationError::EmptyBranch)
        );
        assert_eq!(
            Workspace::try_new(
                "feature-x".to_string(),
                PathBuf::new(),
                "feature-x".to_string(),
                Some(1_700_000_000),
                AgentType::Claude,
                WorkspaceStatus::Idle,
                false
            ),
            Err(WorkspaceValidationError::EmptyPath)
        );
    }

    #[test]
    fn workspace_accepts_valid_values() {
        let workspace = Workspace::try_new(
            "feature-x".to_string(),
            PathBuf::from("/repos/grove-feature-x"),
            "feature-x".to_string(),
            None,
            AgentType::Codex,
            WorkspaceStatus::Unknown,
            false,
        )
        .expect("workspace should be valid")
        .with_base_branch(Some("main".to_string()))
        .with_orphaned(true)
        .with_supported_agent(false);

        assert_eq!(workspace.agent.label(), "Codex");
        assert_eq!(workspace.path, PathBuf::from("/repos/grove-feature-x"));
        assert_eq!(workspace.base_branch.as_deref(), Some("main"));
        assert!(workspace.is_orphaned);
        assert!(!workspace.supported_agent);
        assert!(workspace.pull_requests.is_empty());
    }

    #[test]
    fn workspace_accepts_pull_request_metadata() {
        let workspace = Workspace::try_new(
            "feature-x".to_string(),
            PathBuf::from("/repos/grove-feature-x"),
            "feature-x".to_string(),
            None,
            AgentType::Codex,
            WorkspaceStatus::Idle,
            false,
        )
        .expect("workspace should be valid")
        .with_pull_requests(vec![PullRequest {
            number: 42,
            url: "https://github.com/acme/grove/pull/42".to_string(),
            status: PullRequestStatus::Merged,
        }]);

        assert_eq!(workspace.pull_requests.len(), 1);
        assert_eq!(workspace.pull_requests[0].number, 42);
        assert_eq!(workspace.pull_requests[0].status, PullRequestStatus::Merged);
    }

    #[test]
    fn agent_type_metadata_roundtrips_marker() {
        for agent in AgentType::all() {
            assert_eq!(AgentType::from_marker(agent.marker()), Some(*agent));
            assert!(!agent.label().is_empty());
            assert!(!agent.command_override_env_var().is_empty());
        }
    }

    #[test]
    fn agent_type_cycles_all_variants() {
        let mut forward = AgentType::Claude;
        for _ in 0..AgentType::all().len() {
            forward = forward.next();
        }
        assert_eq!(forward, AgentType::Claude);

        let mut backward = AgentType::Claude;
        for _ in 0..AgentType::all().len() {
            backward = backward.previous();
        }
        assert_eq!(backward, AgentType::Claude);
    }
}
