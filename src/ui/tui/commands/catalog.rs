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

    pub(super) fn keybindings(self) -> &'static [KeybindingSpec] {
        match self {
            UiCommand::ToggleFocus => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Tab,
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('h'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('l'),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::ToggleSidebar => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('\\'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::OpenPreview => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Enter,
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::EnterInteractive => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Enter,
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::FocusPreview => &[],
            UiCommand::FocusList => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Escape,
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::MoveSelectionUp => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('k'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('K'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('k'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Up,
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::MoveSelectionDown => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('j'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('J'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('j'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Down,
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::ScrollUp => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('k'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Up,
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::ScrollDown => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('j'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Down,
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::PageUp => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::PageUp,
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::PageDown => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::PageDown,
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::ScrollBottom => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('G'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::End,
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::PreviousTab => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('['),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('['),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::NextTab => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char(']'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char(']'),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::ResizeSidebarNarrower => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('b'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('B'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('h'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('H'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Left,
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
            ],
            UiCommand::ResizeSidebarWider => &[
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('f'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('F'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('l'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Char('L'),
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
                KeybindingSpec {
                    scope: KeybindingScope::GlobalNavigation,
                    code: KeyCodeMatch::Right,
                    modifiers: KeyModifiersMatch::Contains(Modifiers::ALT),
                },
            ],
            UiCommand::NewWorkspace => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('n'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('N'),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::EditWorkspace => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('e'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('E'),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::StartAgent => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('s'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::StopAgent => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('x'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::RestartAgent => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('r'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::DeleteWorkspace => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('D'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::MergeWorkspace => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('m'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::UpdateFromBase => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('u'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::RefreshWorkspaces => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('R'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::OpenProjects => &[
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('p'),
                    modifiers: KeyModifiersMatch::Any,
                },
                KeybindingSpec {
                    scope: KeybindingScope::NonInteractive,
                    code: KeyCodeMatch::Char('P'),
                    modifiers: KeyModifiersMatch::Any,
                },
            ],
            UiCommand::ReorderProjects => &[],
            UiCommand::DeleteProject => &[],
            UiCommand::OpenSettings => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('S'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::ToggleMouseCapture => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('M'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::ToggleUnsafe => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('!'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::OpenHelp => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('?'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::OpenCommandPalette => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::CtrlChar('k'),
                modifiers: KeyModifiersMatch::Any,
            }],
            UiCommand::Quit => &[KeybindingSpec {
                scope: KeybindingScope::NonInteractive,
                code: KeyCodeMatch::Char('q'),
                modifiers: KeyModifiersMatch::None,
            }],
        }
    }

    pub(super) fn matches_keybinding(self, key_event: &KeyEvent, scope: KeybindingScope) -> bool {
        self.keybindings()
            .iter()
            .any(|binding| binding.scope == scope && binding.matches(key_event))
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
