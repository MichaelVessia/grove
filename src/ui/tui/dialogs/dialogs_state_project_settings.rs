use std::path::PathBuf;

use crate::infrastructure::config::ProjectConfig;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) SettingsDialogField {
    SaveButton, CancelButton,
});
