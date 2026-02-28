use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateDialogState {
    pub(super) tab: CreateDialogTab,
    pub(super) workspace_name: String,
    pub(super) pr_url: String,
    pub(super) project_index: usize,
    pub(super) agent: AgentType,
    pub(super) base_branch: String,
    pub(super) start_config: StartAgentConfigState,
    pub(super) focused_field: CreateDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) is_main: bool,
    pub(super) branch: String,
    pub(super) base_branch: String,
    pub(super) agent: AgentType,
    pub(super) was_running: bool,
    pub(super) focused_field: EditDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogField {
    WorkspaceName,
    PullRequestUrl,
    Project,
    BaseBranch,
    Agent,
    StartConfig(StartAgentConfigField),
    CreateButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogTab {
    Manual,
    PullRequest,
}

impl CreateDialogTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::PullRequest => "From PR URL",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Manual => Self::PullRequest,
            Self::PullRequest => Self::Manual,
        }
    }

    pub(super) fn previous(self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditDialogField {
    BaseBranch,
    Agent,
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) EditDialogField {
    BaseBranch, Agent, SaveButton, CancelButton,
});

impl CreateDialogField {
    pub(super) fn first_for_tab(tab: CreateDialogTab) -> Self {
        match tab {
            CreateDialogTab::Manual => Self::WorkspaceName,
            CreateDialogTab::PullRequest => Self::Project,
        }
    }

    pub(super) fn next(self, tab: CreateDialogTab) -> Self {
        match tab {
            CreateDialogTab::Manual => match self {
                Self::WorkspaceName => Self::Project,
                Self::Project => Self::BaseBranch,
                Self::BaseBranch => Self::Agent,
                Self::Agent => Self::StartConfig(StartAgentConfigField::Prompt),
                Self::StartConfig(field) => {
                    if field == StartAgentConfigField::Unsafe {
                        Self::CreateButton
                    } else {
                        Self::StartConfig(field.next())
                    }
                }
                Self::CreateButton => Self::CancelButton,
                Self::CancelButton => Self::WorkspaceName,
                Self::PullRequestUrl => Self::Project,
            },
            CreateDialogTab::PullRequest => match self {
                Self::Project => Self::PullRequestUrl,
                Self::PullRequestUrl => Self::Agent,
                Self::Agent => Self::StartConfig(StartAgentConfigField::Prompt),
                Self::StartConfig(field) => {
                    if field == StartAgentConfigField::Unsafe {
                        Self::CreateButton
                    } else {
                        Self::StartConfig(field.next())
                    }
                }
                Self::CreateButton => Self::CancelButton,
                Self::CancelButton => Self::Project,
                Self::WorkspaceName | Self::BaseBranch => Self::Project,
            },
        }
    }

    pub(super) fn previous(self, tab: CreateDialogTab) -> Self {
        match tab {
            CreateDialogTab::Manual => match self {
                Self::WorkspaceName => Self::CancelButton,
                Self::Project => Self::WorkspaceName,
                Self::BaseBranch => Self::Project,
                Self::Agent => Self::BaseBranch,
                Self::StartConfig(field) => {
                    if field == StartAgentConfigField::Prompt {
                        Self::Agent
                    } else {
                        Self::StartConfig(field.previous())
                    }
                }
                Self::CreateButton => Self::StartConfig(StartAgentConfigField::Unsafe),
                Self::CancelButton => Self::CreateButton,
                Self::PullRequestUrl => Self::Project,
            },
            CreateDialogTab::PullRequest => match self {
                Self::Project => Self::CancelButton,
                Self::PullRequestUrl => Self::Project,
                Self::Agent => Self::PullRequestUrl,
                Self::StartConfig(field) => {
                    if field == StartAgentConfigField::Prompt {
                        Self::Agent
                    } else {
                        Self::StartConfig(field.previous())
                    }
                }
                Self::CreateButton => Self::StartConfig(StartAgentConfigField::Unsafe),
                Self::CancelButton => Self::CreateButton,
                Self::WorkspaceName | Self::BaseBranch => Self::Project,
            },
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::PullRequestUrl => "pr_url",
            Self::Project => "project",
            Self::BaseBranch => "base_branch",
            Self::Agent => "agent",
            Self::StartConfig(field) => field.label(),
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}
