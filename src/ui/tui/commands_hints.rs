use super::*;

impl UiCommand {
    pub(super) fn help_hints_for(context: HelpHintContext) -> &'static [UiCommand] {
        match context {
            HelpHintContext::Global => &[
                UiCommand::OpenHelp,
                UiCommand::Quit,
                UiCommand::ToggleFocus,
                UiCommand::ToggleSidebar,
                UiCommand::ToggleMouseCapture,
                UiCommand::ResizeSidebarNarrower,
                UiCommand::FocusList,
                UiCommand::MoveSelectionDown,
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::OpenPreview,
                UiCommand::OpenCommandPalette,
            ],
            HelpHintContext::Workspace => &[
                UiCommand::NewWorkspace,
                UiCommand::EditWorkspace,
                UiCommand::MergeWorkspace,
                UiCommand::UpdateFromBase,
                UiCommand::OpenProjects,
                UiCommand::ReorderProjects,
                UiCommand::DeleteWorkspace,
                UiCommand::OpenSettings,
                UiCommand::ToggleUnsafe,
            ],
            HelpHintContext::List => &[UiCommand::MoveSelectionDown],
            HelpHintContext::PreviewAgent => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::EnterInteractive,
                UiCommand::ScrollDown,
                UiCommand::PageDown,
                UiCommand::ScrollBottom,
                UiCommand::StartAgent,
                UiCommand::StopAgent,
                UiCommand::RestartAgent,
            ],
            HelpHintContext::PreviewShell => &[
                UiCommand::PreviousTab,
                UiCommand::NextTab,
                UiCommand::EnterInteractive,
                UiCommand::ScrollDown,
                UiCommand::PageDown,
                UiCommand::ScrollBottom,
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
            (UiCommand::Quit, HelpHintContext::Global) => Some("q quit (confirm)"),
            (UiCommand::ToggleFocus, HelpHintContext::Global) => Some("Tab/h/l switch pane"),
            (UiCommand::ToggleSidebar, HelpHintContext::Global) => Some("\\ toggle sidebar"),
            (UiCommand::ToggleMouseCapture, HelpHintContext::Global) => {
                Some("M toggle mouse capture")
            }
            (UiCommand::ResizeSidebarNarrower, HelpHintContext::Global)
            | (UiCommand::ResizeSidebarWider, HelpHintContext::Global) => {
                Some("Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback)")
            }
            (UiCommand::FocusList, HelpHintContext::Global) => Some("Esc list pane"),
            (UiCommand::MoveSelectionDown, HelpHintContext::Global) => Some("Alt+J/K workspace"),
            (UiCommand::PreviousTab, HelpHintContext::Global) => Some("Alt+[ prev tab"),
            (UiCommand::NextTab, HelpHintContext::Global) => Some("Alt+] next tab"),
            (UiCommand::FocusPreview, HelpHintContext::Global) => Some("l preview pane"),
            (UiCommand::OpenPreview, HelpHintContext::Global) => Some("Enter open/attach"),
            (UiCommand::OpenCommandPalette, HelpHintContext::Global) => {
                Some("Ctrl+K command palette")
            }
            (UiCommand::NewWorkspace, HelpHintContext::Workspace) => Some("n new"),
            (UiCommand::EditWorkspace, HelpHintContext::Workspace) => Some("e edit/switch"),
            (UiCommand::MergeWorkspace, HelpHintContext::Workspace) => Some("m merge"),
            (UiCommand::UpdateFromBase, HelpHintContext::Workspace) => Some("u update"),
            (UiCommand::OpenProjects, HelpHintContext::Workspace) => Some("p projects"),
            (UiCommand::ReorderProjects, HelpHintContext::Workspace) => {
                Some("Ctrl+R reorder projects")
            }
            (UiCommand::DeleteWorkspace, HelpHintContext::Workspace) => Some("D delete"),
            (UiCommand::OpenSettings, HelpHintContext::Workspace) => Some("S settings"),
            (UiCommand::ToggleUnsafe, HelpHintContext::Workspace) => Some("! unsafe toggle"),
            (UiCommand::MoveSelectionDown, HelpHintContext::List) => {
                Some("j/k or Up/Down move selection")
            }
            (UiCommand::PreviousTab, HelpHintContext::PreviewAgent)
            | (UiCommand::PreviousTab, HelpHintContext::PreviewShell)
            | (UiCommand::PreviousTab, HelpHintContext::PreviewGit) => Some("[ prev tab"),
            (UiCommand::NextTab, HelpHintContext::PreviewAgent)
            | (UiCommand::NextTab, HelpHintContext::PreviewShell)
            | (UiCommand::NextTab, HelpHintContext::PreviewGit) => Some("] next tab"),
            (UiCommand::EnterInteractive, HelpHintContext::PreviewAgent) => {
                Some("Enter attach shell/agent")
            }
            (UiCommand::EnterInteractive, HelpHintContext::PreviewShell) => {
                Some("Enter attach shell")
            }
            (UiCommand::ScrollDown, HelpHintContext::PreviewAgent) => Some("j/k or Up/Down scroll"),
            (UiCommand::ScrollDown, HelpHintContext::PreviewShell) => Some("j/k or Up/Down scroll"),
            (UiCommand::PageDown, HelpHintContext::PreviewAgent) => Some("PgUp/PgDn page"),
            (UiCommand::PageDown, HelpHintContext::PreviewShell) => Some("PgUp/PgDn page"),
            (UiCommand::ScrollBottom, HelpHintContext::PreviewAgent) => Some("G or End bottom"),
            (UiCommand::ScrollBottom, HelpHintContext::PreviewShell) => Some("G or End bottom"),
            (UiCommand::StartAgent, HelpHintContext::PreviewAgent) => Some("s start"),
            (UiCommand::StopAgent, HelpHintContext::PreviewAgent) => Some("x stop (confirm)"),
            (UiCommand::RestartAgent, HelpHintContext::PreviewAgent) => Some("r restart"),
            (UiCommand::EnterInteractive, HelpHintContext::PreviewGit) => {
                Some("Enter attach lazygit")
            }
            _ => None,
        }
    }
}
