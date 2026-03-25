use super::view_prelude::*;

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
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let stop_focused = focused(StopDialogField::StopButton);
        let cancel_focused = focused(StopDialogField::CancelButton);
        let path = dialog.workspace.path.display().to_string();
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Session termination"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace.name.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.workspace.branch.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Session",
                dialog.session_name.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                packed(theme.primary),
                packed(theme.border),
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                fit("  [Risk] Agent process will be interrupted immediately"),
                Style::new().fg(packed(theme.accent)).bold(),
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
                border_color: packed(theme.error),
                hit_id: HIT_ID_STOP_DIALOG,
            },
        );
    }
}
