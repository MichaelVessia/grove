use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LaunchDialogState {
    pub(super) prompt: String,
    pub(super) pre_launch_command: String,
    pub(super) skip_permissions: bool,
    pub(super) focused_field: LaunchDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) branch: String,
    pub(super) path: PathBuf,
    pub(super) is_missing: bool,
    pub(super) delete_local_branch: bool,
    pub(super) focused_field: DeleteDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MergeDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) cleanup_workspace: bool,
    pub(super) cleanup_local_branch: bool,
    pub(super) focused_field: MergeDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UpdateFromBaseDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) focused_field: UpdateFromBaseDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeleteDialogField {
    DeleteLocalBranch,
    DeleteButton,
    CancelButton,
}

impl DeleteDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::DeleteButton,
            Self::DeleteButton => Self::CancelButton,
            Self::CancelButton => Self::DeleteLocalBranch,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::CancelButton,
            Self::DeleteButton => Self::DeleteLocalBranch,
            Self::CancelButton => Self::DeleteButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeDialogField {
    CleanupWorkspace,
    CleanupLocalBranch,
    MergeButton,
    CancelButton,
}

impl MergeDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::CleanupWorkspace => Self::CleanupLocalBranch,
            Self::CleanupLocalBranch => Self::MergeButton,
            Self::MergeButton => Self::CancelButton,
            Self::CancelButton => Self::CleanupWorkspace,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::CleanupWorkspace => Self::CancelButton,
            Self::CleanupLocalBranch => Self::CleanupWorkspace,
            Self::MergeButton => Self::CleanupLocalBranch,
            Self::CancelButton => Self::MergeButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateFromBaseDialogField {
    UpdateButton,
    CancelButton,
}

impl UpdateFromBaseDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::UpdateButton => Self::CancelButton,
            Self::CancelButton => Self::UpdateButton,
        }
    }

    pub(super) fn previous(self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchDialogField {
    Prompt,
    PreLaunchCommand,
    Unsafe,
    StartButton,
    CancelButton,
}

impl LaunchDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Prompt => Self::PreLaunchCommand,
            Self::PreLaunchCommand => Self::Unsafe,
            Self::Unsafe => Self::StartButton,
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::Prompt,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Prompt => Self::CancelButton,
            Self::PreLaunchCommand => Self::Prompt,
            Self::Unsafe => Self::PreLaunchCommand,
            Self::StartButton => Self::Unsafe,
            Self::CancelButton => Self::StartButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::PreLaunchCommand => "pre_launch_command",
            Self::Unsafe => "unsafe",
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateDialogState {
    pub(super) workspace_name: String,
    pub(super) project_index: usize,
    pub(super) agent: AgentType,
    pub(super) base_branch: String,
    pub(super) focused_field: CreateDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) branch: String,
    pub(super) agent: AgentType,
    pub(super) was_running: bool,
    pub(super) focused_field: EditDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectDialogState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) selected_filtered_index: usize,
    pub(super) add_dialog: Option<ProjectAddDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectAddDialogState {
    pub(super) name: String,
    pub(super) path: String,
    pub(super) focused_field: ProjectAddDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectAddDialogField {
    Name,
    Path,
    AddButton,
    CancelButton,
}

impl ProjectAddDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Name => Self::Path,
            Self::Path => Self::AddButton,
            Self::AddButton => Self::CancelButton,
            Self::CancelButton => Self::Name,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Name => Self::CancelButton,
            Self::Path => Self::Name,
            Self::AddButton => Self::Path,
            Self::CancelButton => Self::AddButton,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsDialogState {
    pub(super) multiplexer: MultiplexerKind,
    pub(super) focused_field: SettingsDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    Multiplexer,
    SaveButton,
    CancelButton,
}

impl SettingsDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Multiplexer => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Multiplexer,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Multiplexer => Self::CancelButton,
            Self::SaveButton => Self::Multiplexer,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogField {
    WorkspaceName,
    Project,
    BaseBranch,
    Agent,
    CreateButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditDialogField {
    Agent,
    SaveButton,
    CancelButton,
}

impl EditDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Agent => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Agent,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Agent => Self::CancelButton,
            Self::SaveButton => Self::Agent,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

impl CreateDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::WorkspaceName => Self::Project,
            Self::Project => Self::BaseBranch,
            Self::BaseBranch => Self::Agent,
            Self::Agent => Self::CreateButton,
            Self::CreateButton => Self::CancelButton,
            Self::CancelButton => Self::WorkspaceName,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::WorkspaceName => Self::CancelButton,
            Self::Project => Self::WorkspaceName,
            Self::BaseBranch => Self::Project,
            Self::Agent => Self::BaseBranch,
            Self::CreateButton => Self::Agent,
            Self::CancelButton => Self::CreateButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::Project => "project",
            Self::BaseBranch => "base_branch",
            Self::Agent => "agent",
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}
