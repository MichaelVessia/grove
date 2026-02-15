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
    pub(super) fn handle_keybind_help_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::Char('?') => {
                self.keybind_help_open = false;
            }
            _ => {}
        }
    }

    pub(super) fn handle_project_add_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        match key_event.code {
            KeyCode::Escape => {
                project_dialog.add_dialog = None;
            }
            KeyCode::Tab => {
                add_dialog.focused_field = add_dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                add_dialog.focused_field = add_dialog.focused_field.previous();
            }
            KeyCode::Enter => match add_dialog.focused_field {
                ProjectAddDialogField::AddButton => self.add_project_from_dialog(),
                ProjectAddDialogField::CancelButton => project_dialog.add_dialog = None,
                ProjectAddDialogField::Name | ProjectAddDialogField::Path => {
                    add_dialog.focused_field = add_dialog.focused_field.next();
                }
            },
            KeyCode::Backspace => match add_dialog.focused_field {
                ProjectAddDialogField::Name => {
                    add_dialog.name.pop();
                }
                ProjectAddDialogField::Path => {
                    add_dialog.path.pop();
                }
                ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                match add_dialog.focused_field {
                    ProjectAddDialogField::Name => add_dialog.name.push(character),
                    ProjectAddDialogField::Path => add_dialog.path.push(character),
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_project_dialog_key(&mut self, key_event: KeyEvent) {
        if self
            .project_dialog
            .as_ref()
            .and_then(|dialog| dialog.add_dialog.as_ref())
            .is_some()
        {
            self.handle_project_add_dialog_key(key_event);
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && !dialog.filter.is_empty()
                {
                    dialog.filter.clear();
                    self.refresh_project_dialog_filtered();
                    return;
                }
                self.project_dialog = None;
            }
            KeyCode::Enter => {
                if let Some(project_index) = self.selected_project_dialog_project_index() {
                    self.focus_project_by_index(project_index);
                    self.project_dialog = None;
                }
            }
            KeyCode::Up => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index > 0
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index =
                            dialog.selected_filtered_index.saturating_add(1) % len;
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index = if dialog.selected_filtered_index == 0 {
                            len.saturating_sub(1)
                        } else {
                            dialog.selected_filtered_index.saturating_sub(1)
                        };
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.pop();
                }
                self.refresh_project_dialog_filtered();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'a' || character == 'A') =>
            {
                self.open_project_add_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'n' || character == 'N') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'p' || character == 'P') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.push(character);
                }
                self.refresh_project_dialog_filtered();
            }
            _ => {}
        }
    }

    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.previous();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::Multiplexer => {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
                SettingsDialogField::SaveButton => post_action = PostAction::Save,
                SettingsDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.settings_dialog = None;
            }
        }
    }

    pub(super) fn handle_delete_dialog_key(&mut self, key_event: KeyEvent) {
        if self.delete_in_flight {
            return;
        }
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.delete_dialog = None;
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.delete_dialog = None;
                return;
            }
            KeyCode::Char('D') if no_modifiers => {
                self.confirm_delete_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_delete = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.delete_dialog.as_mut() else {
            return;
        };

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                DeleteDialogField::DeleteLocalBranch => {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                }
                DeleteDialogField::DeleteButton => {
                    confirm_delete = true;
                }
                DeleteDialogField::CancelButton => {
                    cancel_dialog = true;
                }
            },
            KeyCode::Tab => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Up | KeyCode::Char('k') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Down | KeyCode::Char('j') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(' ') if no_modifiers => {
                if dialog.focused_field == DeleteDialogField::DeleteLocalBranch {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                }
            }
            KeyCode::Char(character) if no_modifiers => {
                if (dialog.focused_field == DeleteDialogField::DeleteButton
                    || dialog.focused_field == DeleteDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == DeleteDialogField::DeleteButton {
                            DeleteDialogField::CancelButton
                        } else {
                            DeleteDialogField::DeleteButton
                        };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("delete", "dialog_cancelled");
            self.delete_dialog = None;
            return;
        }
        if confirm_delete {
            self.confirm_delete_dialog();
        }
    }

    pub(super) fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.create_in_flight {
            return;
        }

        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.create_dialog = None;
                self.clear_create_branch_picker();
            }
            KeyCode::Enter => {
                if self.select_create_base_branch_from_dropdown() {
                    if let Some(dialog) = self.create_dialog.as_mut() {
                        dialog.focused_field = dialog.focused_field.next();
                    }
                    self.refresh_create_branch_filtered();
                    return;
                }

                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                let action = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                        CreateDialogField::CancelButton => EnterAction::CancelDialog,
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::Project
                        | CreateDialogField::BaseBranch
                        | CreateDialogField::Agent => EnterAction::AdvanceField,
                    });

                match action {
                    Some(EnterAction::ConfirmCreate) => self.confirm_create_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.create_dialog = None;
                        self.clear_create_branch_picker();
                    }
                    Some(EnterAction::AdvanceField) => {
                        if let Some(dialog) = self.create_dialog.as_mut() {
                            dialog.focused_field = dialog.focused_field.next();
                        }
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Up => {
                if self.create_base_branch_dropdown_visible() && self.create_branch_index > 0 {
                    self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(-1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Down => {
                if self.create_base_branch_dropdown_visible()
                    && self.create_branch_index.saturating_add(1)
                        < self.create_branch_filtered.len()
                {
                    self.create_branch_index = self.create_branch_index.saturating_add(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Char(_) if ctrl_n || ctrl_p => {
                let focused_field = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| dialog.focused_field);
                if focused_field == Some(CreateDialogField::BaseBranch)
                    && !self.create_branch_filtered.is_empty()
                {
                    if ctrl_n
                        && self.create_branch_index.saturating_add(1)
                            < self.create_branch_filtered.len()
                    {
                        self.create_branch_index = self.create_branch_index.saturating_add(1);
                    }
                    if ctrl_p && self.create_branch_index > 0 {
                        self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    }
                } else if focused_field == Some(CreateDialogField::Project) {
                    if ctrl_n {
                        self.shift_create_dialog_project(1);
                    }
                    if ctrl_p {
                        self.shift_create_dialog_project(-1);
                    }
                } else if focused_field == Some(CreateDialogField::Agent)
                    && let Some(dialog) = self.create_dialog.as_mut()
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Backspace => {
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.workspace_name.pop();
                        }
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.pop();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    if character == 'j' {
                        self.shift_create_dialog_project(1);
                        return;
                    }
                    if character == 'k' {
                        self.shift_create_dialog_project(-1);
                        return;
                    }
                }
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    if dialog.focused_field == CreateDialogField::Agent
                        && (character == 'j' || character == 'k' || character == ' ')
                    {
                        Self::toggle_create_dialog_agent(dialog);
                        return;
                    }
                    if (dialog.focused_field == CreateDialogField::CreateButton
                        || dialog.focused_field == CreateDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == CreateDialogField::CreateButton {
                                CreateDialogField::CancelButton
                            } else {
                                CreateDialogField::CreateButton
                            };
                        return;
                    }
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            if character.is_ascii_alphanumeric()
                                || character == '-'
                                || character == '_'
                            {
                                dialog.workspace_name.push(character);
                            }
                        }
                        CreateDialogField::Project => {}
                        CreateDialogField::BaseBranch => {
                            if character == 'j'
                                && self.create_branch_index.saturating_add(1)
                                    < self.create_branch_filtered.len()
                            {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_add(1);
                                return;
                            }
                            if character == 'k' && self.create_branch_index > 0 {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_sub(1);
                                return;
                            }
                            if !character.is_control() {
                                dialog.base_branch.push(character);
                                refresh_base_branch = true;
                            }
                        }
                        CreateDialogField::Agent => {}
                        CreateDialogField::CreateButton | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_edit_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.edit_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                EditDialogField::Agent => Self::toggle_edit_dialog_agent(dialog),
                EditDialogField::SaveButton => post_action = PostAction::Save,
                EditDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_edit_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("edit", "dialog_cancelled");
                self.edit_dialog = None;
            }
        }
    }

    pub(super) fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.start_in_flight {
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("launch", "dialog_cancelled");
                self.launch_dialog = None;
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmStart,
                    CancelDialog,
                }

                let action = self
                    .launch_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        LaunchDialogField::StartButton => EnterAction::ConfirmStart,
                        LaunchDialogField::CancelButton => EnterAction::CancelDialog,
                        LaunchDialogField::Prompt
                        | LaunchDialogField::PreLaunchCommand
                        | LaunchDialogField::Unsafe => EnterAction::ConfirmStart,
                    });

                match action {
                    Some(EnterAction::ConfirmStart) => self.confirm_start_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("launch", "dialog_cancelled");
                        self.launch_dialog = None;
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => {
                            dialog.prompt.pop();
                        }
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.pop();
                        }
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    if (dialog.focused_field == LaunchDialogField::StartButton
                        || dialog.focused_field == LaunchDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == LaunchDialogField::StartButton {
                                LaunchDialogField::CancelButton
                            } else {
                                LaunchDialogField::StartButton
                            };
                        return;
                    }
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => dialog.prompt.push(character),
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.push(character)
                        }
                        LaunchDialogField::Unsafe => {
                            if character == ' ' || character == 'j' || character == 'k' {
                                dialog.skip_permissions = !dialog.skip_permissions;
                            }
                        }
                        LaunchDialogField::StartButton | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn filtered_project_indices(&self, query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..self.projects.len()).collect();
        }

        let query_lower = query.to_ascii_lowercase();
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| {
                project.name.to_ascii_lowercase().contains(&query_lower)
                    || project
                        .path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query_lower)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn refresh_project_dialog_filtered(&mut self) {
        let query = match self.project_dialog.as_ref() {
            Some(dialog) => dialog.filter.clone(),
            None => return,
        };
        let filtered = self.filtered_project_indices(&query);
        let Some(dialog) = self.project_dialog.as_mut() else {
            return;
        };

        dialog.filtered_project_indices = filtered;
        if dialog.filtered_project_indices.is_empty() {
            dialog.selected_filtered_index = 0;
            return;
        }
        if dialog.selected_filtered_index >= dialog.filtered_project_indices.len() {
            dialog.selected_filtered_index =
                dialog.filtered_project_indices.len().saturating_sub(1);
        }
    }

    fn selected_project_dialog_project_index(&self) -> Option<usize> {
        let dialog = self.project_dialog.as_ref()?;
        if dialog.filtered_project_indices.is_empty() {
            return None;
        }
        dialog
            .filtered_project_indices
            .get(dialog.selected_filtered_index)
            .copied()
    }

    fn focus_project_by_index(&mut self, project_index: usize) {
        let Some(project) = self.projects.get(project_index) else {
            return;
        };

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace.is_main
                        && workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
            return;
        }

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
        }
    }

    pub(super) fn open_project_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let selected_project_index = self.selected_project_index();
        let filtered_project_indices: Vec<usize> = (0..self.projects.len()).collect();
        let selected_filtered_index = filtered_project_indices
            .iter()
            .position(|index| *index == selected_project_index)
            .unwrap_or(0);
        self.project_dialog = Some(ProjectDialogState {
            filter: String::new(),
            filtered_project_indices,
            selected_filtered_index,
            add_dialog: None,
        });
    }

    fn open_project_add_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        project_dialog.add_dialog = Some(ProjectAddDialogState {
            name: String::new(),
            path: String::new(),
            focused_field: ProjectAddDialogField::Name,
        });
    }

    fn normalized_project_path(raw: &str) -> PathBuf {
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(raw)
    }

    fn save_projects_config(&self) -> Result<(), String> {
        let config = GroveConfig {
            multiplexer: self.multiplexer,
            projects: self.projects.clone(),
        };
        crate::config::save_to_path(&self.config_path, &config)
    }

    fn add_project_from_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_ref() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };

        let path_input = add_dialog.path.trim();
        if path_input.is_empty() {
            self.show_toast("project path is required", true);
            return;
        }
        let normalized = Self::normalized_project_path(path_input);
        let canonical = match normalized.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                self.show_toast(format!("invalid project path: {error}"), true);
                return;
            }
        };

        let repo_root_output = Command::new("git")
            .current_dir(&canonical)
            .args(["rev-parse", "--show-toplevel"])
            .output();
        let repo_root = match repo_root_output {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8(output.stdout).unwrap_or_default();
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    canonical.clone()
                } else {
                    PathBuf::from(trimmed)
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                self.show_toast(format!("not a git repository: {stderr}"), true);
                return;
            }
            Err(error) => {
                self.show_toast(format!("git check failed: {error}"), true);
                return;
            }
        };
        let repo_root = repo_root.canonicalize().unwrap_or(repo_root);

        if self
            .projects
            .iter()
            .any(|project| project_paths_equal(&project.path, &repo_root))
        {
            self.show_toast("project already exists", true);
            return;
        }

        let project_name = if add_dialog.name.trim().is_empty() {
            project_display_name(&repo_root)
        } else {
            add_dialog.name.trim().to_string()
        };
        self.projects.push(ProjectConfig {
            name: project_name.clone(),
            path: repo_root.clone(),
        });
        if let Err(error) = self.save_projects_config() {
            self.show_toast(format!("project save failed: {error}"), true);
            return;
        }

        if let Some(dialog) = self.project_dialog.as_mut() {
            dialog.add_dialog = None;
        }
        self.refresh_project_dialog_filtered();
        self.refresh_workspaces(None);
        self.show_toast(format!("project '{}' added", project_name), false);
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.settings_dialog = Some(SettingsDialogState {
            multiplexer: self.multiplexer,
            focused_field: SettingsDialogField::Multiplexer,
        });
    }

    fn has_running_workspace_sessions(&self) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|workspace| workspace.status.has_session())
    }

    fn apply_settings_dialog_save(&mut self) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };

        if dialog.multiplexer != self.multiplexer && self.has_running_workspace_sessions() {
            self.show_toast(
                "restart running workspaces before switching multiplexer",
                true,
            );
            return;
        }

        let selected = dialog.multiplexer;
        self.multiplexer = selected;
        self.tmux_input = input_for_multiplexer(selected);
        let config = GroveConfig {
            multiplexer: selected,
            projects: self.projects.clone(),
        };
        if let Err(error) = crate::config::save_to_path(&self.config_path, &config) {
            self.show_toast(format!("settings save failed: {error}"), true);
            return;
        }

        self.settings_dialog = None;
        self.interactive = None;
        self.lazygit_ready_sessions.clear();
        self.lazygit_failed_sessions.clear();
        self.refresh_workspaces(None);
        self.poll_preview();
        self.show_toast(format!("multiplexer set to {}", selected.label()), false);
    }

    pub(super) fn open_start_dialog(&mut self) {
        if self.start_in_flight {
            self.show_toast("agent start already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        if !workspace.supported_agent {
            self.show_toast("unsupported workspace agent marker", true);
            return;
        }
        if workspace.status.is_running() {
            self.show_toast("agent already running", true);
            return;
        }
        if !self.can_start_selected_workspace() {
            self.show_toast("workspace cannot be started", true);
            return;
        }

        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let skip_permissions = self.launch_skip_permissions;
        self.launch_dialog = Some(LaunchDialogState {
            prompt: prompt.clone(),
            pre_launch_command: String::new(),
            skip_permissions,
            focused_field: LaunchDialogField::Prompt,
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                (
                    "prompt_len".to_string(),
                    Value::from(u64::try_from(prompt.len()).unwrap_or(u64::MAX)),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(skip_permissions),
                ),
                ("pre_launch_len".to_string(), Value::from(0_u64)),
            ],
        );
        self.last_tmux_error = None;
    }

    pub(super) fn open_delete_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.delete_in_flight {
            self.show_toast("workspace delete already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        if workspace.is_main {
            self.show_toast("cannot delete base workspace", true);
            return;
        }

        let is_missing = !workspace.path.exists();
        self.delete_dialog = Some(DeleteDialogState {
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            workspace_name: workspace.name.clone(),
            branch: workspace.branch.clone(),
            path: workspace.path.clone(),
            is_missing,
            delete_local_branch: is_missing,
            focused_field: DeleteDialogField::DeleteLocalBranch,
        });
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("is_missing".to_string(), Value::from(is_missing)),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn confirm_delete_dialog(&mut self) {
        if self.delete_in_flight {
            return;
        }

        let Some(dialog) = self.delete_dialog.take() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("branch".to_string(), Value::from(dialog.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(dialog.path.display().to_string()),
                ),
                (
                    "delete_local_branch".to_string(),
                    Value::from(dialog.delete_local_branch),
                ),
                ("is_missing".to_string(), Value::from(dialog.is_missing)),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.path.clone();
        if !self.tmux_input.supports_background_send() {
            let (result, warnings) = Self::run_delete_workspace(dialog, self.multiplexer);
            self.apply_delete_workspace_completion(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            });
            return;
        }

        let multiplexer = self.multiplexer;
        self.delete_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = Self::run_delete_workspace(dialog, multiplexer);
            Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            })
        }));
    }
}
