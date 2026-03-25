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
        theme: ftui::ResolvedTheme,
    ) -> Style {
        match status {
            crate::domain::PullRequestStatus::Open => secondary_style.fg(packed(theme.info)).bold(),
            crate::domain::PullRequestStatus::Merged => secondary_style.fg(packed(theme.secondary)).bold(),
            crate::domain::PullRequestStatus::Closed => secondary_style.fg(packed(theme.error)).bold(),
        }
    }

    fn selected_sidebar_target(&self) -> SidebarSelectable {
        self.selected_attention_item
            .map(SidebarSelectable::Attention)
            .unwrap_or(SidebarSelectable::Workspace(self.state.selected_index))
    }

    fn attention_row_label(&self, item: &AttentionItem) -> String {
        let workspace_label = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == item.workspace_path)
            .map(Self::workspace_display_name)
            .unwrap_or_else(|| item.task_slug.clone());
        let age_secs_u64 = item.first_seen_at_ms / 1000;
        let age_secs = i64::try_from(age_secs_u64).unwrap_or(i64::MAX);
        let age_label = self.relative_age_label(Some(age_secs));
        format!("! {} · {} · {}", item.summary, workspace_label, age_label)
    }

    fn push_attention_sidebar_lines(
        &self,
        lines: &mut Vec<SidebarListLine>,
        selected_line: &mut Option<usize>,
        theme: ftui::ResolvedTheme,
    ) {
        lines.push(SidebarListLine::attention_header(vec![SidebarSegment {
            text: format!("Needs You [{}]", self.attention_items.len()),
            style: Style::new().fg(packed(theme.warning)).bold(),
        }]));
        if self.attention_items.is_empty() {
            lines.push(SidebarListLine::attention_placeholder(
                vec![SidebarSegment {
                    text: "  nothing needs your attention".to_string(),
                    style: Style::new().fg(packed(theme.border)),
                }],
                Style::new().fg(packed(theme.selection_bg)),
                Style::new(),
            ));
            lines.push(SidebarListLine::project(Vec::new()));
            return;
        }
        for (item_index, item) in self.attention_items.iter().enumerate() {
            let is_selected = self.selected_sidebar_target() == SidebarSelectable::Attention(item_index);
            if is_selected && selected_line.is_none() {
                *selected_line = Some(lines.len());
            }
            let row_background = if is_selected {
                if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                    Some(packed(theme.selection_bg))
                } else {
                    Some(packed(theme.surface))
                }
            } else {
                None
            };
            let mut border_style = if is_selected {
                Style::new().fg(packed(theme.warning)).bold()
            } else {
                Style::new().fg(packed(theme.selection_bg))
            };
            if let Some(background) = row_background {
                border_style = border_style.bg(background);
            }
            let row_style = row_background.map_or_else(Style::new, |background| Style::new().bg(background));
            let mut label_style = Style::new().fg(packed(theme.text));
            if let Some(background) = row_background {
                label_style = label_style.bg(background);
            }
            if is_selected {
                label_style = label_style.bold();
            }
            lines.push(SidebarListLine::attention_item(
                vec![SidebarSegment {
                    text: format!("  {}", self.attention_row_label(item)),
                    style: label_style,
                }],
                item_index,
                border_style,
                row_style,
            ));
        }
        lines.push(SidebarListLine::project(Vec::new()));
    }

    fn push_workspace_sidebar_lines(
        &self,
        lines: &mut Vec<SidebarListLine>,
        selected_line: &mut Option<usize>,
        theme: ftui::ResolvedTheme,
        workspace_index: usize,
    ) {
        let Some(workspace) = self.state.workspaces.get(workspace_index) else {
            return;
        };

        let is_selected = self.selected_sidebar_target() == SidebarSelectable::Workspace(workspace_index);
        if is_selected && selected_line.is_none() {
            *selected_line = Some(lines.len());
        }

        let is_working = self.status_is_visually_working(
            Some(workspace.path.as_path()),
            is_selected,
        );
        let (attention_symbol, attention_color) = self
            .workspace_attention_indicator(workspace.path.as_path())
            .unwrap_or((" ", packed(theme.border)));
        let row_background = if is_selected {
            if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                Some(packed(theme.selection_bg))
            } else {
                Some(packed(theme.surface))
            }
        } else {
            None
        };

        let mut border_style = if is_selected {
            Style::new().fg(packed(theme.primary))
        } else {
            Style::new().fg(packed(theme.selection_bg))
        };
        if let Some(bg) = row_background {
            border_style = border_style.bg(bg);
        }
        if is_selected {
            border_style = border_style.bold();
        }

        let row_style = row_background.map_or_else(Style::new, |bg| Style::new().bg(bg));
        let mut primary_style = Style::new().fg(packed(theme.text));
        let mut secondary_style = Style::new().fg(packed(theme.text_subtle));
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

        let row_prefix = "   ";
        let row_attention_gap = " ";
        let mut leading_segments = vec![
            SidebarSegment {
                text: row_prefix.to_string(),
                style: primary_style,
            },
            SidebarSegment {
                text: attention_symbol.to_string(),
                style: primary_style.fg(attention_color).bold(),
            },
            SidebarSegment {
                text: row_attention_gap.to_string(),
                style: primary_style,
            },
            SidebarSegment {
                text: workspace_label.clone(),
                style: workspace_label_style,
            },
        ];
        if !branch_text.is_empty() {
            leading_segments.push(SidebarSegment {
                text: branch_text,
                style: secondary_style,
            });
        }

        let mut trailing_segments = Vec::new();
        let mut trailing_width = 0usize;
        let mut pr_hits = Vec::new();
        let needs_attention = self.workspace_attention(workspace.path.as_path()).is_some();
        if needs_attention {
            trailing_segments.push(SidebarSegment {
                text: "WAITING".to_string(),
                style: secondary_style.fg(packed(theme.warning)).bold(),
            });
        } else if is_working {
            trailing_segments.push(SidebarSegment {
                text: "WORKING".to_string(),
                style: secondary_style.fg(self.workspace_agent_color(workspace.agent)).bold(),
            });
        } else if self.dialogs.delete_requested_workspaces.contains(&workspace.path) {
            trailing_segments.push(SidebarSegment {
                text: "Deleting...".to_string(),
                style: secondary_style.fg(packed(theme.accent)).bold(),
            });
        } else if workspace.is_orphaned {
            trailing_segments.push(SidebarSegment {
                text: "session ended".to_string(),
                style: secondary_style.fg(packed(theme.accent)),
            });
        } else if !workspace.is_main && !workspace.pull_requests.is_empty() {
            for (pull_request_index, pull_request) in workspace.pull_requests.iter().enumerate() {
                if pull_request_index > 0 {
                    trailing_segments.push(SidebarSegment {
                        text: " ".to_string(),
                        style: secondary_style,
                    });
                    trailing_width = trailing_width.saturating_add(1);
                }
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
                        start_col: trailing_width,
                        width: token_width,
                        data: hit_data,
                    });
                }
                trailing_segments.push(SidebarSegment {
                    text: pull_request_label,
                    style: Self::pull_request_status_style(
                        pull_request.status,
                        secondary_style,
                        theme,
                    )
                    .underline(),
                });
                trailing_width = trailing_width.saturating_add(token_width);
            }
        }

        lines.push(SidebarListLine::workspace(
            leading_segments,
            trailing_segments,
            workspace_index,
            border_style,
            row_style,
            pr_hits,
        ));
    }

    fn build_sidebar_lines(&self, theme: ftui::ResolvedTheme) -> (Vec<SidebarListLine>, Option<usize>) {
        let mut lines = Vec::new();
        let mut selected_line = None;
        let mut workspace_index = 0usize;
        self.push_attention_sidebar_lines(&mut lines, &mut selected_line, theme);

        for (task_index, task) in self.state.tasks.iter().enumerate() {
            if task_index > 0 && !lines.is_empty() {
                lines.push(SidebarListLine::project(Vec::new()));
            }

            lines.push(SidebarListLine::project(vec![SidebarSegment {
                text: format!(
                    "{} {} [{}]",
                    self.task_header_marker(task),
                    task.name,
                    task.worktrees.len()
                ),
                style: Style::new().fg(packed(theme.border)).bold(),
            }]));

            if task.worktrees.is_empty() {
                lines.push(SidebarListLine::project(vec![SidebarSegment {
                    text: "  (no worktrees)".to_string(),
                    style: Style::new().fg(packed(theme.text_subtle)),
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

    pub(super) fn sidebar_selectable_row_map(&self) -> Vec<Option<SidebarSelectable>> {
        let (lines, _) = self.build_sidebar_lines(self.active_ui_theme());
        lines.iter().map(SidebarListLine::selectable).collect()
    }
}
