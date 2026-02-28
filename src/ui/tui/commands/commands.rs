#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiCommand {
    ToggleFocus,
    ToggleSidebar,
    OpenPreview,
    EnterInteractive,
    FocusPreview,
    FocusList,
    MoveSelectionUp,
    MoveSelectionDown,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollBottom,
    PreviousTab,
    NextTab,
    ResizeSidebarNarrower,
    ResizeSidebarWider,
    NewWorkspace,
    EditWorkspace,
    StartAgent,
    StopAgent,
    RestartAgent,
    DeleteWorkspace,
    MergeWorkspace,
    UpdateFromBase,
    RefreshWorkspaces,
    OpenProjects,
    ReorderProjects,
    DeleteProject,
    OpenSettings,
    ToggleMouseCapture,
    ToggleUnsafe,
    OpenHelp,
    OpenCommandPalette,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaletteCommandSpec {
    pub(super) id: &'static str,
    pub(super) title: &'static str,
    pub(super) description: &'static str,
    pub(super) tags: &'static [&'static str],
    pub(super) category: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HelpHintContext {
    Global,
    Workspace,
    List,
    PreviewAgent,
    PreviewShell,
    PreviewGit,
}

impl UiCommand {
    pub(super) const ALL: [UiCommand; 35] = [
        UiCommand::ToggleFocus,
        UiCommand::ToggleSidebar,
        UiCommand::OpenPreview,
        UiCommand::EnterInteractive,
        UiCommand::FocusPreview,
        UiCommand::FocusList,
        UiCommand::MoveSelectionUp,
        UiCommand::MoveSelectionDown,
        UiCommand::ScrollUp,
        UiCommand::ScrollDown,
        UiCommand::PageUp,
        UiCommand::PageDown,
        UiCommand::ScrollBottom,
        UiCommand::PreviousTab,
        UiCommand::NextTab,
        UiCommand::ResizeSidebarNarrower,
        UiCommand::ResizeSidebarWider,
        UiCommand::NewWorkspace,
        UiCommand::EditWorkspace,
        UiCommand::StartAgent,
        UiCommand::StopAgent,
        UiCommand::RestartAgent,
        UiCommand::DeleteWorkspace,
        UiCommand::MergeWorkspace,
        UiCommand::UpdateFromBase,
        UiCommand::RefreshWorkspaces,
        UiCommand::OpenProjects,
        UiCommand::ReorderProjects,
        UiCommand::DeleteProject,
        UiCommand::OpenSettings,
        UiCommand::ToggleMouseCapture,
        UiCommand::ToggleUnsafe,
        UiCommand::OpenHelp,
        UiCommand::OpenCommandPalette,
        UiCommand::Quit,
    ];

    pub(super) fn all() -> &'static [UiCommand] {
        &Self::ALL
    }

    pub(super) fn from_palette_id(id: &str) -> Option<Self> {
        for command in Self::all() {
            if let Some(spec) = command.palette_spec()
                && spec.id == id
            {
                return Some(*command);
            }
        }
        None
    }
}
