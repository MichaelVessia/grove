use super::*;

impl GroveApp {
    pub(super) fn render_toasts(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        NotificationStack::new(&self.notifications)
            .margin(1)
            .render(area, frame);
    }

    pub(super) fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 11 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 11u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let unsafe_focused = focused(LaunchDialogField::Unsafe);
        let unsafe_state = if dialog.skip_permissions {
            "on, bypass approvals and sandbox"
        } else {
            "off, standard safety checks"
        };
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Launch profile", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Prompt",
                dialog.prompt.as_str(),
                "Describe initial task for the agent",
                focused(LaunchDialogField::Prompt),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "PreLaunch",
                dialog.pre_launch_command.as_str(),
                "Optional command to run before launch",
                focused(LaunchDialogField::PreLaunchCommand),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Unsafe",
                unsafe_state,
                unsafe_focused,
                theme.peach,
                if dialog.skip_permissions {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Start",
                "Cancel",
                start_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Tab move, Space toggle unsafe, Enter start, Esc cancel",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);
        let content = OverlayModalContent {
            title: "Start Agent",
            body,
            theme,
            border_color: theme.mauve,
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
            .hit_id(HitId::new(HIT_ID_LAUNCH_DIALOG))
            .render(area, frame);
    }

    pub(super) fn render_delete_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.delete_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 16u16;
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
        let delete_focused = focused(DeleteDialogField::DeleteButton);
        let cancel_focused = focused(DeleteDialogField::CancelButton);
        let delete_hint = pad_or_truncate_to_display_width(
            "Tab move, Space toggle branch cleanup, Enter or D delete, Esc cancel",
            content_width,
        );
        let path = dialog.path.display().to_string();
        let body = FtText::from_lines(vec![
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
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Delete",
                "Cancel",
                delete_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                delete_hint,
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Delete Worktree?",
            body,
            theme,
            border_color: theme.red,
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
            .hit_id(HitId::new(HIT_ID_DELETE_DIALOG))
            .render(area, frame);
    }

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
        let merge_hint = pad_or_truncate_to_display_width(
            "Tab move, Space toggle cleanup, Enter or m merge, Esc cancel",
            content_width,
        );
        let path = dialog.workspace_path.display().to_string();
        let body = FtText::from_lines(vec![
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
            FtLine::from_spans(vec![FtSpan::styled(
                merge_hint,
                Style::new().fg(theme.overlay0),
            )]),
        ]);

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
        let update_hint = pad_or_truncate_to_display_width(
            "Tab move, h/l switch buttons, Enter or u update, Esc cancel",
            content_width,
        );
        let path = dialog.workspace_path.display().to_string();
        let body = FtText::from_lines(vec![
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
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Strategy: git merge --no-ff <base> into workspace branch",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Update",
                "Cancel",
                update_focused,
                cancel_focused,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                update_hint,
                Style::new().fg(theme.overlay0),
            )]),
        ]);

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

    pub(super) fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };
        if area.width < 40 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 12u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let current = dialog.multiplexer.label();
        let multiplexer_focused = focused(SettingsDialogField::Multiplexer);
        let save_focused = focused(SettingsDialogField::SaveButton);
        let cancel_focused = focused(SettingsDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Global settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Multiplexer",
                current,
                multiplexer_focused,
                theme.blue,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  h/l, Left/Right, Space cycles", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Switching requires restarting running workspaces",
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
                save_focused,
                cancel_focused,
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Saved to ~/.config/grove/config.toml",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Settings",
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
            .hit_id(HitId::new(HIT_ID_SETTINGS_DIALOG))
            .render(area, frame);
    }

    pub(super) fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog.as_ref() else {
            return;
        };
        if area.width < 44 || area.height < 14 {
            return;
        }

        let theme = ui_theme();
        let dialog_width = area.width.saturating_sub(8).min(96);
        let content_width = usize::from(dialog_width.saturating_sub(2));

        if let Some(add_dialog) = dialog.add_dialog.as_ref() {
            let dialog_height = 12u16;
            let focused = |field| add_dialog.focused_field == field;
            let body = FtText::from_lines(vec![
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Name",
                    add_dialog.name.as_str(),
                    "Optional, defaults to directory name",
                    focused(ProjectAddDialogField::Name),
                ),
                modal_labeled_input_row(
                    content_width,
                    theme,
                    "Path",
                    add_dialog.path.as_str(),
                    "Absolute path or ~/path to repo root",
                    focused(ProjectAddDialogField::Path),
                ),
                FtLine::raw(""),
                modal_actions_row(
                    content_width,
                    theme,
                    "Add",
                    "Cancel",
                    focused(ProjectAddDialogField::AddButton),
                    focused(ProjectAddDialogField::CancelButton),
                ),
                FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "Tab move, Enter confirm, Esc back",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]),
            ]);
            let content = OverlayModalContent {
                title: "Add Project",
                body,
                theme,
                border_color: theme.mauve,
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
                .hit_id(HitId::new(HIT_ID_PROJECT_ADD_DIALOG))
                .render(area, frame);
            return;
        }

        let mut lines = Vec::new();
        lines.push(modal_labeled_input_row(
            content_width,
            theme,
            "Filter",
            dialog.filter.as_str(),
            "Type project name or path",
            true,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                format!("{} projects", self.projects.len()).as_str(),
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        lines.push(FtLine::raw(""));

        if dialog.filtered_project_indices.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No matches",
                Style::new().fg(theme.subtext0),
            )]));
        } else {
            for (visible_index, project_index) in
                dialog.filtered_project_indices.iter().take(8).enumerate()
            {
                let Some(project) = self.projects.get(*project_index) else {
                    continue;
                };
                let selected = visible_index == dialog.selected_filtered_index;
                let marker = if selected { ">" } else { " " };
                let name_style = if selected {
                    Style::new().fg(theme.mauve).bold()
                } else {
                    Style::new().fg(theme.text)
                };
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("{marker} {}", project.name),
                    name_style,
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        format!("  {}", project.path.display()).as_str(),
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Enter focus, Up/Down or Tab/S-Tab navigate, Ctrl+A add, Esc close",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));

        let content = OverlayModalContent {
            title: "Projects",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.teal,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(16)
                    .max_height(20),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG))
            .render(area, frame);
    }

    pub(super) fn render_command_palette_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.command_palette.is_visible() {
            return;
        }

        self.command_palette.render(area, frame);
    }

    pub(super) fn render_keybind_help_overlay(&self, frame: &mut Frame, area: Rect) {
        if !self.keybind_help_open {
            return;
        }
        if area.width < 56 || area.height < 18 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(108);
        let dialog_height = area.height.saturating_sub(6).clamp(18, 26);
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let global_hints = self.keybind_help_line(HelpHintContext::Global);
        let workspace_hints = self.keybind_help_line(HelpHintContext::Workspace);
        let list_hints = self.keybind_help_line(HelpHintContext::List);
        let preview_agent_hints = self.keybind_help_line(HelpHintContext::PreviewAgent);
        let preview_git_hints = self.keybind_help_line(HelpHintContext::PreviewGit);

        let lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Global]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  {global_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  {workspace_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[List]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(format!("  {list_hints}").as_str(), content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Preview]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  Agent tab: {preview_agent_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  Git tab: {preview_git_hints}").as_str(),
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("[Interactive]", content_width),
                Style::new().fg(theme.blue).bold(),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  type sends input to agent", content_width),
                Style::new().fg(theme.text),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Esc Esc or Ctrl+\\ exit, Alt+C copy, Alt+V paste",
                    content_width,
                ),
                Style::new().fg(theme.text),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Create: Tab/S-Tab fields, j/k or C-n/C-p move, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Edit:   Tab/S-Tab fields, h/l or Space toggle agent, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Start:  Tab/S-Tab fields, Space toggle unsafe, h/l buttons, Enter/Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Delete: Tab/S-Tab fields, j/k move, Space toggle, Enter/D confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Merge:  Tab/S-Tab fields, j/k move, Space toggle, Enter/m confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "[Modals] Update: Tab/S-Tab fields, h/l buttons, Enter/u confirm, Esc",
                    content_width,
                ),
                Style::new().fg(theme.subtext0),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Close help: Esc, Enter, or ?", content_width),
                Style::new().fg(theme.lavender).bold(),
            )]),
        ];

        let content = OverlayModalContent {
            title: "Keybind Help",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.blue,
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
            .hit_id(HitId::new(HIT_ID_KEYBIND_HELP_DIALOG))
            .render(area, frame);
    }

    pub(super) fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 16u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let selected_project_label = self
            .projects
            .get(dialog.project_index)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "(missing project)".to_string());

        let focused = |field| dialog.focused_field == field;
        let selected_agent = dialog.agent;
        let selected_agent_style = Style::new()
            .fg(theme.text)
            .bg(if focused(CreateDialogField::Agent) {
                theme.surface1
            } else {
                theme.base
            })
            .bold();
        let unselected_agent_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let selected_dropdown_style = Style::new().fg(theme.text).bg(theme.surface1).bold();
        let unselected_dropdown_style = Style::new().fg(theme.subtext0).bg(theme.base);
        let agent_row = |agent: AgentType| {
            let is_selected = selected_agent == agent;
            let prefix = if is_selected { "▸" } else { " " };
            let line = pad_or_truncate_to_display_width(
                format!("{} [Agent] {}", prefix, agent.label()).as_str(),
                content_width,
            );
            if is_selected {
                FtLine::from_spans(vec![FtSpan::styled(line, selected_agent_style)])
            } else {
                FtLine::from_spans(vec![FtSpan::styled(line, unselected_agent_style)])
            }
        };

        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Workspace setup", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                "feature-name",
                focused(CreateDialogField::WorkspaceName),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "Project",
                selected_project_label.as_str(),
                "j/k or C-n/C-p select",
                focused(CreateDialogField::Project),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "BaseBranch",
                dialog.base_branch.as_str(),
                "current branch (fallback: main/master)",
                focused(CreateDialogField::BaseBranch),
            ),
        ];
        if focused(CreateDialogField::Project)
            && let Some(project) = self.projects.get(dialog.project_index)
        {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    format!("  [ProjectPath] {}", project.path.display()).as_str(),
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]));
        }
        if focused(CreateDialogField::BaseBranch) {
            if self.create_branch_all.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] Loading branches...",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else if self.create_branch_filtered.is_empty() {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    pad_or_truncate_to_display_width(
                        "  [Branches] No matching branches",
                        content_width,
                    ),
                    Style::new().fg(theme.overlay0),
                )]));
            } else {
                let max_dropdown = 4usize;
                for (index, branch) in self.create_branch_filtered.iter().enumerate() {
                    if index >= max_dropdown {
                        break;
                    }
                    let is_selected = index == self.create_branch_index;
                    let prefix = if is_selected { "▸" } else { " " };
                    let line = pad_or_truncate_to_display_width(
                        format!("{prefix} [Branches] {branch}").as_str(),
                        content_width,
                    );
                    if is_selected {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            selected_dropdown_style,
                        )]));
                    } else {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            line,
                            unselected_dropdown_style,
                        )]));
                    }
                }
                if self.create_branch_filtered.len() > max_dropdown {
                    lines.push(FtLine::from_spans(vec![FtSpan::styled(
                        pad_or_truncate_to_display_width(
                            format!(
                                "  [Branches] ... and {} more",
                                self.create_branch_filtered.len() - max_dropdown
                            )
                            .as_str(),
                            content_width,
                        ),
                        Style::new().fg(theme.overlay0),
                    )]));
                }
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(agent_row(AgentType::Claude));
        lines.push(agent_row(AgentType::Codex));
        lines.push(FtLine::raw(""));
        let create_focused = focused(CreateDialogField::CreateButton);
        let cancel_focused = focused(CreateDialogField::CancelButton);
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Create",
            "Cancel",
            create_focused,
            cancel_focused,
        ));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            pad_or_truncate_to_display_width(
                "Tab move, j/k or C-n/C-p adjust project/branch, Enter create, Esc cancel",
                content_width,
            ),
            Style::new().fg(theme.overlay0),
        )]));
        let content = OverlayModalContent {
            title: "New Workspace",
            body: FtText::from_lines(lines),
            theme,
            border_color: theme.mauve,
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
            .hit_id(HitId::new(HIT_ID_CREATE_DIALOG))
            .render(area, frame);
    }

    pub(super) fn render_edit_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.edit_dialog.as_ref() else {
            return;
        };
        if area.width < 24 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(10).min(80);
        let dialog_height = 13u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let path = dialog.workspace_path.display().to_string();
        let running_note = if dialog.was_running {
            "Running now, restart agent to apply change"
        } else {
            "Agent change applies on next agent start"
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
                    "Tab move, h/l or Space toggle agent, Enter save, Esc cancel",
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
