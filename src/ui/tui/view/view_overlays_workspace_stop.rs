use super::*;

impl GroveApp {
    pub(super) fn render_stop_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.stop_dialog() else {
            return;
        };
        if area.width < 28 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 14u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let stop_focused = focused(StopDialogField::StopButton);
        let cancel_focused = focused(StopDialogField::CancelButton);
        let path = dialog.workspace.path.display().to_string();

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Session termination", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace.name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.workspace.branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Session",
                dialog.session_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                theme.blue,
                theme.overlay0,
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  [Risk] Agent process will be interrupted immediately",
                    content_width,
                ),
                Style::new().fg(theme.peach).bold(),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Kill Session",
                "Cancel",
                stop_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, h/l switch buttons, Enter or x kill, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Kill Agent Session?",
                theme,
                border_color: theme.red,
                hit_id: HIT_ID_STOP_DIALOG,
            },
        );
    }
}
