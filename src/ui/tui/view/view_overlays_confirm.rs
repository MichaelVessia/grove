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
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let confirm_focused = dialog.focused_field == ConfirmDialogField::ConfirmButton;
        let cancel_focused = dialog.focused_field == ConfirmDialogField::CancelButton;

        let (title, message, detail, border_color) = match &dialog.action {
            ConfirmDialogAction::RestartAgent { workspace_name, .. } => (
                "Restart Agent?",
                format!("Restart agent for workspace '{workspace_name}'?"),
                "Current agent process will be interrupted immediately".to_string(),
                theme.yellow,
            ),
            ConfirmDialogAction::QuitApp => (
                "Are you sure?",
                "Quit Grove now?".to_string(),
                "Agent sessions persist in tmux, you can resume after reopen".to_string(),
                theme.red,
            ),
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(message.as_str(), content_width),
                Style::new().fg(theme.text).bold(),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(detail.as_str(), content_width),
                Style::new().fg(theme.overlay0),
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
