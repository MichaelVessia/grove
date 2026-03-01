use super::*;

const HELP_HINT_ENTRIES: &[(HelpHintContext, UiCommand, &str)] = &[
    (HelpHintContext::Global, UiCommand::OpenHelp, "? help"),
    (
        HelpHintContext::Global,
        UiCommand::Quit,
        "q quit (confirm, Ctrl+C prompts)",
    ),
    (
        HelpHintContext::Global,
        UiCommand::ToggleFocus,
        "Tab/h/l switch pane",
    ),
    (
        HelpHintContext::Global,
        UiCommand::ToggleSidebar,
        "\\ toggle sidebar",
    ),
    (
        HelpHintContext::Global,
        UiCommand::ToggleMouseCapture,
        "M toggle mouse capture",
    ),
    (
        HelpHintContext::Global,
        UiCommand::ResizeSidebarNarrower,
        "Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback)",
    ),
    (
        HelpHintContext::Global,
        UiCommand::FocusList,
        "Esc list pane",
    ),
    (
        HelpHintContext::Global,
        UiCommand::MoveSelectionDown,
        "Alt+J/K workspace",
    ),
    (
        HelpHintContext::Global,
        UiCommand::PreviousTab,
        "Alt+[ prev tab",
    ),
    (
        HelpHintContext::Global,
        UiCommand::NextTab,
        "Alt+] next tab",
    ),
    (
        HelpHintContext::Global,
        UiCommand::OpenPreview,
        "Enter open/attach",
    ),
    (
        HelpHintContext::Global,
        UiCommand::OpenCommandPalette,
        "Ctrl+K command palette",
    ),
    (HelpHintContext::Workspace, UiCommand::NewWorkspace, "n new"),
    (
        HelpHintContext::Workspace,
        UiCommand::EditWorkspace,
        "e edit/switch",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::MergeWorkspace,
        "m merge",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::UpdateFromBase,
        "u update",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::RefreshWorkspaces,
        "R refresh",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::OpenProjects,
        "p projects",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::ReorderProjects,
        "Ctrl+R reorder projects",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::DeleteWorkspace,
        "D delete",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::OpenSettings,
        "S settings",
    ),
    (
        HelpHintContext::Workspace,
        UiCommand::ToggleUnsafe,
        "! unsafe toggle",
    ),
    (
        HelpHintContext::List,
        UiCommand::MoveSelectionDown,
        "j/k or Up/Down move selection",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::PreviousTab,
        "[ prev tab",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::PreviousTab,
        "[ prev tab",
    ),
    (
        HelpHintContext::PreviewGit,
        UiCommand::PreviousTab,
        "[ prev tab",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::NextTab,
        "] next tab",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::NextTab,
        "] next tab",
    ),
    (
        HelpHintContext::PreviewGit,
        UiCommand::NextTab,
        "] next tab",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::EnterInteractive,
        "Enter attach shell/agent",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::EnterInteractive,
        "Enter attach shell",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::ScrollDown,
        "j/k or Up/Down scroll",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::ScrollDown,
        "j/k or Up/Down scroll",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::PageDown,
        "PgUp/PgDn page",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::PageDown,
        "PgUp/PgDn page",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::ScrollBottom,
        "G or End bottom",
    ),
    (
        HelpHintContext::PreviewShell,
        UiCommand::ScrollBottom,
        "G or End bottom",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::StartAgent,
        "s start",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::StopAgent,
        "x stop (confirm)",
    ),
    (
        HelpHintContext::PreviewAgent,
        UiCommand::RestartAgent,
        "r restart",
    ),
    (
        HelpHintContext::PreviewGit,
        UiCommand::EnterInteractive,
        "Enter attach lazygit",
    ),
];

impl UiCommand {
    pub(super) fn help_hints_for(context: HelpHintContext) -> Vec<UiCommand> {
        HELP_HINT_ENTRIES
            .iter()
            .filter_map(|(entry_context, command, _)| {
                if *entry_context == context {
                    Some(*command)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn help_hint_label(self, context: HelpHintContext) -> Option<&'static str> {
        HELP_HINT_ENTRIES
            .iter()
            .find_map(|(entry_context, command, label)| {
                if *entry_context == context && *command == self {
                    Some(*label)
                } else {
                    None
                }
            })
    }
}
