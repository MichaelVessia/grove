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

    fn push_workspace_sidebar_lines(
        &self,
        lines: &mut Vec<SidebarListLine>,
        selected_line: &mut Option<usize>,
        theme: UiTheme,
        workspace_index: usize,
    ) {
        let Some(workspace) = self.state.workspaces.get(workspace_index) else {
            return;
        };

        let is_selected = workspace_index == self.state.selected_index;
        if is_selected && selected_line.is_none() {
            *selected_line = Some(lines.len());
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
        let workspace_label = workspace
            .project_name
            .as_ref()
            .filter(|_| !workspace.is_main)
            .map(|project_name| format!("{workspace_name} ({project_name})"))
            .unwrap_or_else(|| workspace_name.clone());
        let branch_text = if workspace.branch != workspace_name {
            format!(" · {}", workspace.branch)
        } else {
            String::new()
        };

        let line1_prefix = "   ";
        let line1_attention_gap = " ";
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
                text: workspace_label.clone(),
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
        let mut line2_segments = vec![SidebarSegment {
            text: line2_prefix.to_string(),
            style: secondary_style,
        }];
        let mut line2_width = line2_prefix_width;
        let mut pr_hits = Vec::new();
        let waiting_snippet = self.sidebar_waiting_snippet(workspace.path.as_path());
        let local_input_pending =
            self.workspace_has_pending_local_input(workspace.path.as_path(), is_selected);
        if workspace.status == WorkspaceStatus::Waiting && !local_input_pending {
            line2_segments.push(SidebarSegment {
                text: "WAITING".to_string(),
                style: secondary_style.fg(theme.yellow).bold(),
            });
            if let Some(snippet) = waiting_snippet {
                line2_segments.push(SidebarSegment {
                    text: format!(" · {snippet}"),
                    style: secondary_style,
                });
            }
        } else if is_working || local_input_pending {
            line2_segments.push(SidebarSegment {
                text: "WORKING".to_string(),
                style: secondary_style.fg(self.workspace_agent_color(workspace.agent)).bold(),
            });
        } else if self.dialogs.delete_requested_workspaces.contains(&workspace.path) {
            line2_segments.push(SidebarSegment {
                text: "Deleting...".to_string(),
                style: secondary_style.fg(theme.peach).bold(),
            });
        } else if workspace.is_orphaned {
            line2_segments.push(SidebarSegment {
                text: "session ended".to_string(),
                style: secondary_style.fg(theme.peach),
            });
        } else if !workspace.is_main && !workspace.pull_requests.is_empty() {
            line2_segments.push(SidebarSegment {
                text: "PRs:".to_string(),
                style: secondary_style,
            });
            line2_width = line2_width.saturating_add(text_display_width("PRs:"));
            for (pull_request_index, pull_request) in workspace.pull_requests.iter().enumerate() {
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

        lines.push(SidebarListLine::workspace(
            line1_segments,
            workspace_index,
            border_style,
            row_style,
            Vec::new(),
        ));
        lines.push(SidebarListLine::workspace(
            line2_segments,
            workspace_index,
            border_style,
            row_style,
            pr_hits,
        ));
        lines.push(SidebarListLine::workspace(
            Vec::new(),
            workspace_index,
            border_style,
            row_style,
            Vec::new(),
        ));
    }

    fn build_sidebar_lines(&self, theme: UiTheme) -> (Vec<SidebarListLine>, Option<usize>) {
        let mut lines = Vec::new();
        let mut selected_line = None;
        let mut workspace_index = 0usize;

        for (task_index, task) in self.state.tasks.iter().enumerate() {
            if task_index > 0 {
                lines.push(SidebarListLine::project(Vec::new()));
            }

            lines.push(SidebarListLine::project(vec![SidebarSegment {
                text: format!("{} {}", self.task_header_marker(task), task.name),
                style: Style::new().fg(theme.overlay0).bold(),
            }]));

            if task.worktrees.is_empty() {
                lines.push(SidebarListLine::project(vec![SidebarSegment {
                    text: "  (no worktrees)".to_string(),
                    style: Style::new().fg(theme.subtext0),
                }]));
                continue;
            }

            for _worktree in &task.worktrees {
                if self.state.workspaces.get(workspace_index).is_none() {
                    workspace_index = workspace_index.saturating_add(1);
                    continue;
                }
                self.push_workspace_sidebar_lines(
                    &mut lines,
                    &mut selected_line,
                    theme,
                    workspace_index,
                );
                workspace_index = workspace_index.saturating_add(1);
            }
        }

        (lines, selected_line)
    }

    pub(super) fn sidebar_workspace_row_map(&self) -> Vec<Option<usize>> {
        let (lines, _) = self.build_sidebar_lines(self.active_ui_theme());
        lines
            .iter()
            .map(SidebarListLine::workspace_index)
            .collect::<Vec<Option<usize>>>()
    }
}
