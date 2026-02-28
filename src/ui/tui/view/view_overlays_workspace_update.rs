use super::*;

impl GroveApp {
    pub(super) fn render_update_from_base_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.update_from_base_dialog() else {
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
        let (title, strategy, base_label, base_value) = if dialog.is_main_workspace {
            (
                "Update From Upstream?",
                "  Strategy: git pull --ff-only origin <branch> in base workspace",
                "Upstream",
                format!("origin/{}", dialog.base_branch),
            )
        } else {
            (
                "Update From Base?",
                "  Strategy: git merge --no-ff <base> into workspace branch",
                "Base",
                dialog.base_branch.clone(),
            )
        };
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
                base_label,
                base_value.as_str(),
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
            strategy,
            Style::new().fg(theme.subtext0).bg(theme.base),
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
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title,
                theme,
                border_color: theme.teal,
                hit_id: HIT_ID_UPDATE_FROM_BASE_DIALOG,
            },
        );
    }
}
