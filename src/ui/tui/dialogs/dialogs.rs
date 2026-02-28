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

pub(super) fn unsafe_state_label(skip_permissions: bool) -> &'static str {
    if skip_permissions {
        "on, bypass approvals and sandbox"
    } else {
        "off, standard safety checks"
    }
}

pub(super) fn modal_start_agent_config_rows<F>(
    content_width: usize,
    theme: UiTheme,
    start_config: &StartAgentConfigState,
    is_focused: F,
) -> [FtLine; 3]
where
    F: Fn(StartAgentConfigField) -> bool,
{
    [
        modal_labeled_input_row(
            content_width,
            theme,
            "Prompt",
            start_config.prompt.as_str(),
            "Describe initial task for the agent",
            is_focused(StartAgentConfigField::Prompt),
        ),
        modal_labeled_input_row(
            content_width,
            theme,
            "InitCmd",
            start_config.init_command.as_str(),
            "Runs once per workspace start (shared across panes)",
            is_focused(StartAgentConfigField::InitCommand),
        ),
        modal_focus_badged_row(
            content_width,
            theme,
            "Unsafe",
            unsafe_state_label(start_config.skip_permissions),
            is_focused(StartAgentConfigField::Unsafe),
            theme.peach,
            if start_config.skip_permissions {
                theme.red
            } else {
                theme.text
            },
        ),
    ]
}

pub(super) fn modal_wrapped_rows(content_width: usize, text: &str, style: Style) -> Vec<FtLine> {
    ftui::text::wrap_text(text, content_width, ftui::text::WrapMode::Word)
        .into_iter()
        .map(|line| FtLine::from_spans(vec![FtSpan::styled(line, style)]))
        .collect()
}

pub(super) fn modal_wrapped_hint_rows(
    content_width: usize,
    theme: UiTheme,
    text: &str,
) -> Vec<FtLine> {
    modal_wrapped_rows(
        content_width,
        text,
        Style::new().fg(theme.overlay0).bg(theme.base),
    )
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ModalDialogSpec<'a> {
    pub(super) dialog_width: u16,
    pub(super) dialog_height: u16,
    pub(super) title: &'a str,
    pub(super) theme: UiTheme,
    pub(super) border_color: PackedRgba,
    pub(super) hit_id: u32,
}

pub(super) fn render_modal_dialog(
    frame: &mut Frame,
    area: Rect,
    body: FtText,
    spec: ModalDialogSpec<'_>,
) {
    let content = OverlayModalContent {
        title: spec.title,
        body,
        theme: spec.theme,
        border_color: spec.border_color,
    };

    Modal::new(content)
        .size(
            ModalSizeConstraints::new()
                .min_width(spec.dialog_width)
                .max_width(spec.dialog_width)
                .min_height(spec.dialog_height)
                .max_height(spec.dialog_height),
        )
        .backdrop(BackdropConfig::new(spec.theme.crust, 0.55))
        .hit_id(HitId::new(spec.hit_id))
        .render(area, frame);
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

macro_rules! active_dialog_accessors {
    ($get:ident, $get_mut:ident, $set:ident, $variant:ident, $ty:ty) => {
        pub(super) fn $get(&self) -> Option<&$ty> {
            match self.active_dialog.as_ref() {
                Some(ActiveDialog::$variant(dialog)) => Some(dialog),
                _ => None,
            }
        }

        pub(super) fn $get_mut(&mut self) -> Option<&mut $ty> {
            match self.active_dialog.as_mut() {
                Some(ActiveDialog::$variant(dialog)) => Some(dialog),
                _ => None,
            }
        }

        pub(super) fn $set(&mut self, dialog: $ty) {
            self.active_dialog = Some(ActiveDialog::$variant(dialog));
        }
    };
}

macro_rules! active_dialog_take_accessor {
    ($take:ident, $variant:ident, $ty:ty) => {
        pub(super) fn $take(&mut self) -> Option<$ty> {
            let active_dialog = self.active_dialog.take()?;
            match active_dialog {
                ActiveDialog::$variant(dialog) => Some(dialog),
                other => {
                    self.active_dialog = Some(other);
                    None
                }
            }
        }
    };
}

impl GroveApp {
    pub(super) fn close_active_dialog(&mut self) {
        self.active_dialog = None;
    }

    pub(super) fn active_dialog_kind(&self) -> Option<&'static str> {
        match self.active_dialog.as_ref() {
            Some(ActiveDialog::Launch(_)) => Some("launch"),
            Some(ActiveDialog::Stop(_)) => Some("stop"),
            Some(ActiveDialog::Confirm(_)) => Some("confirm"),
            Some(ActiveDialog::Delete(_)) => Some("delete"),
            Some(ActiveDialog::Merge(_)) => Some("merge"),
            Some(ActiveDialog::UpdateFromBase(_)) => Some("update_from_base"),
            Some(ActiveDialog::Create(_)) => Some("create"),
            Some(ActiveDialog::Edit(_)) => Some("edit"),
            Some(ActiveDialog::Project(_)) => Some("project"),
            Some(ActiveDialog::Settings(_)) => Some("settings"),
            None => None,
        }
    }

    active_dialog_accessors!(
        launch_dialog,
        launch_dialog_mut,
        set_launch_dialog,
        Launch,
        LaunchDialogState
    );
    active_dialog_take_accessor!(take_launch_dialog, Launch, LaunchDialogState);
    active_dialog_accessors!(
        stop_dialog,
        stop_dialog_mut,
        set_stop_dialog,
        Stop,
        StopDialogState
    );
    active_dialog_take_accessor!(take_stop_dialog, Stop, StopDialogState);
    active_dialog_accessors!(
        confirm_dialog,
        confirm_dialog_mut,
        set_confirm_dialog,
        Confirm,
        ConfirmDialogState
    );
    active_dialog_take_accessor!(take_confirm_dialog, Confirm, ConfirmDialogState);
    active_dialog_accessors!(
        delete_dialog,
        delete_dialog_mut,
        set_delete_dialog,
        Delete,
        DeleteDialogState
    );
    active_dialog_take_accessor!(take_delete_dialog, Delete, DeleteDialogState);
    active_dialog_accessors!(
        merge_dialog,
        merge_dialog_mut,
        set_merge_dialog,
        Merge,
        MergeDialogState
    );
    active_dialog_take_accessor!(take_merge_dialog, Merge, MergeDialogState);
    active_dialog_accessors!(
        update_from_base_dialog,
        update_from_base_dialog_mut,
        set_update_from_base_dialog,
        UpdateFromBase,
        UpdateFromBaseDialogState
    );
    active_dialog_take_accessor!(
        take_update_from_base_dialog,
        UpdateFromBase,
        UpdateFromBaseDialogState
    );
    active_dialog_accessors!(
        create_dialog,
        create_dialog_mut,
        set_create_dialog,
        Create,
        CreateDialogState
    );
    active_dialog_accessors!(
        edit_dialog,
        edit_dialog_mut,
        set_edit_dialog,
        Edit,
        EditDialogState
    );
    active_dialog_accessors!(
        project_dialog,
        project_dialog_mut,
        set_project_dialog,
        Project,
        ProjectDialogState
    );
    active_dialog_accessors!(
        settings_dialog,
        settings_dialog_mut,
        set_settings_dialog,
        Settings,
        SettingsDialogState
    );

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
