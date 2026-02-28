impl GroveApp {
    fn pull_request_status_icon(status: crate::domain::PullRequestStatus) -> &'static str {
        match status {
            crate::domain::PullRequestStatus::Open => "",
            crate::domain::PullRequestStatus::Merged => "",
            crate::domain::PullRequestStatus::Closed => "",
        }
    }

    fn pull_request_status_style(
        status: crate::domain::PullRequestStatus,
        secondary_style: Style,
        theme: UiTheme,
    ) -> Style {
        match status {
            crate::domain::PullRequestStatus::Open => secondary_style.fg(theme.teal).bold(),
            crate::domain::PullRequestStatus::Merged => secondary_style.fg(theme.mauve).bold(),
            crate::domain::PullRequestStatus::Closed => secondary_style.fg(theme.red).bold(),
        }
    }

    fn project_workspace_indices(&self, project: &ProjectConfig) -> Vec<usize> {
        self.state
            .workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| {
                workspace
                    .project_path
                    .as_ref()
                    .is_some_and(|path| refer_to_same_location(path, &project.path))
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn build_sidebar_lines(&self, theme: UiTheme) -> (Vec<SidebarListLine>, Option<usize>) {
        let mut lines = Vec::new();
        let mut selected_line = None;

        for (project_index, project) in self.projects.iter().enumerate() {
            if project_index > 0 {
                lines.push(SidebarListLine::project(Vec::new()));
            }

            lines.push(SidebarListLine::project(vec![SidebarSegment {
                text: format!("▾ {}", project.name),
                style: Style::new().fg(theme.overlay0).bold(),
            }]));

            let workspace_indices = self.project_workspace_indices(project);
            if workspace_indices.is_empty() {
                lines.push(SidebarListLine::project(vec![SidebarSegment {
                    text: "  (no workspaces)".to_string(),
                    style: Style::new().fg(theme.subtext0),
                }]));
                continue;
            }

            for workspace_index in workspace_indices {
                let Some(workspace) = self.state.workspaces.get(workspace_index) else {
                    continue;
                };

                let is_selected = workspace_index == self.state.selected_index;
                if is_selected && selected_line.is_none() {
                    selected_line = Some(lines.len());
                }

                let is_working = self.status_is_visually_working(
                    Some(workspace.path.as_path()),
                    workspace.status,
                    is_selected,
                );
                let (attention_symbol, attention_color) = self
                    .workspace_attention_indicator(workspace.path.as_path())
                    .unwrap_or((" ", theme.overlay0));
                let row_background = if is_selected {
                    if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                        Some(theme.surface1)
                    } else {
                        Some(theme.surface0)
                    }
                } else {
                    None
                };

                let mut border_style = if is_selected {
                    Style::new().fg(theme.blue)
                } else {
                    Style::new().fg(theme.surface1)
                };
                if let Some(bg) = row_background {
                    border_style = border_style.bg(bg);
                }
                if is_selected {
                    border_style = border_style.bold();
                }

                let row_style = row_background.map_or_else(Style::new, |bg| Style::new().bg(bg));
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
                let branch_text = if workspace.branch != workspace_name {
                    format!(" · {}", workspace.branch)
                } else {
                    String::new()
                };

                let line1_prefix = "   ";
                let line1_attention_gap = " ";
                let line1_prefix_width = text_display_width(line1_prefix)
                    .saturating_add(text_display_width(attention_symbol))
                    .saturating_add(text_display_width(line1_attention_gap));
                let mut line1_segments = vec![
                    SidebarSegment {
                        text: line1_prefix.to_string(),
                        style: primary_style,
                    },
                    SidebarSegment {
                        text: attention_symbol.to_string(),
                        style: primary_style.fg(attention_color).bold(),
                    },
                    SidebarSegment {
                        text: line1_attention_gap.to_string(),
                        style: primary_style,
                    },
                    SidebarSegment {
                        text: workspace_name.clone(),
                        style: workspace_label_style,
                    },
                ];
                if !branch_text.is_empty() {
                    line1_segments.push(SidebarSegment {
                        text: branch_text,
                        style: secondary_style,
                    });
                }

                let line2_prefix = "     ";
                let line2_prefix_width = text_display_width(line2_prefix);
                let agent_label = workspace.agent.label().to_string();
                let mut line2_segments = vec![
                    SidebarSegment {
                        text: line2_prefix.to_string(),
                        style: secondary_style,
                    },
                    SidebarSegment {
                        text: agent_label.clone(),
                        style: secondary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold(),
                    },
                ];
                let mut line2_width =
                    line2_prefix_width.saturating_add(text_display_width(&agent_label));
                let mut pr_hits = Vec::new();
                if !workspace.is_main && !workspace.pull_requests.is_empty() {
                    line2_segments.push(SidebarSegment {
                        text: " · PRs:".to_string(),
                        style: secondary_style,
                    });
                    line2_width = line2_width.saturating_add(text_display_width(" · PRs:"));
                    for (pull_request_index, pull_request) in
                        workspace.pull_requests.iter().enumerate()
                    {
                        line2_segments.push(SidebarSegment {
                            text: " ".to_string(),
                            style: secondary_style,
                        });
                        line2_width = line2_width.saturating_add(1);
                        let pull_request_label = format!(
                            "{} #{}",
                            Self::pull_request_status_icon(pull_request.status),
                            pull_request.number
                        );
                        let token_width = text_display_width(&pull_request_label);
                        if let Some(hit_data) =
                            encode_workspace_pr_hit_data(workspace_index, pull_request_index)
                        {
                            pr_hits.push(SidebarPrHit {
                                start_col: line2_width,
                                width: token_width,
                                data: hit_data,
                            });
                        }
                        line2_segments.push(SidebarSegment {
                            text: pull_request_label,
                            style: Self::pull_request_status_style(
                                pull_request.status,
                                secondary_style,
                                theme,
                            )
                            .underline(),
                        });
                        line2_width = line2_width.saturating_add(token_width);
                    }
                }
                if self.delete_requested_workspaces.contains(&workspace.path) {
                    line2_segments.push(SidebarSegment {
                        text: " · Deleting...".to_string(),
                        style: secondary_style.fg(theme.peach).bold(),
                    });
                }
                if workspace.is_orphaned {
                    line2_segments.push(SidebarSegment {
                        text: " · session ended".to_string(),
                        style: secondary_style.fg(theme.peach),
                    });
                }

                let top_activity = if is_working {
                    Some(SidebarActivityLabel {
                        label: workspace_name,
                        agent: workspace.agent,
                        start_col: line1_prefix_width,
                    })
                } else {
                    None
                };
                let meta_activity = if is_working {
                    Some(SidebarActivityLabel {
                        label: workspace.agent.label().to_string(),
                        agent: workspace.agent,
                        start_col: line2_prefix_width,
                    })
                } else {
                    None
                };

                lines.push(SidebarListLine::workspace(
                    line1_segments,
                    workspace_index,
                    border_style,
                    row_style,
                    Vec::new(),
                    top_activity,
                ));
                lines.push(SidebarListLine::workspace(
                    line2_segments,
                    workspace_index,
                    border_style,
                    row_style,
                    pr_hits,
                    meta_activity,
                ));
                lines.push(SidebarListLine::workspace(
                    Vec::new(),
                    workspace_index,
                    border_style,
                    row_style,
                    Vec::new(),
                    None,
                ));
            }
        }

        (lines, selected_line)
    }

    pub(super) fn sidebar_workspace_row_map(&self) -> Vec<Option<usize>> {
        let (lines, _) = self.build_sidebar_lines(ui_theme());
        lines
            .iter()
            .map(SidebarListLine::workspace_index)
            .collect::<Vec<Option<usize>>>()
    }
}
