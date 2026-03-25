use super::*;
use ftui::widgets::help::{HelpCategory, HelpEntry, HelpMode, KeyFormat, KeybindingHints};
use ftui::widgets::{HelpContent, HelpId, HelpRegistry, Keybinding};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HelpSection {
    Global,
    Workspace,
    List,
    Preview,
    Palette,
    Interactive,
    Modals,
}

impl HelpSection {
    fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Workspace => "Workspace",
            Self::List => "List",
            Self::Preview => "Preview",
            Self::Palette => "Palette",
            Self::Interactive => "Interactive",
            Self::Modals => "Modals",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HelpCatalogEntry {
    pub(super) section: HelpSection,
    pub(super) key: String,
    pub(super) action: String,
}

impl HelpCatalogEntry {
    fn new(section: HelpSection, key: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            section,
            key: key.into(),
            action: action.into(),
        }
    }
}

fn format_help_hint(hint: &HelpHintSpec) -> String {
    format!("{} {}", hint.key, hint.action)
}

impl GroveApp {
    fn command_help_labels(&self, context: HelpHintContext) -> Vec<String> {
        UiCommand::help_hints_for(context)
            .iter()
            .filter_map(|command| command.help_hint(context))
            .map(format_help_hint)
            .collect()
    }

    fn join_help_labels(labels: &[String], indexes: &[usize]) -> String {
        indexes
            .iter()
            .filter_map(|index| labels.get(*index))
            .map(String::as_str)
            .collect::<Vec<&str>>()
            .join(", ")
    }

    pub(super) fn help_catalog_entries(&self) -> Vec<HelpCatalogEntry> {
        let global = self.command_help_labels(HelpHintContext::Global);
        let workspace = self.command_help_labels(HelpHintContext::Workspace);
        let list = self.command_help_labels(HelpHintContext::List);
        let preview_agent = self.command_help_labels(HelpHintContext::PreviewAgent);
        let preview_shell = self.command_help_labels(HelpHintContext::PreviewShell);
        let preview_git = self.command_help_labels(HelpHintContext::PreviewGit);
        let preview_diff = self.command_help_labels(HelpHintContext::PreviewDiff);

        let mut entries = vec![
            HelpCatalogEntry::new(
                HelpSection::Global,
                "Core",
                Self::join_help_labels(&global, &[0, 1, 12]),
            ),
            HelpCatalogEntry::new(
                HelpSection::Global,
                "Focus",
                Self::join_help_labels(&global, &[2, 3, 10, 11]),
            ),
            HelpCatalogEntry::new(
                HelpSection::Global,
                "Layout",
                Self::join_help_labels(&global, &[5, 6, 7]),
            ),
            HelpCatalogEntry::new(
                HelpSection::Global,
                "Workspace nav",
                Self::join_help_labels(&global, &[4, 8, 9, 13]),
            ),
            HelpCatalogEntry::new(
                HelpSection::Global,
                "System",
                Self::join_help_labels(&global, &[14]),
            ),
            HelpCatalogEntry::new(
                HelpSection::Workspace,
                "Task/worktree",
                workspace.join(", "),
            ),
            HelpCatalogEntry::new(HelpSection::Palette, "Search", "[Palette] Type search"),
            HelpCatalogEntry::new(
                HelpSection::Palette,
                "Navigate",
                "Up/Down or C-n/C-p move selection",
            ),
            HelpCatalogEntry::new(HelpSection::Palette, "Run/Close", "Enter run, Esc close"),
            HelpCatalogEntry::new(
                HelpSection::List,
                "Move",
                format!(
                    "{}, i focus needs you inbox, a acknowledge attention item",
                    list.join(", ")
                ),
            ),
            HelpCatalogEntry::new(HelpSection::Preview, "Agent tab", preview_agent.join(", ")),
            HelpCatalogEntry::new(HelpSection::Preview, "Shell tab", preview_shell.join(", ")),
            HelpCatalogEntry::new(HelpSection::Preview, "Git tab", preview_git.join(", ")),
            HelpCatalogEntry::new(HelpSection::Preview, "Diff tab", preview_diff.join(", ")),
        ];
        entries.extend(self.synthetic_help_catalog_entries());
        entries
    }

    fn synthetic_help_catalog_entries(&self) -> Vec<HelpCatalogEntry> {
        vec![
            HelpCatalogEntry::new(
                HelpSection::Interactive,
                "Input",
                "typing sends input to attached session, includes Shift+Tab and Shift+Enter",
            ),
            HelpCatalogEntry::new(
                HelpSection::Interactive,
                "Reserved",
                "Ctrl+K palette, Ctrl+\\ exit, Alt+J/K browse, Alt+[/] tabs, {/} reorder tabs, Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback), Alt+C copy, Alt+V paste",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Create",
                "Tab/S-Tab/C-n/C-p fields, click mode tabs or Alt+[/Alt+], Enter browse projects, picker supports filter + Up/Down + Space toggle, base branch comes from Project Defaults or git, Enter/Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Edit",
                "Tab/S-Tab/C-n/C-p fields, type/backspace base branch (or branch on main), Enter/Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Rename tab",
                "Tab/S-Tab/C-n/C-p fields, type/backspace title, Enter rename, Esc cancel",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Start",
                "Tab/S-Tab or C-n/C-p fields, Space toggle unsafe, h/l buttons, Enter/Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Delete",
                "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/D delete task, Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Merge",
                "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/m merge worktree, Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Update",
                "Tab/S-Tab or C-n/C-p fields, h/l buttons, Enter/u update worktree, Esc",
            ),
            HelpCatalogEntry::new(
                HelpSection::Modals,
                "Projects",
                "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Enter/Esc",
            ),
        ]
    }

    pub(super) fn build_help_registry(&self) -> HelpRegistry {
        let mut registry = HelpRegistry::new();

        for (index, entry) in self.help_catalog_entries().into_iter().enumerate() {
            let Some(id) = u64::try_from(index + 1).ok().map(HelpId) else {
                continue;
            };
            registry.register(
                id,
                HelpContent {
                    short: format!("{} {}", entry.key, entry.action),
                    long: Some(entry.section.label().to_string()),
                    keybindings: vec![Keybinding::new(entry.key, entry.action)],
                    see_also: Vec::new(),
                },
            );
        }

        registry
    }

    pub(super) fn build_keybind_help_hints(&self) -> KeybindingHints {
        let theme = self.active_ui_theme();
        let key_style = Style::new()
            .fg(packed(theme.secondary))
            .bg(packed(theme.background))
            .bold();
        let desc_style = Style::new()
            .fg(packed(theme.text))
            .bg(packed(theme.background));
        let category_style = Style::new()
            .fg(packed(theme.primary))
            .bg(packed(theme.background))
            .bold();
        let registry = self.build_help_registry();
        let mut hints = KeybindingHints::new()
            .with_mode(HelpMode::Full)
            .with_show_categories(true)
            .with_show_context(false)
            .with_key_format(KeyFormat::Bracketed)
            .with_key_style(key_style)
            .with_desc_style(desc_style)
            .with_category_style(category_style);

        let mut ids = registry.ids().collect::<Vec<HelpId>>();
        ids.sort_by_key(|id| id.0);

        for id in ids {
            let Some(content) = registry.peek(id) else {
                continue;
            };
            let Some(keybinding) = content.keybindings.first() else {
                continue;
            };
            let category = content
                .long
                .clone()
                .map(HelpCategory::Custom)
                .unwrap_or(HelpCategory::General);
            hints = hints.with_global_entry(
                HelpEntry::new(keybinding.key.clone(), keybinding.action.clone())
                    .with_category(category),
            );
        }

        hints
    }
}
