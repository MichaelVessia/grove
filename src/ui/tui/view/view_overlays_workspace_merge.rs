use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_merge_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.merge_dialog() else {
            return;
        };
        if area.width < 28 || area.height < 14 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(98);
        let dialog_height = 17u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let cleanup_workspace_focused = focused(MergeDialogField::CleanupWorkspace);
        let cleanup_workspace_state = if dialog.cleanup_workspace {
            "enabled, remove workspace directory".to_string()
        } else {
            "disabled, keep workspace directory".to_string()
        };
        let cleanup_branch_focused = focused(MergeDialogField::CleanupLocalBranch);
        let cleanup_branch_state = if dialog.cleanup_local_branch {
            format!("enabled, delete '{}' branch", dialog.workspace_branch)
        } else {
            "disabled, keep local branch".to_string()
        };
        let merge_focused = focused(MergeDialogField::MergeButton);
        let cancel_focused = focused(MergeDialogField::CancelButton);
        let path = dialog.workspace_path.display().to_string();
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Merge plan"),
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
                "Base",
                dialog.base_branch.as_str(),
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
            modal_focus_badged_row(
                content_width,
                theme,
                "CleanupWorktree",
                cleanup_workspace_state.as_str(),
                cleanup_workspace_focused,
                packed(theme.accent),
                if dialog.cleanup_workspace {
                    packed(theme.error)
                } else {
                    packed(theme.text)
                },
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "CleanupBranch",
                cleanup_branch_state.as_str(),
                cleanup_branch_focused,
                packed(theme.accent),
                if dialog.cleanup_local_branch {
                    packed(theme.error)
                } else {
                    packed(theme.text)
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Merge",
                "Cancel",
                merge_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Space toggle cleanup, Enter or m merge, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Merge Workspace?",
                theme,
                border_color: packed(theme.accent),
                hit_id: HIT_ID_MERGE_DIALOG,
            },
        );
    }
}
