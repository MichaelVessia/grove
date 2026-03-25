use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_edit_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.edit_dialog() else {
            return;
        };
        if area.width < 24 || area.height < 13 {
            return;
        }

        let dialog_width = area.width.saturating_sub(10).min(80);
        let dialog_height = 14u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let path = dialog.workspace_path.display().to_string();
        let running_note = if dialog.was_running && dialog.is_main {
            "Branch switches now, restart active tabs to apply change"
        } else if dialog.was_running {
            "Base branch applies now, restart active tabs to apply change"
        } else if dialog.is_main {
            "Branch switches immediately"
        } else {
            "Base branch applies immediately"
        };
        let branch_field_label = if dialog.is_main {
            "Branch"
        } else {
            "BaseBranch"
        };
        let edit_hint = if dialog.is_main {
            "Tab/C-n next, S-Tab/C-p prev, type/backspace branch, Enter save, Esc cancel"
        } else {
            "Tab/C-n next, S-Tab/C-p prev, type/backspace base branch, Enter save, Esc cancel"
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
                fit("Workspace settings"),
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
                dialog.branch.as_str(),
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
            modal_labeled_input_row(
                content_width,
                theme,
                branch_field_label,
                dialog.base_branch.as_str(),
                "main",
                focused(EditDialogField::BaseBranch),
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            format!("  [Note] {running_note}").as_str(),
        ));
        lines.push(FtLine::raw(""));
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Save",
            "Cancel",
            focused(EditDialogField::SaveButton),
            focused(EditDialogField::CancelButton),
        ));
        lines.extend(modal_wrapped_hint_rows(content_width, theme, edit_hint));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Edit Workspace",
                theme,
                border_color: packed(theme.info),
                hit_id: HIT_ID_EDIT_DIALOG,
            },
        );
    }
}
