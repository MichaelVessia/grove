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

    pub const fn exit_command(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("/exit"),
            _ => None,
        }
    }

    pub const fn resume_command_pattern(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some(r"(claude\s+--resume\s+\S+(?:[ \t]+\S+)*)"),
            _ => None,
        }
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
}

#[cfg(test)]
mod tests;
