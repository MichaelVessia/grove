use super::*;

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
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let path = dialog.workspace_path.display().to_string();
        let running_note = if dialog.was_running && dialog.is_main {
            "Branch switches now, restart agent to apply agent change"
        } else if dialog.was_running {
            "Base branch applies now, restart agent to apply agent change"
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
            "Tab/C-n next, S-Tab/C-p prev, type/backspace branch, h/l or Space toggle agent, Enter save, Esc cancel"
        } else {
            "Tab/C-n next, S-Tab/C-p prev, type/backspace base branch, h/l or Space toggle agent, Enter save, Esc cancel"
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace settings", content_width),
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
                dialog.branch.as_str(),
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
            modal_labeled_input_row(
                content_width,
                theme,
                branch_field_label,
                dialog.base_branch.as_str(),
                "main",
                focused(EditDialogField::BaseBranch),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Agent",
                dialog.agent.label(),
                focused(EditDialogField::Agent),
                theme.peach,
                theme.text,
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
                border_color: theme.teal,
                hit_id: HIT_ID_EDIT_DIALOG,
            },
        );
    }
}
