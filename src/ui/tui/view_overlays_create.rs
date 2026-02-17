use super::*;

impl GroveApp {
    pub(super) fn render_create_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.create_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 10 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(90);
        let dialog_height = 20u16;
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
            modal_labeled_input_row(
                content_width,
                theme,
                "SetupCmds",
                dialog.setup_commands.as_str(),
                "direnv allow; cmd2; cmd3",
                focused(CreateDialogField::SetupCommands),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "AutoRun",
                if dialog.auto_run_setup_commands {
                    "on"
                } else {
                    "off"
                },
                focused(CreateDialogField::AutoRunSetupCommands),
                theme.peach,
                theme.text,
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
        if focused(CreateDialogField::SetupCommands) {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  [SetupCmds] Separate commands with ';'",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]));
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
                "Tab/C-n next, S-Tab/C-p prev, j/k adjust project/branch, Space toggles auto-run, Enter create, Esc cancel",
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
}
