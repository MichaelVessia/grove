#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectDialogState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) selected_filtered_index: usize,
    pub(super) add_dialog: Option<ProjectAddDialogState>,
    pub(super) defaults_dialog: Option<ProjectDefaultsDialogState>,
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
    pub(super) setup_commands: String,
    pub(super) auto_run_setup_commands: bool,
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
    SetupCommands,
    AutoRunSetupCommands,
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) ProjectAddDialogField {
    Name, Path, AddButton, CancelButton,
});

cyclic_field_nav!(pub(super) ProjectDefaultsDialogField {
    BaseBranch, SetupCommands, AutoRunSetupCommands, SaveButton, CancelButton,
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsDialogState {
    pub(super) focused_field: SettingsDialogField,
    pub(super) remote_name: String,
    pub(super) remote_host: String,
    pub(super) remote_user: String,
    pub(super) remote_socket_path: String,
    pub(super) remote_default_repo_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    RemoteName,
    RemoteHost,
    RemoteUser,
    RemoteSocketPath,
    RemoteDefaultRepoPath,
    SaveProfileButton,
    DeleteProfileButton,
    TestProfileButton,
    ConnectProfileButton,
    DisconnectProfileButton,
    SaveButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) SettingsDialogField {
    RemoteName,
    RemoteHost,
    RemoteUser,
    RemoteSocketPath,
    RemoteDefaultRepoPath,
    SaveProfileButton,
    DeleteProfileButton,
    TestProfileButton,
    ConnectProfileButton,
    DisconnectProfileButton,
    SaveButton,
    CancelButton,
});
