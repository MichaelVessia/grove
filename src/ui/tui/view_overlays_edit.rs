use super::*;

impl GroveApp {
    pub(super) fn render_edit_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.edit_dialog.as_ref() else {
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
        let running_note = if dialog.was_running {
            "Base branch applies now, restart agent to apply agent change"
        } else {
            "Base branch applies immediately"
        };

        let body = FtText::from_lines(vec![
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
                "BaseBranch",
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
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [Note] {running_note}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                focused(EditDialogField::SaveButton),
                focused(EditDialogField::CancelButton),
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, type/backspace base branch, h/l or Space toggle agent, Enter save, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Edit Workspace",
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
            .hit_id(HitId::new(HIT_ID_EDIT_DIALOG))
            .render(area, frame);
    }
}
