use super::*;

impl UiCommand {
    pub(super) fn status_hints_for(context: StatusHintContext) -> &'static [UiCommand] {
        match context {
            StatusHintContext::List => &[
                UiCommand::MoveSelectionDown,
                UiCommand::FocusList,
                UiCommand::OpenPreview,
                UiCommand::NewWorkspace,
                UiCommand::EditWorkspace,
                UiCommand::MergeWorkspace,
                UiCommand::UpdateFromBase,
                UiCommand::OpenProjects,
                UiCommand::DeleteWorkspace,
                UiCommand::OpenSettings,
                UiCommand::ToggleSidebar,
                UiCommand::OpenCommandPalette,
                UiCommand::OpenHelp,
                UiCommand::Quit,
            ],
            StatusHintContext::PreviewAgent => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::ScrollDown,
                UiCommand::PageDown,
                UiCommand::ScrollBottom,
                UiCommand::FocusList,
                UiCommand::OpenPreview,
                UiCommand::NewWorkspace,
                UiCommand::EditWorkspace,
                UiCommand::MergeWorkspace,
                UiCommand::UpdateFromBase,
                UiCommand::OpenProjects,
                UiCommand::StartAgent,
                UiCommand::StopAgent,
                UiCommand::DeleteWorkspace,
                UiCommand::OpenSettings,
                UiCommand::ToggleSidebar,
                UiCommand::OpenCommandPalette,
                UiCommand::OpenHelp,
                UiCommand::Quit,
            ],
            StatusHintContext::PreviewGit => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::FocusList,
                UiCommand::EnterInteractive,
                UiCommand::NewWorkspace,
                UiCommand::EditWorkspace,
                UiCommand::MergeWorkspace,
                UiCommand::UpdateFromBase,
                UiCommand::OpenProjects,
                UiCommand::DeleteWorkspace,
                UiCommand::OpenSettings,
                UiCommand::ToggleSidebar,
                UiCommand::OpenCommandPalette,
                UiCommand::OpenHelp,
                UiCommand::Quit,
            ],
        }
    }

    pub(super) fn status_hint_label(self, context: StatusHintContext) -> Option<&'static str> {
        match (self, context) {
            (UiCommand::MoveSelectionDown, StatusHintContext::List) => Some("j/k move"),
            (UiCommand::FocusList, StatusHintContext::List)
            | (UiCommand::FocusList, StatusHintContext::PreviewAgent)
            | (UiCommand::FocusList, StatusHintContext::PreviewGit) => Some("h/l pane"),
            (UiCommand::OpenPreview, StatusHintContext::List)
            | (UiCommand::OpenPreview, StatusHintContext::PreviewAgent) => Some("Enter open"),
            (UiCommand::EnterInteractive, StatusHintContext::PreviewGit) => {
                Some("Enter attach lazygit")
            }
            (UiCommand::PreviousTab, StatusHintContext::PreviewAgent)
            | (UiCommand::PreviousTab, StatusHintContext::PreviewGit) => Some("[ prev tab"),
            (UiCommand::NextTab, StatusHintContext::PreviewAgent)
            | (UiCommand::NextTab, StatusHintContext::PreviewGit) => Some("] next tab"),
            (UiCommand::ScrollDown, StatusHintContext::PreviewAgent) => Some("j/k scroll"),
            (UiCommand::PageDown, StatusHintContext::PreviewAgent) => Some("PgUp/PgDn"),
            (UiCommand::ScrollBottom, StatusHintContext::PreviewAgent) => Some("G bottom"),
            (UiCommand::NewWorkspace, _context) => Some("n new"),
            (UiCommand::EditWorkspace, _context) => Some("e edit"),
            (UiCommand::MergeWorkspace, _context) => Some("m merge"),
            (UiCommand::UpdateFromBase, _context) => Some("u update"),
            (UiCommand::OpenProjects, _context) => Some("p projects"),
            (UiCommand::StartAgent, StatusHintContext::PreviewAgent) => Some("s start"),
            (UiCommand::StopAgent, StatusHintContext::PreviewAgent) => Some("x stop"),
            (UiCommand::DeleteWorkspace, _context) => Some("D delete"),
            (UiCommand::OpenSettings, _context) => Some("S settings"),
            (UiCommand::ToggleSidebar, _context) => Some("\\ sidebar"),
            (UiCommand::OpenCommandPalette, _context) => Some("Ctrl+K palette"),
            (UiCommand::OpenHelp, _context) => Some("? help"),
            (UiCommand::Quit, _context) => Some("q quit"),
            _ => None,
        }
    }

    pub(super) fn help_hints_for(context: HelpHintContext) -> &'static [UiCommand] {
        match context {
            HelpHintContext::Global => &[
                UiCommand::OpenHelp,
                UiCommand::Quit,
                UiCommand::ToggleFocus,
                UiCommand::ToggleSidebar,
                UiCommand::FocusList,
                UiCommand::OpenPreview,
                UiCommand::OpenCommandPalette,
            ],
            HelpHintContext::Workspace => &[
                UiCommand::NewWorkspace,
                UiCommand::EditWorkspace,
                UiCommand::MergeWorkspace,
                UiCommand::UpdateFromBase,
                UiCommand::OpenProjects,
                UiCommand::DeleteWorkspace,
                UiCommand::OpenSettings,
                UiCommand::ToggleUnsafe,
            ],
            HelpHintContext::List => &[UiCommand::MoveSelectionDown],
            HelpHintContext::PreviewAgent => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::ScrollDown,
                UiCommand::PageDown,
                UiCommand::ScrollBottom,
                UiCommand::StartAgent,
                UiCommand::StopAgent,
            ],
            HelpHintContext::PreviewGit => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::EnterInteractive,
            ],
        }
    }

    pub(super) fn help_hint_label(self, context: HelpHintContext) -> Option<&'static str> {
        match (self, context) {
            (UiCommand::OpenHelp, HelpHintContext::Global) => Some("? help"),
            (UiCommand::Quit, HelpHintContext::Global) => Some("q quit"),
            (UiCommand::ToggleFocus, HelpHintContext::Global) => Some("Tab/h/l switch pane"),
            (UiCommand::ToggleSidebar, HelpHintContext::Global) => Some("\\ toggle sidebar"),
            (UiCommand::FocusList, HelpHintContext::Global) => Some("Esc list pane"),
            (UiCommand::FocusPreview, HelpHintContext::Global) => Some("l preview pane"),
            (UiCommand::OpenPreview, HelpHintContext::Global) => Some("Enter open/attach"),
            (UiCommand::OpenCommandPalette, HelpHintContext::Global) => {
                Some("Ctrl+K command palette")
            }
            (UiCommand::NewWorkspace, HelpHintContext::Workspace) => Some("n new"),
            (UiCommand::EditWorkspace, HelpHintContext::Workspace) => Some("e edit"),
            (UiCommand::MergeWorkspace, HelpHintContext::Workspace) => Some("m merge"),
            (UiCommand::UpdateFromBase, HelpHintContext::Workspace) => Some("u update"),
            (UiCommand::OpenProjects, HelpHintContext::Workspace) => Some("p projects"),
            (UiCommand::DeleteWorkspace, HelpHintContext::Workspace) => Some("D delete"),
            (UiCommand::OpenSettings, HelpHintContext::Workspace) => Some("S settings"),
            (UiCommand::ToggleUnsafe, HelpHintContext::Workspace) => Some("! unsafe toggle"),
            (UiCommand::MoveSelectionDown, HelpHintContext::List) => {
                Some("j/k or Up/Down move selection")
            }
            (UiCommand::PreviousTab, HelpHintContext::PreviewAgent)
            | (UiCommand::PreviousTab, HelpHintContext::PreviewGit) => Some("[ prev tab"),
            (UiCommand::NextTab, HelpHintContext::PreviewAgent)
            | (UiCommand::NextTab, HelpHintContext::PreviewGit) => Some("] next tab"),
            (UiCommand::ScrollDown, HelpHintContext::PreviewAgent) => Some("j/k or Up/Down scroll"),
            (UiCommand::PageDown, HelpHintContext::PreviewAgent) => Some("PgUp/PgDn page"),
            (UiCommand::ScrollBottom, HelpHintContext::PreviewAgent) => Some("G bottom"),
            (UiCommand::StartAgent, HelpHintContext::PreviewAgent) => Some("s start"),
            (UiCommand::StopAgent, HelpHintContext::PreviewAgent) => Some("x stop"),
            (UiCommand::EnterInteractive, HelpHintContext::PreviewGit) => {
                Some("Enter attach lazygit")
            }
            _ => None,
        }
    }
}
