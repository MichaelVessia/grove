use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Claude,
    Codex,
}

impl AgentType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
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
