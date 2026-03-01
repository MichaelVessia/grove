#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayMsg {
    Key {
        event: ReplayKeyEvent,
    },
    Mouse {
        event: ReplayMouseEvent,
    },
    Paste {
        event: ReplayPasteEvent,
    },
    Tick,
    Resize {
        width: u16,
        height: u16,
    },
    PreviewPollCompleted {
        completion: ReplayPreviewPollCompletion,
    },
    LazygitLaunchCompleted {
        completion: ReplayLazygitLaunchCompletion,
    },
    WorkspaceShellLaunchCompleted {
        completion: ReplayWorkspaceShellLaunchCompletion,
    },
    RefreshWorkspacesCompleted {
        completion: ReplayRefreshWorkspacesCompletion,
    },
    DeleteProjectCompleted {
        completion: ReplayDeleteProjectCompletion,
    },
    DeleteWorkspaceCompleted {
        completion: ReplayDeleteWorkspaceCompletion,
    },
    MergeWorkspaceCompleted {
        completion: ReplayMergeWorkspaceCompletion,
    },
    UpdateWorkspaceFromBaseCompleted {
        completion: ReplayUpdateWorkspaceFromBaseCompletion,
    },
    CreateWorkspaceCompleted {
        completion: ReplayCreateWorkspaceCompletion,
    },
    StartAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    StopAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    RestartAgentCompleted {
        completion: ReplaySessionCompletion,
    },
    InteractiveSendCompleted {
        completion: ReplayInteractiveSendCompletion,
    },
    Noop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayKeyEvent {
    code: ReplayKeyCode,
    modifiers: u8,
    kind: ReplayKeyEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayKeyCode {
    Char { value: char },
    Enter,
    Escape,
    Backspace,
    Tab,
    BackTab,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Function { value: u8 },
    Null,
    MediaPlayPause,
    MediaStop,
    MediaNextTrack,
    MediaPrevTrack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayKeyEventKind {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayMouseEvent {
    kind: ReplayMouseEventKind,
    x: u16,
    y: u16,
    modifiers: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReplayMouseEventKind {
    Down { button: ReplayMouseButton },
    Up { button: ReplayMouseButton },
    Drag { button: ReplayMouseButton },
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReplayMouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayPasteEvent {
    text: String,
    bracketed: bool,
}

impl ReplayMsg {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Key { .. } => "key",
            Self::Mouse { .. } => "mouse",
            Self::Paste { .. } => "paste",
            Self::Tick => "tick",
            Self::Resize { .. } => "resize",
            Self::PreviewPollCompleted { .. } => "preview_poll_completed",
            Self::LazygitLaunchCompleted { .. } => "lazygit_launch_completed",
            Self::WorkspaceShellLaunchCompleted { .. } => "workspace_shell_launch_completed",
            Self::RefreshWorkspacesCompleted { .. } => "refresh_workspaces_completed",
            Self::DeleteProjectCompleted { .. } => "delete_project_completed",
            Self::DeleteWorkspaceCompleted { .. } => "delete_workspace_completed",
            Self::MergeWorkspaceCompleted { .. } => "merge_workspace_completed",
            Self::UpdateWorkspaceFromBaseCompleted { .. } => "update_workspace_from_base_completed",
            Self::CreateWorkspaceCompleted { .. } => "create_workspace_completed",
            Self::StartAgentCompleted { .. } => "start_agent_completed",
            Self::StopAgentCompleted { .. } => "stop_agent_completed",
            Self::RestartAgentCompleted { .. } => "restart_agent_completed",
            Self::InteractiveSendCompleted { .. } => "interactive_send_completed",
            Self::Noop => "noop",
        }
    }

    fn from_msg(msg: &Msg) -> Self {
        match msg {
            Msg::Key(event) => Self::Key {
                event: ReplayKeyEvent::from_key_event(event),
            },
            Msg::Mouse(event) => Self::Mouse {
                event: ReplayMouseEvent::from_mouse_event(event),
            },
            Msg::Paste(event) => Self::Paste {
                event: ReplayPasteEvent::from_paste_event(event),
            },
            Msg::Tick => Self::Tick,
            Msg::Resize { width, height } => Self::Resize {
                width: *width,
                height: *height,
            },
            Msg::PreviewPollCompleted(completion) => Self::PreviewPollCompleted {
                completion: ReplayPreviewPollCompletion::from_completion(completion),
            },
            Msg::LazygitLaunchCompleted(completion) => Self::LazygitLaunchCompleted {
                completion: ReplayLazygitLaunchCompletion::from_completion(completion),
            },
            Msg::WorkspaceShellLaunchCompleted(completion) => Self::WorkspaceShellLaunchCompleted {
                completion: ReplayWorkspaceShellLaunchCompletion::from_completion(completion),
            },
            Msg::RefreshWorkspacesCompleted(completion) => Self::RefreshWorkspacesCompleted {
                completion: ReplayRefreshWorkspacesCompletion::from_completion(completion),
            },
            Msg::DeleteProjectCompleted(completion) => Self::DeleteProjectCompleted {
                completion: ReplayDeleteProjectCompletion::from_completion(completion),
            },
            Msg::DeleteWorkspaceCompleted(completion) => Self::DeleteWorkspaceCompleted {
                completion: ReplayDeleteWorkspaceCompletion::from_completion(completion),
            },
            Msg::MergeWorkspaceCompleted(completion) => Self::MergeWorkspaceCompleted {
                completion: ReplayMergeWorkspaceCompletion::from_completion(completion),
            },
            Msg::UpdateWorkspaceFromBaseCompleted(completion) => {
                Self::UpdateWorkspaceFromBaseCompleted {
                    completion: ReplayUpdateWorkspaceFromBaseCompletion::from_completion(
                        completion,
                    ),
                }
            }
            Msg::CreateWorkspaceCompleted(completion) => Self::CreateWorkspaceCompleted {
                completion: ReplayCreateWorkspaceCompletion::from_completion(completion),
            },
            Msg::StartAgentCompleted(completion) => Self::StartAgentCompleted {
                completion: ReplaySessionCompletion::from_start_completion(completion),
            },
            Msg::StopAgentCompleted(completion) => Self::StopAgentCompleted {
                completion: ReplaySessionCompletion::from_stop_completion(completion),
            },
            Msg::RestartAgentCompleted(completion) => Self::RestartAgentCompleted {
                completion: ReplaySessionCompletion::from_restart_completion(completion),
            },
            Msg::InteractiveSendCompleted(completion) => Self::InteractiveSendCompleted {
                completion: ReplayInteractiveSendCompletion::from_completion(completion),
            },
            Msg::Noop => Self::Noop,
        }
    }

    fn to_msg(&self) -> Msg {
        match self {
            Self::Key { event } => Msg::Key(event.to_key_event()),
            Self::Mouse { event } => Msg::Mouse(event.to_mouse_event()),
            Self::Paste { event } => Msg::Paste(event.to_paste_event()),
            Self::Tick => Msg::Tick,
            Self::Resize { width, height } => Msg::Resize {
                width: *width,
                height: *height,
            },
            Self::PreviewPollCompleted { completion } => {
                Msg::PreviewPollCompleted(completion.to_completion())
            }
            Self::LazygitLaunchCompleted { completion } => {
                Msg::LazygitLaunchCompleted(completion.to_completion())
            }
            Self::WorkspaceShellLaunchCompleted { completion } => {
                Msg::WorkspaceShellLaunchCompleted(completion.to_completion())
            }
            Self::RefreshWorkspacesCompleted { completion } => {
                Msg::RefreshWorkspacesCompleted(completion.to_completion())
            }
            Self::DeleteProjectCompleted { completion } => {
                Msg::DeleteProjectCompleted(completion.to_completion())
            }
            Self::DeleteWorkspaceCompleted { completion } => {
                Msg::DeleteWorkspaceCompleted(completion.to_completion())
            }
            Self::MergeWorkspaceCompleted { completion } => {
                Msg::MergeWorkspaceCompleted(completion.to_completion())
            }
            Self::UpdateWorkspaceFromBaseCompleted { completion } => {
                Msg::UpdateWorkspaceFromBaseCompleted(completion.to_completion())
            }
            Self::CreateWorkspaceCompleted { completion } => {
                Msg::CreateWorkspaceCompleted(completion.to_completion())
            }
            Self::StartAgentCompleted { completion } => {
                Msg::StartAgentCompleted(completion.to_start_completion())
            }
            Self::StopAgentCompleted { completion } => {
                Msg::StopAgentCompleted(completion.to_stop_completion())
            }
            Self::RestartAgentCompleted { completion } => {
                Msg::RestartAgentCompleted(completion.to_restart_completion())
            }
            Self::InteractiveSendCompleted { completion } => {
                Msg::InteractiveSendCompleted(completion.to_completion())
            }
            Self::Noop => Msg::Noop,
        }
    }
}

impl ReplayKeyEvent {
    fn from_key_event(event: &KeyEvent) -> Self {
        Self {
            code: ReplayKeyCode::from_key_code(event.code),
            modifiers: event.modifiers.bits(),
            kind: ReplayKeyEventKind::from_key_event_kind(event.kind),
        }
    }

    fn to_key_event(&self) -> KeyEvent {
        KeyEvent::new(self.code.to_key_code())
            .with_modifiers(Modifiers::from_bits_retain(self.modifiers))
            .with_kind(self.kind.to_key_event_kind())
    }
}

impl ReplayKeyCode {
    fn from_key_code(code: KeyCode) -> Self {
        match code {
            KeyCode::Char(value) => Self::Char { value },
            KeyCode::Enter => Self::Enter,
            KeyCode::Escape => Self::Escape,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Tab => Self::Tab,
            KeyCode::BackTab => Self::BackTab,
            KeyCode::Delete => Self::Delete,
            KeyCode::Insert => Self::Insert,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::F(value) => Self::Function { value },
            KeyCode::Null => Self::Null,
            KeyCode::MediaPlayPause => Self::MediaPlayPause,
            KeyCode::MediaStop => Self::MediaStop,
            KeyCode::MediaNextTrack => Self::MediaNextTrack,
            KeyCode::MediaPrevTrack => Self::MediaPrevTrack,
        }
    }

    fn to_key_code(&self) -> KeyCode {
        match self {
            Self::Char { value } => KeyCode::Char(*value),
            Self::Enter => KeyCode::Enter,
            Self::Escape => KeyCode::Escape,
            Self::Backspace => KeyCode::Backspace,
            Self::Tab => KeyCode::Tab,
            Self::BackTab => KeyCode::BackTab,
            Self::Delete => KeyCode::Delete,
            Self::Insert => KeyCode::Insert,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PageUp => KeyCode::PageUp,
            Self::PageDown => KeyCode::PageDown,
            Self::Up => KeyCode::Up,
            Self::Down => KeyCode::Down,
            Self::Left => KeyCode::Left,
            Self::Right => KeyCode::Right,
            Self::Function { value } => KeyCode::F(*value),
            Self::Null => KeyCode::Null,
            Self::MediaPlayPause => KeyCode::MediaPlayPause,
            Self::MediaStop => KeyCode::MediaStop,
            Self::MediaNextTrack => KeyCode::MediaNextTrack,
            Self::MediaPrevTrack => KeyCode::MediaPrevTrack,
        }
    }
}

impl ReplayKeyEventKind {
    fn from_key_event_kind(kind: KeyEventKind) -> Self {
        match kind {
            KeyEventKind::Press => Self::Press,
            KeyEventKind::Repeat => Self::Repeat,
            KeyEventKind::Release => Self::Release,
        }
    }

    fn to_key_event_kind(self) -> KeyEventKind {
        match self {
            Self::Press => KeyEventKind::Press,
            Self::Repeat => KeyEventKind::Repeat,
            Self::Release => KeyEventKind::Release,
        }
    }
}

impl ReplayMouseEvent {
    fn from_mouse_event(event: &MouseEvent) -> Self {
        Self {
            kind: ReplayMouseEventKind::from_mouse_event_kind(event.kind),
            x: event.x,
            y: event.y,
            modifiers: event.modifiers.bits(),
        }
    }

    fn to_mouse_event(&self) -> MouseEvent {
        MouseEvent::new(self.kind.to_mouse_event_kind(), self.x, self.y)
            .with_modifiers(Modifiers::from_bits_retain(self.modifiers))
    }
}

impl ReplayMouseEventKind {
    fn from_mouse_event_kind(kind: MouseEventKind) -> Self {
        match kind {
            MouseEventKind::Down(button) => Self::Down {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Up(button) => Self::Up {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Drag(button) => Self::Drag {
                button: ReplayMouseButton::from_mouse_button(button),
            },
            MouseEventKind::Moved => Self::Moved,
            MouseEventKind::ScrollUp => Self::ScrollUp,
            MouseEventKind::ScrollDown => Self::ScrollDown,
            MouseEventKind::ScrollLeft => Self::ScrollLeft,
            MouseEventKind::ScrollRight => Self::ScrollRight,
        }
    }

    fn to_mouse_event_kind(&self) -> MouseEventKind {
        match self {
            Self::Down { button } => MouseEventKind::Down(button.to_mouse_button()),
            Self::Up { button } => MouseEventKind::Up(button.to_mouse_button()),
            Self::Drag { button } => MouseEventKind::Drag(button.to_mouse_button()),
            Self::Moved => MouseEventKind::Moved,
            Self::ScrollUp => MouseEventKind::ScrollUp,
            Self::ScrollDown => MouseEventKind::ScrollDown,
            Self::ScrollLeft => MouseEventKind::ScrollLeft,
            Self::ScrollRight => MouseEventKind::ScrollRight,
        }
    }
}

impl ReplayMouseButton {
    fn from_mouse_button(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
        }
    }

    fn to_mouse_button(self) -> MouseButton {
        match self {
            Self::Left => MouseButton::Left,
            Self::Right => MouseButton::Right,
            Self::Middle => MouseButton::Middle,
        }
    }
}

impl ReplayPasteEvent {
    fn from_paste_event(event: &PasteEvent) -> Self {
        Self {
            text: event.text.clone(),
            bracketed: event.bracketed,
        }
    }

    fn to_paste_event(&self) -> PasteEvent {
        PasteEvent::new(self.text.clone(), self.bracketed)
    }
}
