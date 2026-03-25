use super::view_prelude::*;

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
        let theme = self.active_ui_theme();
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
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Update plan"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.workspace_branch.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                base_label,
                base_value.as_str(),
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
        ];
        lines.extend(modal_wrapped_rows(
            content_width,
            strategy,
            Style::new()
                .fg(packed(theme.text_subtle))
                .bg(packed(theme.background)),
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
                border_color: packed(theme.info),
                hit_id: HIT_ID_UPDATE_FROM_BASE_DIALOG,
            },
        );
    }
}
