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
            "PreLaunchCmd",
            start_config.pre_launch_command.as_str(),
            "Runs before each agent start, e.g. direnv allow",
            is_focused(StartAgentConfigField::PreLaunchCommand),
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
