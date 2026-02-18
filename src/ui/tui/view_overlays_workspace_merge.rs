use super::*;

impl GroveApp {
    pub(super) fn render_merge_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.merge_dialog.as_ref() else {
            return;
        };
        if area.width < 28 || area.height < 14 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(98);
        let dialog_height = 17u16;
        let theme = ui_theme();
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
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Merge plan", content_width),
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
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "CleanupWorktree",
                cleanup_workspace_state.as_str(),
                cleanup_workspace_focused,
                theme.peach,
                if dialog.cleanup_workspace {
                    theme.red
                } else {
                    theme.text
                },
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "CleanupBranch",
                cleanup_branch_state.as_str(),
                cleanup_branch_focused,
                theme.peach,
                if dialog.cleanup_local_branch {
                    theme.red
                } else {
                    theme.text
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

        let content = OverlayModalContent {
            title: "Merge Workspace?",
            body,
            theme,
            border_color: theme.peach,
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
            .hit_id(HitId::new(HIT_ID_MERGE_DIALOG))
            .render(area, frame);
    }
}
