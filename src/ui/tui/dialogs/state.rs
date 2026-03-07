use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartAgentConfigState {
    pub(super) name: String,
    pub(super) prompt: String,
    pub(super) init_command: String,
    pub(super) skip_permissions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartOptions {
    pub(super) name: Option<String>,
    pub(super) prompt: Option<String>,
    pub(super) init_command: Option<String>,
    pub(super) skip_permissions: bool,
}

impl StartAgentConfigState {
    pub(super) fn new(
        name: String,
        prompt: String,
        init_command: String,
        skip_permissions: bool,
    ) -> Self {
        Self {
            name,
            prompt,
            init_command,
            skip_permissions,
        }
    }

    pub(super) fn is_input_nonempty(&self) -> bool {
        !self.name.is_empty() || !self.prompt.is_empty() || !self.init_command.is_empty()
    }

    pub(super) fn parse_start_options(&self) -> StartOptions {
        StartOptions {
            name: trimmed_nonempty(&self.name),
            prompt: trimmed_nonempty(&self.prompt),
            init_command: trimmed_nonempty(&self.init_command),
            skip_permissions: self.skip_permissions,
        }
    }

    fn text_field_mut(&mut self, field: StartAgentConfigField) -> Option<&mut String> {
        match field {
            StartAgentConfigField::Name => Some(&mut self.name),
            StartAgentConfigField::Prompt => Some(&mut self.prompt),
            StartAgentConfigField::InitCommand => Some(&mut self.init_command),
            StartAgentConfigField::Unsafe => None,
        }
    }

    pub(super) fn backspace(&mut self, field: StartAgentConfigField) {
        if let Some(text) = self.text_field_mut(field) {
            text.pop();
        }
    }

    pub(super) fn clear(&mut self, field: StartAgentConfigField) {
        if let Some(text) = self.text_field_mut(field) {
            text.clear();
        }
    }

    pub(super) fn push_char(&mut self, field: StartAgentConfigField, character: char) {
        if let Some(text) = self.text_field_mut(field) {
            text.push(character);
        }
    }

    pub(super) fn toggle_unsafe(&mut self) {
        self.skip_permissions = !self.skip_permissions;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LaunchDialogState {
    pub(super) agent: AgentType,
    pub(super) start_config: StartAgentConfigState,
    pub(super) focused_field: LaunchDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StopDialogState {
    pub(super) workspace: Workspace,
    pub(super) session_name: String,
    pub(super) focused_field: StopDialogField,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConfirmDialogAction {
    CloseActiveTab {
        workspace_path: PathBuf,
        tab_id: u64,
        session_name: String,
    },
    QuitApp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfirmDialogState {
    pub(super) action: ConfirmDialogAction,
    pub(super) focused_field: ConfirmDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionCleanupDialogState {
    pub(super) options: SessionCleanupOptions,
    pub(super) plan: SessionCleanupPlan,
    pub(super) last_error: Option<String>,
    pub(super) focused_field: SessionCleanupDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteDialogState {
    pub(super) task: Task,
    pub(super) is_missing: bool,
    pub(super) delete_local_branch: bool,
    pub(super) kill_tmux_sessions: bool,
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
    pub(super) is_main_workspace: bool,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) focused_field: UpdateFromBaseDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeleteDialogField {
    DeleteLocalBranch,
    KillTmuxSessions,
    DeleteButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopDialogField {
    StopButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfirmDialogField {
    ConfirmButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionCleanupDialogField {
    IncludeStale,
    IncludeAttached,
    ApplyButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) StopDialogField {
    StopButton, CancelButton,
});

cyclic_field_nav!(pub(super) ConfirmDialogField {
    ConfirmButton, CancelButton,
});

cyclic_field_nav!(pub(super) SessionCleanupDialogField {
    IncludeStale, IncludeAttached, ApplyButton, CancelButton,
});

cyclic_field_nav!(pub(super) DeleteDialogField {
    DeleteLocalBranch, KillTmuxSessions, DeleteButton, CancelButton,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeDialogField {
    CleanupWorkspace,
    CleanupLocalBranch,
    MergeButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) MergeDialogField {
    CleanupWorkspace, CleanupLocalBranch, MergeButton, CancelButton,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateFromBaseDialogField {
    UpdateButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) UpdateFromBaseDialogField {
    UpdateButton, CancelButton,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartAgentConfigField {
    Name,
    Prompt,
    InitCommand,
    Unsafe,
}

cyclic_field_nav!(pub(super) StartAgentConfigField {
    Name, Prompt, InitCommand, Unsafe,
});

impl StartAgentConfigField {
    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Prompt => "prompt",
            Self::InitCommand => "init_command",
            Self::Unsafe => "unsafe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchDialogField {
    Agent,
    StartConfig(StartAgentConfigField),
    StartButton,
    CancelButton,
}

impl LaunchDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Agent => Self::StartConfig(StartAgentConfigField::Name),
            Self::StartConfig(field) => {
                if field == StartAgentConfigField::Unsafe {
                    Self::StartButton
                } else {
                    Self::StartConfig(field.next())
                }
            }
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::StartConfig(StartAgentConfigField::Name),
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::StartConfig(field) => {
                if field == StartAgentConfigField::Name {
                    Self::Agent
                } else {
                    Self::StartConfig(field.previous())
                }
            }
            Self::StartButton => Self::StartConfig(StartAgentConfigField::Unsafe),
            Self::CancelButton => Self::StartButton,
            Self::Agent => Self::CancelButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::StartConfig(field) => field.label(),
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateDialogState {
    pub(super) tab: CreateDialogTab,
    pub(super) task_name: String,
    pub(super) pr_url: String,
    pub(super) project_index: usize,
    pub(super) selected_repository_indices: Vec<usize>,
    pub(super) project_picker: Option<CreateProjectPickerState>,
    pub(super) focused_field: CreateDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateProjectPickerState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) selected_filtered_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) is_main: bool,
    pub(super) branch: String,
    pub(super) base_branch: String,
    pub(super) was_running: bool,
    pub(super) focused_field: EditDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RenameTabDialogState {
    pub(super) workspace_path: PathBuf,
    pub(super) tab_id: u64,
    pub(super) current_title: String,
    pub(super) title: String,
    pub(super) focused_field: RenameTabDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogField {
    WorkspaceName,
    PullRequestUrl,
    Project,
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
            Self::PullRequest => "From GitHub PR",
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
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) EditDialogField {
    BaseBranch, SaveButton, CancelButton,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenameTabDialogField {
    Title,
    RenameButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) RenameTabDialogField {
    Title, RenameButton, CancelButton,
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
                Self::Project => Self::CreateButton,
                Self::CreateButton => Self::CancelButton,
                Self::CancelButton => Self::WorkspaceName,
                Self::PullRequestUrl => Self::Project,
            },
            CreateDialogTab::PullRequest => match self {
                Self::Project => Self::PullRequestUrl,
                Self::PullRequestUrl => Self::CreateButton,
                Self::CreateButton => Self::CancelButton,
                Self::CancelButton => Self::Project,
                Self::WorkspaceName => Self::Project,
            },
        }
    }

    pub(super) fn previous(self, tab: CreateDialogTab) -> Self {
        match tab {
            CreateDialogTab::Manual => match self {
                Self::WorkspaceName => Self::CancelButton,
                Self::Project => Self::WorkspaceName,
                Self::CreateButton => Self::Project,
                Self::CancelButton => Self::CreateButton,
                Self::PullRequestUrl => Self::Project,
            },
            CreateDialogTab::PullRequest => match self {
                Self::Project => Self::CancelButton,
                Self::PullRequestUrl => Self::Project,
                Self::CreateButton => Self::PullRequestUrl,
                Self::CancelButton => Self::CreateButton,
                Self::WorkspaceName => Self::Project,
            },
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::PullRequestUrl => "pr_url",
            Self::Project => "project",
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectDialogState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) selected_filtered_index: usize,
    pub(super) reorder: Option<ProjectReorderState>,
    pub(super) add_dialog: Option<ProjectAddDialogState>,
    pub(super) defaults_dialog: Option<ProjectDefaultsDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectReorderState {
    pub(super) original_projects: Vec<ProjectConfig>,
    pub(super) moving_project_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectAddDialogState {
    pub(super) name: String,
    pub(super) path: String,
    pub(super) focused_field: ProjectAddDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectDefaultsDialogState {
    pub(super) project_index: usize,
    pub(super) base_branch: String,
    pub(super) workspace_init_command: String,
    pub(super) claude_env: String,
    pub(super) codex_env: String,
    pub(super) opencode_env: String,
    pub(super) focused_field: ProjectDefaultsDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectAddDialogField {
    Name,
    Path,
    AddButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectDefaultsDialogField {
    BaseBranch,
    WorkspaceInitCommand,
    ClaudeEnv,
    CodexEnv,
    OpenCodeEnv,
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) ProjectAddDialogField {
    Name, Path, AddButton, CancelButton,
});

cyclic_field_nav!(pub(super) ProjectDefaultsDialogField {
    BaseBranch, WorkspaceInitCommand, ClaudeEnv, CodexEnv, OpenCodeEnv, SaveButton, CancelButton,
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsDialogState {
    pub(super) focused_field: SettingsDialogField,
    pub(super) initial_theme: ThemeName,
    pub(super) theme: ThemeName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    Theme,
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) SettingsDialogField {
    Theme, SaveButton, CancelButton,
});
