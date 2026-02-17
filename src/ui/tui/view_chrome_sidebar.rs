use super::*;

impl GroveApp {
    pub(super) fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Workspaces")
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(
                self.state.focus == PaneFocus::WorkspaceList && !self.modal_open(),
            ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_WORKSPACE_LIST));

        if inner.is_empty() {
            return;
        }

        let theme = ui_theme();
        let mut lines: Vec<FtLine> = Vec::new();
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let max_lines = usize::from(inner.height);
        if self.projects.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No projects configured",
                Style::new().fg(theme.subtext0),
            )]));
            lines.push(FtLine::raw(""));
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "Press 'p' to add a project",
                Style::new().fg(theme.text).bold(),
            )]));
        } else if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "Discovery error",
                    Style::new().fg(theme.red).bold(),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    message.as_str(),
                    Style::new().fg(theme.peach),
                )]));
            }
        } else {
            for (project_index, project) in self.projects.iter().enumerate() {
                if lines.len() >= max_lines {
                    break;
                }
                if project_index > 0 && lines.len() < max_lines {
                    lines.push(FtLine::raw(""));
                }
                if lines.len() >= max_lines {
                    break;
                }
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("▾ {}", project.name),
                    Style::new().fg(theme.overlay0).bold(),
                )]));

                let project_workspaces: Vec<(usize, &Workspace)> = self
                    .state
                    .workspaces
                    .iter()
                    .enumerate()
                    .filter(|(_, workspace)| {
                        workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                    })
                    .collect();

                if project_workspaces.is_empty() {
                    if lines.len() < max_lines {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            "  (no workspaces)",
                            Style::new().fg(theme.subtext0),
                        )]));
                    }
                    continue;
                }

                for (idx, workspace) in project_workspaces {
                    if lines
                        .len()
                        .saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT))
                        > max_lines
                    {
                        break;
                    }

                    let row_y = inner
                        .y
                        .saturating_add(u16::try_from(lines.len()).unwrap_or(u16::MAX));
                    let is_selected = idx == self.state.selected_index;
                    let is_working = self.status_is_visually_working(
                        Some(workspace.path.as_path()),
                        workspace.status,
                        is_selected,
                    );
                    let selected = if is_selected { "▸" } else { " " };
                    let row_background = if is_selected {
                        if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                            Some(theme.surface1)
                        } else {
                            Some(theme.surface0)
                        }
                    } else {
                        None
                    };

                    let mut primary_style = Style::new().fg(theme.text);
                    let mut secondary_style = Style::new().fg(theme.subtext0);
                    if let Some(bg) = row_background {
                        primary_style = primary_style.bg(bg);
                        secondary_style = secondary_style.bg(bg);
                    }
                    if is_selected {
                        primary_style = primary_style.bold();
                    }

                    let workspace_label_style = if is_working {
                        primary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold()
                    } else {
                        primary_style
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    let show_branch = workspace.branch != workspace_name;
                    let branch_text = if show_branch {
                        format!(" · {}", workspace.branch)
                    } else {
                        String::new()
                    };
                    let agent_separator = " · ";
                    let mut row_spans = vec![
                        FtSpan::styled(format!("{selected} "), primary_style),
                        FtSpan::styled(workspace_name.clone(), workspace_label_style),
                    ];
                    if !branch_text.is_empty() {
                        row_spans.push(FtSpan::styled(branch_text.clone(), secondary_style));
                    }
                    row_spans.push(FtSpan::styled(agent_separator, secondary_style));
                    row_spans.push(FtSpan::styled(
                        workspace.agent.label().to_string(),
                        secondary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold(),
                    ));
                    let workspace_is_deleting = self.delete_in_flight
                        && self
                            .delete_in_flight_workspace
                            .as_ref()
                            .is_some_and(|path| path == &workspace.path);
                    if workspace_is_deleting {
                        row_spans.push(FtSpan::styled(
                            " · Deleting...",
                            secondary_style.fg(theme.peach).bold(),
                        ));
                    }
                    if workspace.is_orphaned {
                        row_spans.push(FtSpan::styled(
                            " · session ended",
                            secondary_style.fg(theme.peach),
                        ));
                    }
                    lines.push(FtLine::from_spans(row_spans));

                    if is_working {
                        let primary_label_x = inner.x.saturating_add(
                            u16::try_from(text_display_width("▸ ")).unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace_name.clone(),
                            workspace.agent,
                            primary_label_x,
                            row_y,
                        ));
                        let agent_prefix =
                            format!("{workspace_name}{branch_text}{agent_separator}");
                        let secondary_label_x = inner.x.saturating_add(
                            u16::try_from(
                                text_display_width("▸ ")
                                    .saturating_add(text_display_width(&agent_prefix)),
                            )
                            .unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace.agent.label().to_string(),
                            workspace.agent,
                            secondary_label_x,
                            row_y,
                        ));
                    }

                    if let Ok(data) = u64::try_from(idx) {
                        let row_height =
                            WORKSPACE_ITEM_HEIGHT.min(inner.bottom().saturating_sub(row_y));
                        let row_rect = Rect::new(inner.x, row_y, inner.width, row_height);
                        let _ = frame.register_hit(
                            row_rect,
                            HitId::new(HIT_ID_WORKSPACE_ROW),
                            FrameHitRegion::Content,
                            data,
                        );
                    }
                }
            }
        }

        Paragraph::new(FtText::from_lines(lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
    }
}
