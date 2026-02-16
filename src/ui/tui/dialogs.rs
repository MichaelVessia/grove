use super::*;

pub(super) fn modal_labeled_input_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
) -> FtLine {
    let row_bg = if focused { theme.surface1 } else { theme.base };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let value_raw = if value.is_empty() { placeholder } else { value };
    let rendered = truncate_to_display_width(value_raw, content_width.saturating_sub(prefix_width));
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    theme.yellow
                } else {
                    theme.overlay0
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(badge, Style::new().fg(theme.blue).bg(row_bg).bold()),
        FtSpan::styled(
            rendered,
            Style::new()
                .fg(if value.is_empty() {
                    theme.overlay0
                } else {
                    theme.text
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

pub(super) fn modal_static_badged_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine {
    let badge = format!("[{label}] ");
    let prefix = format!("  {badge}");
    let available = content_width.saturating_sub(text_display_width(prefix.as_str()));
    let rendered = truncate_to_display_width(value, available);
    let used =
        text_display_width(prefix.as_str()).saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled("  ", Style::new().bg(theme.base)),
        FtSpan::styled(badge, Style::new().fg(badge_fg).bg(theme.base).bold()),
        FtSpan::styled(rendered, Style::new().fg(value_fg).bg(theme.base)),
        FtSpan::styled(pad, Style::new().bg(theme.base)),
    ])
}

pub(super) fn modal_focus_badged_row(
    content_width: usize,
    theme: UiTheme,
    label: &str,
    value: &str,
    focused: bool,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine {
    let row_bg = if focused { theme.surface1 } else { theme.base };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let rendered = truncate_to_display_width(value, content_width.saturating_sub(prefix_width));
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    theme.yellow
                } else {
                    theme.overlay0
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(badge, Style::new().fg(badge_fg).bg(row_bg).bold()),
        FtSpan::styled(rendered, Style::new().fg(value_fg).bg(row_bg).bold()),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

pub(super) fn modal_actions_row(
    content_width: usize,
    theme: UiTheme,
    primary_label: &str,
    secondary_label: &str,
    primary_focused: bool,
    secondary_focused: bool,
) -> FtLine {
    let actions_bg = if primary_focused || secondary_focused {
        theme.surface1
    } else {
        theme.base
    };
    let actions_prefix = if primary_focused || secondary_focused {
        "> "
    } else {
        "  "
    };
    let primary = if primary_focused {
        format!("[{primary_label}]")
    } else {
        format!(" {primary_label} ")
    };
    let secondary = if secondary_focused {
        format!("[{secondary_label}]")
    } else {
        format!(" {secondary_label} ")
    };
    let row = pad_or_truncate_to_display_width(
        format!("{actions_prefix}{primary}   {secondary}").as_str(),
        content_width,
    );

    FtLine::from_spans(vec![FtSpan::styled(
        row,
        Style::new().fg(theme.text).bg(actions_bg).bold(),
    )])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LaunchDialogState {
    pub(super) prompt: String,
    pub(super) pre_launch_command: String,
    pub(super) skip_permissions: bool,
    pub(super) focused_field: LaunchDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) branch: String,
    pub(super) path: PathBuf,
    pub(super) is_missing: bool,
    pub(super) delete_local_branch: bool,
    pub(super) focused_field: DeleteDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MergeDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) cleanup_workspace: bool,
    pub(super) cleanup_local_branch: bool,
    pub(super) focused_field: MergeDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UpdateFromBaseDialogState {
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) focused_field: UpdateFromBaseDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeleteDialogField {
    DeleteLocalBranch,
    DeleteButton,
    CancelButton,
}

impl DeleteDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::DeleteButton,
            Self::DeleteButton => Self::CancelButton,
            Self::CancelButton => Self::DeleteLocalBranch,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::DeleteLocalBranch => Self::CancelButton,
            Self::DeleteButton => Self::DeleteLocalBranch,
            Self::CancelButton => Self::DeleteButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeDialogField {
    CleanupWorkspace,
    CleanupLocalBranch,
    MergeButton,
    CancelButton,
}

impl MergeDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::CleanupWorkspace => Self::CleanupLocalBranch,
            Self::CleanupLocalBranch => Self::MergeButton,
            Self::MergeButton => Self::CancelButton,
            Self::CancelButton => Self::CleanupWorkspace,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::CleanupWorkspace => Self::CancelButton,
            Self::CleanupLocalBranch => Self::CleanupWorkspace,
            Self::MergeButton => Self::CleanupLocalBranch,
            Self::CancelButton => Self::MergeButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateFromBaseDialogField {
    UpdateButton,
    CancelButton,
}

impl UpdateFromBaseDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::UpdateButton => Self::CancelButton,
            Self::CancelButton => Self::UpdateButton,
        }
    }

    pub(super) fn previous(self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchDialogField {
    Prompt,
    PreLaunchCommand,
    Unsafe,
    StartButton,
    CancelButton,
}

impl LaunchDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Prompt => Self::PreLaunchCommand,
            Self::PreLaunchCommand => Self::Unsafe,
            Self::Unsafe => Self::StartButton,
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::Prompt,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Prompt => Self::CancelButton,
            Self::PreLaunchCommand => Self::Prompt,
            Self::Unsafe => Self::PreLaunchCommand,
            Self::StartButton => Self::Unsafe,
            Self::CancelButton => Self::StartButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::PreLaunchCommand => "pre_launch_command",
            Self::Unsafe => "unsafe",
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateDialogState {
    pub(super) workspace_name: String,
    pub(super) project_index: usize,
    pub(super) agent: AgentType,
    pub(super) base_branch: String,
    pub(super) focused_field: CreateDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) branch: String,
    pub(super) agent: AgentType,
    pub(super) was_running: bool,
    pub(super) focused_field: EditDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectDialogState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) selected_filtered_index: usize,
    pub(super) add_dialog: Option<ProjectAddDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectAddDialogState {
    pub(super) name: String,
    pub(super) path: String,
    pub(super) focused_field: ProjectAddDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectAddDialogField {
    Name,
    Path,
    AddButton,
    CancelButton,
}

impl ProjectAddDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Name => Self::Path,
            Self::Path => Self::AddButton,
            Self::AddButton => Self::CancelButton,
            Self::CancelButton => Self::Name,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Name => Self::CancelButton,
            Self::Path => Self::Name,
            Self::AddButton => Self::Path,
            Self::CancelButton => Self::AddButton,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsDialogState {
    pub(super) multiplexer: MultiplexerKind,
    pub(super) focused_field: SettingsDialogField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    Multiplexer,
    SaveButton,
    CancelButton,
}

impl SettingsDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Multiplexer => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Multiplexer,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Multiplexer => Self::CancelButton,
            Self::SaveButton => Self::Multiplexer,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogField {
    WorkspaceName,
    Project,
    BaseBranch,
    Agent,
    CreateButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditDialogField {
    Agent,
    SaveButton,
    CancelButton,
}

impl EditDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Agent => Self::SaveButton,
            Self::SaveButton => Self::CancelButton,
            Self::CancelButton => Self::Agent,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Agent => Self::CancelButton,
            Self::SaveButton => Self::Agent,
            Self::CancelButton => Self::SaveButton,
        }
    }
}

impl CreateDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::WorkspaceName => Self::Project,
            Self::Project => Self::BaseBranch,
            Self::BaseBranch => Self::Agent,
            Self::Agent => Self::CreateButton,
            Self::CreateButton => Self::CancelButton,
            Self::CancelButton => Self::WorkspaceName,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::WorkspaceName => Self::CancelButton,
            Self::Project => Self::WorkspaceName,
            Self::BaseBranch => Self::Project,
            Self::Agent => Self::BaseBranch,
            Self::CreateButton => Self::Agent,
            Self::CancelButton => Self::CreateButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::Project => "project",
            Self::BaseBranch => "base_branch",
            Self::Agent => "agent",
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct OverlayModalContent<'a> {
    pub(super) title: &'a str,
    pub(super) body: FtText,
    pub(super) theme: UiTheme,
    pub(super) border_color: PackedRgba,
}

impl Widget for OverlayModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new().bg(self.theme.base).fg(self.theme.text);

        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title(self.title)
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(self.border_color).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        Paragraph::new(self.body.clone())
            .style(content_style)
            .render(inner, frame);
    }
}

impl GroveApp {
    pub(super) fn allows_text_input_modifiers(modifiers: Modifiers) -> bool {
        modifiers.is_empty() || modifiers == Modifiers::SHIFT
    }

    pub(super) fn handle_keybind_help_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::Char('?') => {
                self.keybind_help_open = false;
            }
            _ => {}
        }
    }
}
