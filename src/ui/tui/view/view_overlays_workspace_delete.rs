use super::view_prelude::*;

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
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let warning_lines =
            if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task {
                (
                    "  • Remove the task manifest from Grove's task list",
                    "  • Keep the primary checkout and local branch untouched",
                )
            } else if dialog.is_missing {
                (
                    "  • Selected path already removed",
                    "  • Clean up git worktree metadata",
                )
            } else if dialog.deletes_task()
                && matches!(dialog.target, DeleteDialogTarget::Worktree { .. })
            {
                (
                    "  • Delete the final worktree in this task",
                    "  • The task will also be removed because no worktrees remain",
                )
            } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                (
                    "  • Remove only the selected worktree from this task",
                    "  • Uncommitted changes in this worktree will be lost",
                )
            } else {
                (
                    "  • Remove the task root and all child worktrees",
                    "  • Uncommitted changes in any worktree will be lost",
                )
            };
        let cleanup_focused = focused(DeleteDialogField::DeleteLocalBranch);
        let cleanup_state = if !dialog.delete_local_branch_enabled() {
            "disabled, keep local branch".to_string()
        } else if dialog.delete_local_branch {
            let branch = match &dialog.target {
                DeleteDialogTarget::Task => dialog.task.branch.as_str(),
                DeleteDialogTarget::Worktree { worktree, .. } => worktree.branch.as_str(),
            };
            format!("enabled, remove '{branch}' local branch")
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
        let (name, branch, path) = match &dialog.target {
            DeleteDialogTarget::Task => (
                dialog.task.name.as_str(),
                dialog.task.branch.as_str(),
                dialog.task.root_path.display().to_string(),
            ),
            DeleteDialogTarget::Worktree { worktree, .. } => (
                worktree.repository_name.as_str(),
                worktree.branch.as_str(),
                worktree.path.display().to_string(),
            ),
        };
        let worktree_count = dialog.task.worktrees.len().to_string();
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Deletion plan"),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(content_width, theme, "Name", name, theme.blue, theme.text),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                branch,
                theme.blue,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Worktrees",
                worktree_count.as_str(),
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
                fit(
                    if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task {
                        "  [Info] Remove from Grove only"
                    } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                        "  [Scope] Selected worktree only"
                    } else {
                        "  [Risk] Changes are destructive"
                    },
                ),
                Style::new()
                    .fg(
                        if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task
                        {
                            theme.blue
                        } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                            theme.yellow
                        } else {
                            theme.peach
                        },
                    )
                    .bold(),
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
                if dialog.is_base_task {
                    theme.overlay0
                } else if dialog.delete_local_branch {
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
                if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task {
                    "Remove"
                } else {
                    "Delete"
                },
                "Cancel",
                delete_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task {
                "Tab/C-n next, S-Tab/C-p prev, Space toggle option, Enter or D remove task, Esc cancel"
            } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                "Tab/C-n next, S-Tab/C-p prev, Space toggle option, Enter or d delete worktree, Esc cancel"
            } else {
                "Tab/C-n next, S-Tab/C-p prev, Space toggle option, Enter or D delete task, Esc cancel"
            },
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: if matches!(dialog.target, DeleteDialogTarget::Task) && dialog.is_base_task {
                    "Remove Task From List?"
                } else if dialog.deletes_task()
                    && matches!(dialog.target, DeleteDialogTarget::Worktree { .. })
                {
                    "Delete Final Worktree?"
                } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                    "Delete Worktree?"
                } else {
                    "Delete Task?"
                },
                theme,
                border_color: if matches!(dialog.target, DeleteDialogTarget::Task)
                    && dialog.is_base_task
                {
                    theme.blue
                } else if matches!(dialog.target, DeleteDialogTarget::Worktree { .. }) {
                    theme.yellow
                } else {
                    theme.red
                },
                hit_id: HIT_ID_DELETE_DIALOG,
            },
        );
    }
}
