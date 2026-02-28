use super::*;

impl GroveApp {
    pub(super) fn render_delete_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.delete_dialog() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 17u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let warning_lines = if dialog.is_missing {
            (
                "  • Directory already removed",
                "  • Clean up git worktree metadata",
            )
        } else {
            (
                "  • Remove the working directory",
                "  • Uncommitted changes will be lost",
            )
        };
        let cleanup_focused = focused(DeleteDialogField::DeleteLocalBranch);
        let cleanup_state = if dialog.delete_local_branch {
            format!("enabled, remove '{}' branch locally", dialog.branch)
        } else {
            "disabled, keep local branch".to_string()
        };
        let kill_sessions_focused = focused(DeleteDialogField::KillTmuxSessions);
        let kill_sessions_state = if dialog.kill_tmux_sessions {
            "enabled, kill Grove tmux sessions".to_string()
        } else {
            "disabled, keep tmux sessions running".to_string()
        };
        let delete_focused = focused(DeleteDialogField::DeleteButton);
        let cancel_focused = focused(DeleteDialogField::CancelButton);
        let path = dialog.path.display().to_string();
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Deletion plan", content_width),
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
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  [Risk] Changes are destructive", content_width),
                Style::new().fg(theme.peach).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.0,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                warning_lines.1,
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "BranchCleanup",
                cleanup_state.as_str(),
                cleanup_focused,
                theme.peach,
                if dialog.delete_local_branch {
                    theme.red
                } else {
                    theme.text
                },
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "SessionCleanup",
                kill_sessions_state.as_str(),
                kill_sessions_focused,
                theme.peach,
                if dialog.kill_tmux_sessions {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Delete",
                "Cancel",
                delete_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Space toggle option, Enter or D delete, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Delete Worktree?",
                theme,
                border_color: theme.red,
                hit_id: HIT_ID_DELETE_DIALOG,
            },
        );
    }
}
