use super::*;

static COMMAND_META: [UiCommandMeta; 38] = [
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
            description: "Show or hide workspace sidebar (\\)",
            tags: &["sidebar", "layout", "\\", "toggle"],
            category: "Navigation",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Global,
            label: "\\ toggle sidebar",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('\\'),
            modifiers: KeyModifiersMatch::Any,
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "Enter attach shell",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "Enter attach lazygit",
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
            },
            HelpHintSpec {
                context: HelpHintContext::List,
                label: "j/k or Up/Down move selection",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "j/k or Up/Down scroll",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "PgUp/PgDn page",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "G or End bottom",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "[ prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "[ prev tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "[ prev tab",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "] next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "] next tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "] next tab",
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
            title: "New Workspace",
            description: "Open workspace creation dialog (n)",
            tags: &["new", "workspace", "create", "n"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "n new",
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
            id: "palette:edit_workspace",
            title: "Edit Workspace",
            description: "Open workspace edit dialog (agent + base branch, or base branch switch on main) (e)",
            tags: &[
                "edit",
                "workspace",
                "agent",
                "base",
                "branch",
                "switch",
                "e",
            ],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "e edit/switch",
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
                context: HelpHintContext::Workspace,
                label: "a new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "a new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "a new agent tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "a new agent tab",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "s new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "s new shell tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "s new shell tab",
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
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "g git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "g git tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "g git tab",
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
            id: "palette:kill_active_tab_session",
            title: "Kill Active Tab Session",
            description: "Kill whatever is running in the active tab (x)",
            tags: &["kill", "stop", "session", "tab", "x"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "x kill session",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "x kill session",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "x kill session",
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
            id: "palette:close_active_tab",
            title: "Close Active Tab",
            description: "Close active tab, confirm kill+close when session is live (X)",
            tags: &["close", "tab", "session", "X"],
            category: "Workspace",
        }),
        help_hints: &[
            HelpHintSpec {
                context: HelpHintContext::PreviewAgent,
                label: "X close tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewShell,
                label: "X close tab",
            },
            HelpHintSpec {
                context: HelpHintContext::PreviewGit,
                label: "X close tab",
            },
        ],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('X'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:delete_workspace",
            title: "Delete Workspace",
            description: "Open delete dialog for selected workspace (D)",
            tags: &["delete", "workspace", "worktree", "D"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "D delete",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('D'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:merge_workspace",
            title: "Merge Workspace",
            description: "Merge selected workspace branch into base (m)",
            tags: &["merge", "workspace", "branch", "m"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "m merge",
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
            title: "Update From Base",
            description: "Update selected workspace (feature: merge base, base: pull origin) (u)",
            tags: &["update", "sync", "base", "workspace", "u"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "u update",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('u'),
            modifiers: KeyModifiersMatch::Any,
        }],
    },
    UiCommandMeta {
        palette: Some(PaletteCommandSpec {
            id: "palette:refresh_workspaces",
            title: "Refresh Workspaces",
            description: "Refresh workspaces and PR metadata from git + GitHub (R)",
            tags: &["refresh", "workspace", "pull request", "github", "R"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "R refresh",
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
            id: "palette:reorder_projects",
            title: "Reorder Projects",
            description: "Open projects dialog in reorder mode (Ctrl+R, j/k or Up/Down, Enter/Esc)",
            tags: &[
                "projects", "project", "reorder", "move", "ctrl+r", "up", "down",
            ],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "Ctrl+R reorder projects",
        }],
        keybindings: &[],
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
            description: "Toggle launch skip-permissions default (!)",
            tags: &["unsafe", "permissions", "!"],
            category: "Workspace",
        }),
        help_hints: &[HelpHintSpec {
            context: HelpHintContext::Workspace,
            label: "! unsafe toggle",
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('!'),
            modifiers: KeyModifiersMatch::Any,
        }],
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
        }],
        keybindings: &[KeybindingSpec {
            scope: KeybindingScope::NonInteractive,
            code: KeyCodeMatch::Char('q'),
            modifiers: KeyModifiersMatch::None,
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
            UiCommand::ResizeSidebarNarrower => &COMMAND_META[15],
            UiCommand::ResizeSidebarWider => &COMMAND_META[16],
            UiCommand::NewWorkspace => &COMMAND_META[17],
            UiCommand::EditWorkspace => &COMMAND_META[18],
            UiCommand::StartAgent => &COMMAND_META[19],
            UiCommand::OpenShellTab => &COMMAND_META[20],
            UiCommand::OpenGitTab => &COMMAND_META[21],
            UiCommand::StopAgent => &COMMAND_META[22],
            UiCommand::RestartAgent => &COMMAND_META[23],
            UiCommand::DeleteWorkspace => &COMMAND_META[24],
            UiCommand::MergeWorkspace => &COMMAND_META[25],
            UiCommand::UpdateFromBase => &COMMAND_META[26],
            UiCommand::RefreshWorkspaces => &COMMAND_META[27],
            UiCommand::OpenProjects => &COMMAND_META[28],
            UiCommand::ReorderProjects => &COMMAND_META[29],
            UiCommand::DeleteProject => &COMMAND_META[30],
            UiCommand::OpenSettings => &COMMAND_META[31],
            UiCommand::ToggleMouseCapture => &COMMAND_META[32],
            UiCommand::ToggleUnsafe => &COMMAND_META[33],
            UiCommand::CleanupSessions => &COMMAND_META[34],
            UiCommand::OpenHelp => &COMMAND_META[35],
            UiCommand::OpenCommandPalette => &COMMAND_META[36],
            UiCommand::Quit => &COMMAND_META[37],
        }
    }
}
