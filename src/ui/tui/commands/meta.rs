use super::*;

static COMMAND_META: [UiCommandMeta; 51] = [
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:toggle_focus",
            title: "Toggle Pane Focus",
            description: "Switch focus between workspace list and preview (Tab/h/l)",
            tags: &["tab", "h", "l", "focus", "pane"],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Tab/h/l switch pane",
            key: "Tab/h/l",
            action: "switch pane",
        }],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:toggle_sidebar",
            title: "Toggle Sidebar",
            description: "Show or hide workspace sidebar (Ctrl+B)",
            tags: &["sidebar", "layout", "ctrl+b", "toggle"],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Ctrl+B toggle sidebar",
            key: "Ctrl+B",
            action: "toggle sidebar",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::CtrlChar('b'),
            modifiers: KeyModifiersMatch::None,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_preview",
            title: "Open Preview",
            description: "Focus preview pane, launch shell preview when needed (Enter/l)",
            tags: &["open", "preview", "enter", "l"],
            category: "List",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Enter open/attach",
            key: "Enter",
            action: "open/attach",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Enter,
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:enter_interactive",
            title: "Enter Interactive Mode",
            description: "Attach to selected preview session (agent, shell, or lazygit) (Enter)",
            tags: &[
                "interactive",
                "attach",
                "shell",
                "agent",
                "lazygit",
                "enter",
            ],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "Enter attach shell/agent",
                key: "Enter",
                action: "attach shell/agent",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "Enter attach shell",
                key: "Enter",
                action: "attach shell",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "Enter attach lazygit",
                key: "Enter",
                action: "attach lazygit",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Enter,
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: None,
        help_hints: &[],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:focus_list",
            title: "Focus Workspace List",
            description: "Return focus to workspace list (Esc)",
            tags: &["list", "focus", "esc"],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Esc list pane",
            key: "Esc",
            action: "list pane",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Escape,
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:move_selection_up",
            title: "Select Previous Workspace",
            description: "Move workspace selection up (k / Up / Alt+K)",
            tags: &["up", "previous", "workspace", "k", "alt+k"],
            category: "List",
        }),
        help_hints: &[],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:move_selection_down",
            title: "Select Next Workspace",
            description: "Move workspace selection down (j / Down / Alt+J)",
            tags: &["down", "next", "workspace", "j", "alt+j"],
            category: "List",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Global,
                label: "Alt+J/K workspace",
                key: "Alt+J/K",
                action: "workspace",
            },
            HelpHintSpec {
                context: HelpHintContext::List,
                label: "j/k or Up/Down move selection",
                key: "j/k or Up/Down",
                action: "move selection",
            },
        ],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:scroll_up",
            title: "Scroll Up",
            description: "Scroll preview output up (k / Up)",
            tags: &["scroll", "up", "k"],
            category: "Preview",
        }),
        help_hints: &[],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:scroll_down",
            title: "Scroll Down",
            description: "Scroll preview output down (j / Down)",
            tags: &["scroll", "down", "j"],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "j/k or Up/Down scroll",
                key: "j/k or Up/Down",
                action: "scroll",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "j/k or Up/Down scroll",
                key: "j/k or Up/Down",
                action: "scroll",
            },
        ],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:page_up",
            title: "Page Up",
            description: "Scroll preview up by one page (PgUp)",
            tags: &["pageup", "pgup", "scroll"],
            category: "Preview",
        }),
        help_hints: &[],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::PageUp,
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:page_down",
            title: "Page Down",
            description: "Scroll preview down by one page (PgDn)",
            tags: &["pagedown", "pgdn", "scroll"],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "PgUp/PgDn page",
                key: "PgUp/PgDn",
                action: "page",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "PgUp/PgDn page",
                key: "PgUp/PgDn",
                action: "page",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::PageDown,
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:scroll_bottom",
            title: "Jump To Bottom",
            description: "Jump preview output to bottom (G / End)",
            tags: &["bottom", "latest", "G", "End"],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "G or End bottom",
                key: "G or End",
                action: "bottom",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "G or End bottom",
                key: "G or End",
                action: "bottom",
            },
        ],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:previous_tab",
            title: "Previous Preview Tab",
            description: "Switch to previous preview tab (Alt+[ global, [ in preview)",
            tags: &[
                "previous", "tab", "[", "alt+[", "agent", "shell", "git", "lazygit",
            ],
            category: "Navigation",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Global,
                label: "Alt+[ prev tab",
                key: "Alt+[",
                action: "prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "[ prev tab",
                key: "[",
                action: "prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "[ prev tab",
                key: "[",
                action: "prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "[ prev tab",
                key: "[",
                action: "prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "[ prev tab",
                key: "[",
                action: "prev tab",
            },
        ],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:next_tab",
            title: "Next Preview Tab",
            description: "Switch to next preview tab (Alt+] global, ] in preview)",
            tags: &[
                "next", "tab", "]", "alt+]", "agent", "shell", "git", "lazygit",
            ],
            category: "Navigation",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Global,
                label: "Alt+] next tab",
                key: "Alt+]",
                action: "next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "] next tab",
                key: "]",
                action: "next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "] next tab",
                key: "]",
                action: "next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "] next tab",
                key: "]",
                action: "next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "] next tab",
                key: "]",
                action: "next tab",
            },
        ],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:move_tab_left",
            title: "Move Preview Tab Left",
            description: "Move the active preview tab left ({)",
            tags: &["move", "tab", "left", "{", "reorder", "preview"],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "{ move tab left",
                key: "{",
                action: "move tab left",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "{ move tab left",
                key: "{",
                action: "move tab left",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "{ move tab left",
                key: "{",
                action: "move tab left",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "{ move tab left",
                key: "{",
                action: "move tab left",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('{'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:move_tab_right",
            title: "Move Preview Tab Right",
            description: "Move the active preview tab right (})",
            tags: &["move", "tab", "right", "}", "reorder", "preview"],
            category: "Preview",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "} move tab right",
                key: "}",
                action: "move tab right",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "} move tab right",
                key: "}",
                action: "move tab right",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "} move tab right",
                key: "}",
                action: "move tab right",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "} move tab right",
                key: "}",
                action: "move tab right",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('}'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:resize_sidebar_narrower",
            title: "Resize Sidebar Narrower",
            description: "Shrink sidebar and widen preview (Alt+Left, Alt+H, Alt+B)",
            tags: &[
                "resize", "sidebar", "preview", "left", "alt+left", "h", "alt+h", "b", "alt+b",
            ],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback)",
            key: "Alt+Left/Right or Alt+H/L",
            action: "resize sidebar (Alt+B/F fallback)",
        }],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:resize_sidebar_wider",
            title: "Resize Sidebar Wider",
            description: "Widen sidebar and shrink preview (Alt+Right, Alt+L, Alt+F)",
            tags: &[
                "resize",
                "sidebar",
                "preview",
                "right",
                "alt+right",
                "l",
                "alt+l",
                "f",
                "alt+f",
            ],
            category: "Navigation",
        }),
        help_hints: &[],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:new_workspace",
            title: "New Task",
            description: "Open task creation dialog (n)",
            tags: &["new", "task", "repository", "create", "n"],
            category: "Task",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "n new task",
            key: "n",
            action: "new task",
        }],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:add_worktree",
            title: "Add Worktree",
            description: "Add a repository worktree to the selected task (a)",
            tags: &["add", "worktree", "task", "repository", "a"],
            category: "Worktree",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "a add worktree",
            key: "a",
            action: "add worktree",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('a'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:edit_workspace",
            title: "Edit Workspace",
            description: "Open workspace edit dialog (base branch, or branch switch on main) (e)",
            tags: &["edit", "workspace", "base", "branch", "switch", "e"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "e edit/switch",
            key: "e",
            action: "edit/switch",
        }],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:start_agent",
            title: "New Agent Tab",
            description: "Open agent picker and launch a new agent tab (a)",
            tags: &["new", "agent", "tab", "workspace", "a"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "a new agent tab",
                key: "a",
                action: "new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "a new agent tab",
                key: "a",
                action: "new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "a new agent tab",
                key: "a",
                action: "new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "a new agent tab",
                key: "a",
                action: "new agent tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('a'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:start_parent_agent",
            title: "Start Parent Agent",
            description: "Open the task-root parent agent launch dialog (A)",
            tags: &["parent", "agent", "task", "root", "A"],
            category: "Task",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "A start parent agent",
            key: "A",
            action: "start parent agent",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('A'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:new_shell_tab",
            title: "New Shell Tab",
            description: "Launch a new shell tab for the selected workspace (s)",
            tags: &["new", "shell", "tab", "workspace", "s"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Workspace,
                label: "s new shell tab",
                key: "s",
                action: "new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "s new shell tab",
                key: "s",
                action: "new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "s new shell tab",
                key: "s",
                action: "new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "s new shell tab",
                key: "s",
                action: "new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "s new shell tab",
                key: "s",
                action: "new shell tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('s'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_git_tab",
            title: "Open Git Tab",
            description: "Focus existing git tab or launch it if missing (g)",
            tags: &["git", "tab", "lazygit", "workspace", "g"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Workspace,
                label: "g git tab",
                key: "g",
                action: "git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "g git tab",
                key: "g",
                action: "git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "g git tab",
                key: "g",
                action: "git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "g git tab",
                key: "g",
                action: "git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "g git tab",
                key: "g",
                action: "git tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('g'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_diff_tab",
            title: "Open Diff Tab",
            description: "Show live git diff for the selected workspace (d)",
            tags: &["diff", "git", "changes", "tab", "workspace", "d"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Workspace,
                label: "d diff tab",
                key: "d",
                action: "diff tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "d diff tab",
                key: "d",
                action: "diff tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "d diff tab",
                key: "d",
                action: "diff tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "d diff tab",
                key: "d",
                action: "diff tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "d diff tab",
                key: "d",
                action: "diff tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('d'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:rename_active_tab",
            title: "Rename Active Tab",
            description: "Rename the active tab title (,)",
            tags: &["rename", "tab", "title", ","],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::Workspace,
                label: ", rename tab",
                key: ",",
                action: "rename tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: ", rename tab",
                key: ",",
                action: "rename tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: ", rename tab",
                key: ",",
                action: "rename tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: ", rename tab",
                key: ",",
                action: "rename tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: ", rename tab",
                key: ",",
                action: "rename tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char(','),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: None,
        help_hints: &[],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:close_active_tab",
            title: "Close Active Tab",
            description: "Close active tab, confirm kill+close when session is live (x)",
            tags: &["close", "tab", "session", "x"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "x close tab",
                key: "x",
                action: "close tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "x close tab",
                key: "x",
                action: "close tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "x close tab",
                key: "x",
                action: "close tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewDiff,
                label: "x close tab",
                key: "x",
                action: "close tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('x'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:delete_workspace",
            title: "Delete Task",
            description: "Delete the selected task and all of its worktrees (D)",
            tags: &["delete", "task", "worktree", "D"],
            category: "Task",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "D delete task",
            key: "D",
            action: "delete task",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('D'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:delete_worktree",
            title: "Delete Worktree",
            description: "Delete the selected worktree, or the task if it is the last one (d)",
            tags: &["delete", "worktree", "task", "d"],
            category: "Worktree",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "d delete worktree",
            key: "d",
            action: "delete worktree",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('d'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:merge_workspace",
            title: "Merge Worktree",
            description: "Merge selected worktree branch into base (m)",
            tags: &["merge", "worktree", "branch", "m"],
            category: "Worktree",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "m merge worktree",
            key: "m",
            action: "merge worktree",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('m'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:update_from_base",
            title: "Update Worktree",
            description: "Update selected worktree (feature: merge base, base: pull origin) (u)",
            tags: &["update", "sync", "base", "worktree", "u"],
            category: "Worktree",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "u update worktree",
            key: "u",
            action: "update worktree",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('u'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:pull_upstream",
            title: "Pull Upstream",
            description: "Pull upstream and propagate to workspaces (U)",
            tags: &["pull", "upstream", "sync", "fetch", "U"],
            category: "Task",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "U pull upstream + propagate",
            key: "U",
            action: "pull upstream + propagate",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('U'),
            modifiers: KeyModifiersMatch::None,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:refresh_workspaces",
            title: "Refresh Tasks",
            description: "Refresh tasks, worktrees, and PR metadata (R)",
            tags: &["refresh", "task", "worktree", "pull request", "github", "R"],
            category: "Task",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "R refresh",
            key: "R",
            action: "refresh",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('R'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_projects",
            title: "Projects",
            description: "Open projects dialog (switch, add/remove, edit defaults) (p)",
            tags: &["projects", "project", "switcher", "defaults", "setup", "p"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "p projects",
            key: "p",
            action: "projects",
        }],
        keybindings: &[
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
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:reorder_tasks",
            title: "Reorder Tasks",
            description: "Reorder task groups in the sidebar (Ctrl+R, j/k or Up/Down, Enter/Esc)",
            tags: &["tasks", "task", "reorder", "move", "ctrl+r", "up", "down"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::List,
            label: "Ctrl+R reorder tasks",
            key: "Ctrl+R",
            action: "reorder tasks",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::CtrlChar('r'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:delete_project",
            title: "Remove Selected Project",
            description: "Remove selected workspace project (Ctrl+X/Del)",
            tags: &[
                "remove",
                "delete",
                "project",
                "workspace list",
                "ctrl+x",
                "del",
            ],
            category: "Workspace",
        }),
        help_hints: &[],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_settings",
            title: "Settings",
            description: "Open settings dialog (S)",
            tags: &["settings", "multiplexer", "S"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "S settings",
            key: "S",
            action: "settings",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('S'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:toggle_mouse_capture",
            title: "Toggle Mouse Capture",
            description: "Toggle Grove mouse handling to allow terminal URL click/select (M)",
            tags: &["mouse", "capture", "url", "click", "select", "M"],
            category: "System",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "M toggle mouse capture",
            key: "M",
            action: "toggle mouse capture",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('M'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:toggle_unsafe",
            title: "Toggle Unsafe Launch",
            description: "Cycle launch permission mode (! default/auto/unsafe)",
            tags: &["unsafe", "permissions", "!", "auto"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "! unsafe toggle",
            key: "!",
            action: "unsafe toggle",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('!'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:focus_attention_inbox",
            title: "Focus Needs You Inbox",
            description: "Select the highest-priority attention item in the sidebar",
            tags: &["attention", "inbox", "needs you", "focus", "sidebar"],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "i focus needs you inbox",
            key: "i",
            action: "focus needs you inbox",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('i'),
            modifiers: KeyModifiersMatch::None,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:acknowledge_attention",
            title: "Acknowledge Attention Item",
            description: "Acknowledge the selected inbox item and clear it",
            tags: &["attention", "acknowledge", "clear", "needs you", "sidebar"],
            category: "Attention",
        }),
        help_hints: &[],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:cleanup_sessions",
            title: "Cleanup Sessions",
            description: "Review and clean orphaned Grove tmux sessions",
            tags: &["cleanup", "sessions", "tmux", "orphaned", "stale"],
            category: "System",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Palette cleanup sessions",
            key: "Palette",
            action: "cleanup sessions",
        }],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_help",
            title: "Keybind Help",
            description: "Open keyboard shortcut help (?)",
            tags: &["help", "shortcuts", "?"],
            category: "System",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "? help",
            key: "?",
            action: "help",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('?'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: None,
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Ctrl+K command palette",
            key: "Ctrl+K",
            action: "command palette",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::CtrlChar('k'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:quit",
            title: "Quit Grove",
            description: "Open confirm dialog to exit application (q, Ctrl+C prompts)",
            tags: &["quit", "exit", "q", "ctrl+c"],
            category: "System",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "q quit (confirm, Ctrl+C prompts)",
            key: "q",
            action: "quit (confirm, Ctrl+C prompts)",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('q'),
            modifiers: KeyModifiersMatch::None,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_performance",
            title: "Performance",
            description: "Inspect Grove runtime stats and polling behavior",
            tags: &["performance", "perf", "profile", "profiling", "stats"],
            category: "System",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "Palette performance",
            key: "Palette",
            action: "performance",
        }],
        keybindings: &[],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:open_repository",
            title: "Open Repository",
            description: "Open the selected repository remote in your browser (o)",
            tags: &[
                "open",
                "repository",
                "repo",
                "remote",
                "browser",
                "origin",
                "o",
            ],
            category: "Worktree",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "o open repository",
            key: "o",
            action: "open repository",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('o'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: None,
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "/ open workspace jump",
            key: "/",
            action: "open workspace jump",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('/'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
];

impl UiCommand {
    pub(super) fn meta(self) -> &'static UiCommandMeta {
        match self {
            UiCommand::ToggleFocus => &COMMAND_META[0],
            UiCommand::ToggleSidebar => &COMMAND_META[1],
            UiCommand::OpenPreview => &COMMAND_META[2],
            UiCommand::EnterInteractive => &COMMAND_META[3],
            UiCommand::FocusPreview => &COMMAND_META[4],
            UiCommand::FocusList => &COMMAND_META[5],
            UiCommand::MoveSelectionUp => &COMMAND_META[6],
            UiCommand::MoveSelectionDown => &COMMAND_META[7],
            UiCommand::ScrollUp => &COMMAND_META[8],
            UiCommand::ScrollDown => &COMMAND_META[9],
            UiCommand::PageUp => &COMMAND_META[10],
            UiCommand::PageDown => &COMMAND_META[11],
            UiCommand::ScrollBottom => &COMMAND_META[12],
            UiCommand::PreviousTab => &COMMAND_META[13],
            UiCommand::NextTab => &COMMAND_META[14],
            UiCommand::MoveTabLeft => &COMMAND_META[15],
            UiCommand::MoveTabRight => &COMMAND_META[16],
            UiCommand::ResizeSidebarNarrower => &COMMAND_META[17],
            UiCommand::ResizeSidebarWider => &COMMAND_META[18],
            UiCommand::NewWorkspace => &COMMAND_META[19],
            UiCommand::AddWorktree => &COMMAND_META[20],
            UiCommand::EditWorkspace => &COMMAND_META[21],
            UiCommand::StartAgent => &COMMAND_META[22],
            UiCommand::StartParentAgent => &COMMAND_META[23],
            UiCommand::OpenShellTab => &COMMAND_META[24],
            UiCommand::OpenGitTab => &COMMAND_META[25],
            UiCommand::OpenDiffTab => &COMMAND_META[26],
            UiCommand::RenameActiveTab => &COMMAND_META[27],
            UiCommand::StopAgent => &COMMAND_META[28],
            UiCommand::RestartAgent => &COMMAND_META[29],
            UiCommand::DeleteWorkspace => &COMMAND_META[30],
            UiCommand::DeleteWorktree => &COMMAND_META[31],
            UiCommand::MergeWorkspace => &COMMAND_META[32],
            UiCommand::UpdateFromBase => &COMMAND_META[33],
            UiCommand::PullUpstream => &COMMAND_META[34],
            UiCommand::RefreshWorkspaces => &COMMAND_META[35],
            UiCommand::OpenProjects => &COMMAND_META[36],
            UiCommand::ReorderTasks => &COMMAND_META[37],
            UiCommand::DeleteProject => &COMMAND_META[38],
            UiCommand::OpenSettings => &COMMAND_META[39],
            UiCommand::ToggleMouseCapture => &COMMAND_META[40],
            UiCommand::ToggleUnsafe => &COMMAND_META[41],
            UiCommand::FocusAttentionInbox => &COMMAND_META[42],
            UiCommand::AcknowledgeAttention => &COMMAND_META[43],
            UiCommand::CleanupSessions => &COMMAND_META[44],
            UiCommand::OpenHelp => &COMMAND_META[45],
            UiCommand::OpenCommandPalette => &COMMAND_META[46],
            UiCommand::Quit => &COMMAND_META[47],
            UiCommand::OpenPerformance => &COMMAND_META[48],
            UiCommand::OpenRepository => &COMMAND_META[49],
            UiCommand::OpenWorkspaceJump => &COMMAND_META[50],
        }
    }
}
