use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_rename_tab_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.rename_tab_dialog() else {
            return;
        };
        if area.width < 24 || area.height < 11 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 12u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Rename active tab title"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Current",
                dialog.current_title.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Title",
                dialog.title.as_str(),
                "Tab title",
                focused(RenameTabDialogField::Title),
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Rename",
                "Cancel",
                focused(RenameTabDialogField::RenameButton),
                focused(RenameTabDialogField::CancelButton),
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, type/backspace title, Enter rename, Esc cancel",
        ));
        let body = FtText::from_lines(lines);

        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Rename Tab",
                theme,
                border_color: packed(theme.info),
                hit_id: HIT_ID_RENAME_TAB_DIALOG,
            },
        );
    }
}
