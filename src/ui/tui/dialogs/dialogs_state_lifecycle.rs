use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartAgentConfigState {
    pub(super) prompt: String,
    pub(super) init_command: String,
    pub(super) skip_permissions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartOptions {
    pub(super) prompt: Option<String>,
    pub(super) init_command: Option<String>,
    pub(super) skip_permissions: bool,
}

impl StartAgentConfigState {
    pub(super) fn new(prompt: String, init_command: String, skip_permissions: bool) -> Self {
        Self {
            prompt,
            init_command,
            skip_permissions,
        }
    }

    pub(super) fn is_input_nonempty(&self) -> bool {
        !self.prompt.is_empty() || !self.init_command.is_empty()
    }

    pub(super) fn parse_start_options(&self) -> StartOptions {
        StartOptions {
            prompt: trimmed_nonempty(&self.prompt),
            init_command: trimmed_nonempty(&self.init_command),
            skip_permissions: self.skip_permissions,
        }
    }

    fn text_field_mut(&mut self, field: StartAgentConfigField) -> Option<&mut String> {
        match field {
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
    pub(super) start_config: StartAgentConfigState,
    pub(super) focused_field: LaunchDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StopDialogState {
    pub(super) workspace: Workspace,
    pub(super) session_name: String,
    pub(super) focused_field: StopDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConfirmDialogAction {
    RestartAgent {
        workspace_name: String,
        workspace_path: PathBuf,
    },
    QuitApp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfirmDialogState {
    pub(super) action: ConfirmDialogAction,
    pub(super) focused_field: ConfirmDialogField,
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

cyclic_field_nav!(pub(super) StopDialogField {
    StopButton, CancelButton,
});

cyclic_field_nav!(pub(super) ConfirmDialogField {
    ConfirmButton, CancelButton,
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
    Prompt,
    InitCommand,
    Unsafe,
}

cyclic_field_nav!(pub(super) StartAgentConfigField {
    Prompt, InitCommand, Unsafe,
});

impl StartAgentConfigField {
    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::InitCommand => "init_command",
            Self::Unsafe => "unsafe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchDialogField {
    StartConfig(StartAgentConfigField),
    StartButton,
    CancelButton,
}

impl LaunchDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::StartConfig(field) => {
                if field == StartAgentConfigField::Unsafe {
                    Self::StartButton
                } else {
                    Self::StartConfig(field.next())
                }
            }
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::StartConfig(StartAgentConfigField::Prompt),
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::StartConfig(field) => {
                if field == StartAgentConfigField::Prompt {
                    Self::CancelButton
                } else {
                    Self::StartConfig(field.previous())
                }
            }
            Self::StartButton => Self::StartConfig(StartAgentConfigField::Unsafe),
            Self::CancelButton => Self::StartButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::StartConfig(field) => field.label(),
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}
