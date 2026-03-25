use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_confirm_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.confirm_dialog() else {
            return;
        };
        if area.width < 24 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(24).clamp(44, 72);
        let dialog_height = 10u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let confirm_focused = dialog.focused_field == ConfirmDialogField::ConfirmButton;
        let cancel_focused = dialog.focused_field == ConfirmDialogField::CancelButton;
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };

        let (title, message, detail, border_color) = match &dialog.action {
            ConfirmDialogAction::CloseActiveTab { session_name, .. } => (
                "Close Active Tab?",
                "Kill session and close this tab?".to_string(),
                format!("Session '{session_name}' is still live in tmux"),
                packed(theme.warning),
            ),
            ConfirmDialogAction::QuitApp => (
                "Are you sure?",
                "Quit Grove now?".to_string(),
                "Agent sessions persist in tmux, you can resume after reopen".to_string(),
                packed(theme.error),
            ),
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit(message.as_str()),
                Style::new().fg(packed(theme.text)).bold(),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                fit(detail.as_str()),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Yes",
                "No",
                confirm_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Enter select, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title,
                theme,
                border_color,
                hit_id: HIT_ID_CONFIRM_DIALOG,
            },
        );
    }
}
