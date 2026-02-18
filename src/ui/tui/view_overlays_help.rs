use super::*;

impl GroveApp {
    const KEYBIND_HELP_MIN_WIDTH: u16 = 56;
    const KEYBIND_HELP_MIN_HEIGHT: u16 = 16;
    const KEYBIND_HELP_HORIZONTAL_MARGIN: u16 = 1;
    const KEYBIND_HELP_VERTICAL_MARGIN: u16 = 0;

    fn keybind_help_hints(context: HelpHintContext) -> Vec<&'static str> {
        UiCommand::help_hints_for(context)
            .iter()
            .filter_map(|command| command.help_hint_label(context))
            .collect()
    }

    fn keybind_help_join_hint_indexes(hints: &[&str], indexes: &[usize]) -> String {
        indexes
            .iter()
            .filter_map(|index| hints.get(*index).copied())
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn keybind_help_section_title(content_width: usize, theme: UiTheme, title: &str) -> FtLine {
        let title_text = format!("[{title}]");
        let title_width = text_display_width(title_text.as_str());
        let remaining = content_width.saturating_sub(title_width);
        let fill = if remaining > 0 {
            format!(" {}", "-".repeat(remaining.saturating_sub(1)))
        } else {
            String::new()
        };
        FtLine::from_spans(vec![
            FtSpan::styled(title_text, Style::new().fg(theme.blue).bold()),
            FtSpan::styled(fill, Style::new().fg(theme.overlay0)),
        ])
    }

    fn keybind_help_push_labeled_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
        value_fg: PackedRgba,
    ) {
        let label_cell =
            pad_or_truncate_to_display_width(format!("  {label}:").as_str(), label_width);
        let continuation_label = " ".repeat(label_width);
        if content_width <= label_width.saturating_add(1) {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(label_cell.as_str(), content_width),
                Style::new().fg(theme.lavender).bold(),
            )]));
            return;
        }
        let text_width = content_width.saturating_sub(label_width.saturating_add(1));
        let wrapped = ftui::text::wrap_text(text, text_width, ftui::text::WrapMode::Word);
        for (line_index, segment) in wrapped.iter().enumerate() {
            let row_label = if line_index == 0 {
                label_cell.as_str()
            } else {
                continuation_label.as_str()
            };
            lines.push(FtLine::from_spans(vec![
                FtSpan::styled(
                    row_label,
                    Style::new()
                        .fg(if line_index == 0 {
                            theme.lavender
                        } else {
                            theme.overlay0
                        })
                        .bold(),
                ),
                FtSpan::styled(" ", Style::new().fg(theme.overlay0)),
                FtSpan::styled(
                    pad_or_truncate_to_display_width(segment.as_str(), text_width),
                    Style::new().fg(value_fg),
                ),
            ]));
        }
    }

    fn keybind_help_push_section_gap(lines: &mut Vec<FtLine>, section_gap: usize) {
        if section_gap > 0 {
            lines.extend(
                std::iter::repeat_with(|| FtLine::raw(""))
                    .take(section_gap)
                    .collect::<Vec<FtLine>>(),
            );
        }
    }

    fn keybind_help_push_modal_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.subtext0,
        );
    }

    fn keybind_help_push_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.text,
        );
    }

    fn keybind_help_push_palette_row(
        lines: &mut Vec<FtLine>,
        content_width: usize,
        label_width: usize,
        label: &str,
        text: &str,
        theme: UiTheme,
    ) {
        Self::keybind_help_push_labeled_row(
            lines,
            content_width,
            label_width,
            label,
            text,
            theme,
            theme.subtext0,
        );
    }

    pub(super) fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }

    pub(super) fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.command_palette.is_visible() {
            return;
        }

        self.command_palette.render(area, frame);
    }

    pub(super) fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.keybind_help_open {
            return;
        }
        if area.width < Self::KEYBIND_HELP_MIN_WIDTH.saturating_add(2)
            || area.height < Self::KEYBIND_HELP_MIN_HEIGHT.saturating_add(2)
        {
            return;
        }

        let dialog_width = area
            .width
            .saturating_sub(Self::KEYBIND_HELP_HORIZONTAL_MARGIN.saturating_mul(2))
            .max(Self::KEYBIND_HELP_MIN_WIDTH);
        let dialog_height = area
            .height
            .saturating_sub(Self::KEYBIND_HELP_VERTICAL_MARGIN.saturating_mul(2))
            .max(Self::KEYBIND_HELP_MIN_HEIGHT);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let section_gap = if dialog_height >= 42 { 1 } else { 0 };
        let label_width = if content_width >= 132 {
            18
        } else if content_width >= 96 {
            15
        } else {
            12
        };
        let global_hints = Self::keybind_help_hints(HelpHintContext::Global);
        let workspace_hints = Self::keybind_help_hints(HelpHintContext::Workspace);
        let list_hints = Self::keybind_help_hints(HelpHintContext::List);
        let preview_agent_hints = Self::keybind_help_hints(HelpHintContext::PreviewAgent);
        let preview_shell_hints = Self::keybind_help_hints(HelpHintContext::PreviewShell);
        let preview_git_hints = Self::keybind_help_hints(HelpHintContext::PreviewGit);
        let global_core =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[0, 1, 11]);
        let global_focus =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[2, 6, 10]);
        let global_layout =
            Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[3, 4, 5]);
        let global_tabs = Self::keybind_help_join_hint_indexes(global_hints.as_slice(), &[7, 8, 9]);
        let workspace_actions = workspace_hints.join(", ");

        let mut lines = vec![Self::keybind_help_section_title(
            content_width,
            theme,
            "Global",
        )];
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Core",
            global_core.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Focus",
            global_focus.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Layout",
            global_layout.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Workspace nav",
            global_tabs.as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Workspace",
            workspace_actions.as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Palette",
        ));
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Search",
            "[Palette] Type search",
            theme,
        );
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Navigate",
            "Up/Down or C-n/C-p move selection",
            theme,
        );
        Self::keybind_help_push_palette_row(
            &mut lines,
            content_width,
            label_width,
            "Run/Close",
            "Enter run, Esc close",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "List",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Move",
            list_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Preview",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Agent tab",
            preview_agent_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Shell tab",
            preview_shell_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Git tab",
            preview_git_hints.join(", ").as_str(),
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Interactive",
        ));
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Input",
            "typing sends input to attached session, includes Shift+Tab and Shift+Enter",
            theme,
        );
        Self::keybind_help_push_row(
            &mut lines,
            content_width,
            label_width,
            "Reserved",
            "Ctrl+K palette, Esc Esc/Ctrl+\\ exit, Alt+J/K browse, Alt+[/] tabs, Alt+Left/Right or Alt+H/L resize (Alt+B/F fallback), Alt+C copy, Alt+V paste",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);

        lines.push(Self::keybind_help_section_title(
            content_width,
            theme,
            "Modals",
        ));
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Create",
            "Tab/S-Tab or C-n/C-p fields, j/k adjust controls, ';' separates setup commands, Space toggles auto-run, h/l buttons, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Edit",
            "Tab/S-Tab or C-n/C-p fields, type/backspace base, h/l or Space toggle agent, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Start",
            "Tab/S-Tab or C-n/C-p fields, Space toggle unsafe, h/l buttons, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Delete",
            "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/D confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Merge",
            "Tab/S-Tab or C-n/C-p fields, j/k move, Space toggle, Enter/m confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Update",
            "Tab/S-Tab or C-n/C-p fields, h/l buttons, Enter/u confirm, Esc",
            theme,
        );
        Self::keybind_help_push_modal_row(
            &mut lines,
            content_width,
            label_width,
            "Projects",
            "Type filter, Up/Down or Tab/S-Tab/C-n/C-p move, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Enter/Esc",
            theme,
        );
        Self::keybind_help_push_section_gap(&mut lines, section_gap);
        lines.extend(modal_wrapped_rows(
            content_width,
            "Close help: Esc, Enter, or ?",
            Style::new().fg(theme.lavender).bold(),
        ));

        let content = OverlayModalContent {
            title: "Keybind Help",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.blue,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_KEYBIND_HELP_DIALOG))
            .render(area, frame);
    }
}
