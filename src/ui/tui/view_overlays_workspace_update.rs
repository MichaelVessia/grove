use super::*;

impl GroveApp {
    pub(super) fn render_update_from_base_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.update_from_base_dialog.as_ref() else {
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
        let update_focused = focused(UpdateFromBaseDialogField::UpdateButton);
        let cancel_focused = focused(UpdateFromBaseDialogField::CancelButton);
        let path = dialog.workspace_path.display().to_string();
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Update plan", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.workspace_branch.as_str(),
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Base",
                dialog.base_branch.as_str(),
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
        ];
        lines.extend(modal_wrapped_rows(
            content_width,
            "  Strategy: git merge --no-ff <base> into workspace branch",
            Style::new().fg(theme.subtext0),
        ));
        lines.push(FtLine::raw(""));
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Update",
            "Cancel",
            update_focused,
            cancel_focused,
        ));
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, h/l switch buttons, Enter or u update, Esc cancel",
        ));
        let body = FtText::from_lines(lines);

        let content = OverlayModalContent {
            title: "Update From Base?",
            body,
            theme,
            border_color: theme.teal,
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
            .hit_id(HitId::new(HIT_ID_UPDATE_FROM_BASE_DIALOG))
            .render(area, frame);
    }
}
