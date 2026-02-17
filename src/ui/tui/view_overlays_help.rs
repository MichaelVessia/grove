use super::*;

impl GroveApp {
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
        if area.width < 56 || area.height < 18 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(108);
        let dialog_height = area.height.saturating_sub(6).clamp(18, 26);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let global_hints = self.keybind_help_line(HelpHintContext::Global);
        let workspace_hints = self.keybind_help_line(HelpHintContext::Workspace);
        let list_hints = self.keybind_help_line(HelpHintContext::List);
        let preview_agent_hints = self.keybind_help_line(HelpHintContext::PreviewAgent);
        let preview_git_hints = self.keybind_help_line(HelpHintContext::PreviewGit);

        let lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Global]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  {global_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  {workspace_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[List]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(format!("  {list_hints}").as_str(), content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Preview]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  Agent tab: {preview_agent_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  Git tab: {preview_git_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Interactive]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  type sends input to attached session",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Esc Esc or Ctrl+\\ exit, Alt+J/K browse workspaces, Alt+C copy, Alt+V paste",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Create: Tab/S-Tab fields, j/k or C-n/C-p move, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Edit:   Tab/S-Tab fields, type/backspace base, h/l or Space toggle agent, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Start:  Tab/S-Tab fields, Space toggle unsafe, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Delete: Tab/S-Tab fields, j/k move, Space toggle, Enter/D confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Merge:  Tab/S-Tab fields, j/k move, Space toggle, Enter/m confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Update: Tab/S-Tab fields, h/l buttons, Enter/u confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Projects: Type filter, Up/Down or Tab/S-Tab move, Ctrl+A add, Ctrl+X/Del remove, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Close help: Esc, Enter, or ?", content_width),
                Style::new().fg(theme.lavender).bold(),
            )]),
        ];

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
