use super::*;

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
    StartParentAgent,
    OpenShellTab,
    OpenGitTab,
    RenameActiveTab,
    StopAgent,
    RestartAgent,
    DeleteWorkspace,
    MergeWorkspace,
    UpdateFromBase,
    RefreshWorkspaces,
    OpenProjects,
    ReorderTasks,
    DeleteProject,
    OpenSettings,
    ToggleMouseCapture,
    ToggleUnsafe,
    CleanupSessions,
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
pub(super) struct HelpHintSpec {
    pub(super) context: HelpHintContext,
    pub(super) label: &'static str,
    pub(super) key: &'static str,
    pub(super) action: &'static str,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeybindingScope {
    GlobalNavigation,
    NonInteractive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyCodeMatch {
    Char(char),
    Enter,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    End,
    CtrlChar(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyModifiersMatch {
    Any,
    None,
    Contains(Modifiers),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct KeybindingSpec {
    pub(super) scope: KeybindingScope,
    pub(super) code: KeyCodeMatch,
    pub(super) modifiers: KeyModifiersMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UiCommandMeta {
    pub(super) palette: Option<PaletteCommandSpec>,
    pub(super) help_hints: &'static [HelpHintSpec],
    pub(super) keybindings: &'static [KeybindingSpec],
}

impl KeyCodeMatch {
    fn matches(self, key_event: &KeyEvent) -> bool {
        match self {
            Self::Char(expected) => {
                matches!(key_event.code, KeyCode::Char(actual) if actual == expected)
            }
            Self::Enter => key_event.code == KeyCode::Enter,
            Self::Tab => key_event.code == KeyCode::Tab,
            Self::Escape => key_event.code == KeyCode::Escape,
            Self::Up => key_event.code == KeyCode::Up,
            Self::Down => key_event.code == KeyCode::Down,
            Self::Left => key_event.code == KeyCode::Left,
            Self::Right => key_event.code == KeyCode::Right,
            Self::PageUp => key_event.code == KeyCode::PageUp,
            Self::PageDown => key_event.code == KeyCode::PageDown,
            Self::End => key_event.code == KeyCode::End,
            Self::CtrlChar(expected) => {
                if key_event.kind != KeyEventKind::Press {
                    return false;
                }
                let KeyCode::Char(value) = key_event.code else {
                    return false;
                };
                if value.eq_ignore_ascii_case(&expected) && key_event.modifiers == Modifiers::CTRL {
                    return true;
                }
                let Some(control_character) = control_character_for(expected) else {
                    return false;
                };
                value == control_character
                    && (key_event.modifiers.is_empty() || key_event.modifiers == Modifiers::CTRL)
            }
        }
    }
}

impl KeyModifiersMatch {
    fn matches(self, modifiers: Modifiers) -> bool {
        match self {
            Self::Any => true,
            Self::None => modifiers.is_empty(),
            Self::Contains(required) => modifiers.contains(required),
        }
    }
}

impl KeybindingSpec {
    fn matches(self, key_event: &KeyEvent) -> bool {
        self.code.matches(key_event)
            && (matches!(self.code, KeyCodeMatch::CtrlChar(_))
                || self.modifiers.matches(key_event.modifiers))
    }
}

fn control_character_for(character: char) -> Option<char> {
    let normalized = character.to_ascii_lowercase();
    if !normalized.is_ascii_lowercase() {
        return None;
    }
    let normalized_code = u32::from(normalized);
    let a_code = u32::from('a');
    let offset = normalized_code.checked_sub(a_code)?;
    let control_code = offset.checked_add(1)?;
    char::from_u32(control_code)
}

impl UiCommand {
    pub(super) const ALL: [UiCommand; 40] = [
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
        UiCommand::StartParentAgent,
        UiCommand::OpenShellTab,
        UiCommand::OpenGitTab,
        UiCommand::RenameActiveTab,
        UiCommand::StopAgent,
        UiCommand::RestartAgent,
        UiCommand::DeleteWorkspace,
        UiCommand::MergeWorkspace,
        UiCommand::UpdateFromBase,
        UiCommand::RefreshWorkspaces,
        UiCommand::OpenProjects,
        UiCommand::ReorderTasks,
        UiCommand::DeleteProject,
        UiCommand::OpenSettings,
        UiCommand::ToggleMouseCapture,
        UiCommand::ToggleUnsafe,
        UiCommand::CleanupSessions,
        UiCommand::OpenHelp,
        UiCommand::OpenCommandPalette,
        UiCommand::Quit,
    ];

    pub(super) fn all() -> &'static [UiCommand] {
        &Self::ALL
    }

    pub(super) fn keybindings(self) -> &'static [KeybindingSpec] {
        self.meta().keybindings
    }

    pub(super) fn matches_keybinding(self, key_event: &KeyEvent, scope: KeybindingScope) -> bool {
        self.keybindings()
            .iter()
            .any(|binding| binding.scope == scope && binding.matches(key_event))
    }

    pub(super) fn from_palette_id(id: &str) -> Option<Self> {
        for command in Self::all() {
            if let Some(spec) = command.meta().palette
                && spec.id == id
            {
                return Some(*command);
            }
        }
        None
    }
}
