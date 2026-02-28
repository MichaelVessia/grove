use super::*;
use ftui::widgets::virtualized::{RenderItem, VirtualizedList};

#[derive(Debug, Clone)]
struct SidebarSegment {
    text: String,
    style: Style,
}

#[derive(Debug, Clone)]
struct SidebarPrHit {
    start_col: usize,
    width: usize,
    data: u64,
}

#[derive(Debug, Clone)]
struct SidebarActivityLabel {
    label: String,
    agent: AgentType,
    start_col: usize,
}

#[derive(Debug, Clone)]
enum SidebarLineKind {
    Project,
    Workspace {
        workspace_index: usize,
        border_style: Style,
        row_style: Style,
        pr_hits: Vec<SidebarPrHit>,
        activity: Option<SidebarActivityLabel>,
    },
}

#[derive(Debug, Clone)]
struct SidebarListLine {
    segments: Vec<SidebarSegment>,
    kind: SidebarLineKind,
}

impl SidebarListLine {
    fn project(segments: Vec<SidebarSegment>) -> Self {
        Self {
            segments,
            kind: SidebarLineKind::Project,
        }
    }

    fn workspace(
        segments: Vec<SidebarSegment>,
        workspace_index: usize,
        border_style: Style,
        row_style: Style,
        pr_hits: Vec<SidebarPrHit>,
        activity: Option<SidebarActivityLabel>,
    ) -> Self {
        Self {
            segments,
            kind: SidebarLineKind::Workspace {
                workspace_index,
                border_style,
                row_style,
                pr_hits,
                activity,
            },
        }
    }

    fn workspace_index(&self) -> Option<usize> {
        match self.kind {
            SidebarLineKind::Project => None,
            SidebarLineKind::Workspace {
                workspace_index, ..
            } => Some(workspace_index),
        }
    }

    fn activity(&self) -> Option<&SidebarActivityLabel> {
        match &self.kind {
            SidebarLineKind::Project => None,
            SidebarLineKind::Workspace { activity, .. } => activity.as_ref(),
        }
    }
}

impl RenderItem for SidebarListLine {
    fn render(&self, area: Rect, frame: &mut Frame, _selected: bool) {
        if area.is_empty() {
            return;
        }

        match &self.kind {
            SidebarLineKind::Project => {
                render_sidebar_segments(self.segments.as_slice(), area, frame);
            }
            SidebarLineKind::Workspace {
                workspace_index,
                border_style,
                row_style,
                pr_hits,
                ..
            } => {
                let fill = " ".repeat(usize::from(area.width));
                Paragraph::new(fill).style(*row_style).render(area, frame);

                let left_border_area = Rect::new(area.x, area.y, 1, 1);
                render_sidebar_segments(
                    &[SidebarSegment {
                        text: "│".to_string(),
                        style: *border_style,
                    }],
                    left_border_area,
                    frame,
                );
                let right_border_x = area.right().saturating_sub(1);
                let right_border_area = Rect::new(right_border_x, area.y, 1, 1);
                render_sidebar_segments(
                    &[SidebarSegment {
                        text: "│".to_string(),
                        style: *border_style,
                    }],
                    right_border_area,
                    frame,
                );

                let content_x = area.x.saturating_add(2);
                let content_width = area.width.saturating_sub(4);
                let content_area = Rect::new(content_x, area.y, content_width, 1);
                if content_width > 0 {
                    render_sidebar_segments(self.segments.as_slice(), content_area, frame);
                }

                if let Ok(data) = u64::try_from(*workspace_index) {
                    let _ = frame.register_hit(
                        area,
                        HitId::new(HIT_ID_WORKSPACE_ROW),
                        FrameHitRegion::Content,
                        data,
                    );
                }

                if content_width > 0 {
                    for pr_hit in pr_hits {
                        let Some(start) = u16::try_from(pr_hit.start_col).ok() else {
                            continue;
                        };
                        let token_x = content_x.saturating_add(start);
                        if token_x >= content_area.right() {
                            continue;
                        }
                        let Some(token_width) = u16::try_from(pr_hit.width).ok() else {
                            continue;
                        };
                        let visible_width =
                            token_width.min(content_area.right().saturating_sub(token_x));
                        if visible_width == 0 {
                            continue;
                        }
                        let _ = frame.register_hit(
                            Rect::new(token_x, area.y, visible_width, 1),
                            HitId::new(HIT_ID_WORKSPACE_PR_LINK),
                            FrameHitRegion::Content,
                            pr_hit.data,
                        );
                    }
                }
            }
        }
    }
}

fn render_sidebar_segments(segments: &[SidebarSegment], area: Rect, frame: &mut Frame) {
    if area.is_empty() {
        return;
    }

    let spans = segments
        .iter()
        .map(|segment| FtSpan::styled(segment.text.clone(), segment.style))
        .collect::<Vec<FtSpan>>();
    Paragraph::new(FtText::from_lines(vec![FtLine::from_spans(spans)])).render(area, frame);
}

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

        if self.projects.is_empty() {
            Paragraph::new(FtText::from_lines(vec![
                FtLine::from_spans(vec![FtSpan::styled(
                    "No projects configured",
                    Style::new().fg(theme.subtext0),
                )]),
                FtLine::raw(""),
                FtLine::from_spans(vec![FtSpan::styled(
                    "Press 'p' to add a project",
                    Style::new().fg(theme.text).bold(),
                )]),
            ]))
            .render(inner, frame);
            return;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                Paragraph::new(FtText::from_lines(vec![
                    FtLine::from_spans(vec![FtSpan::styled(
                        "Discovery error",
                        Style::new().fg(theme.red).bold(),
                    )]),
                    FtLine::from_spans(vec![FtSpan::styled(
                        message.as_str(),
                        Style::new().fg(theme.peach),
                    )]),
                ]))
                .render(inner, frame);
            }
            return;
        }

        let (lines, selected_line) = self.build_sidebar_lines(theme);
        if lines.is_empty() {
            return;
        }

        let mut list_state = self.sidebar_list_state.borrow_mut();
        if selected_line.is_some_and(|line| line <= 1) && inner.height > 1 {
            list_state.scroll_to_top();
        }
        list_state.select(selected_line);
        let list = VirtualizedList::new(lines.as_slice())
            .fixed_height(1)
            .show_scrollbar(true)
            .highlight_style(Style::new());
        ftui::widgets::StatefulWidget::render(&list, inner, frame, &mut *list_state);

        let scroll_offset = list_state.scroll_offset();
        let visible_count = list_state.visible_count();
        drop(list_state);

        let row_width = if lines.len() > visible_count {
            inner.width.saturating_sub(1)
        } else {
            inner.width
        };
        let content_x = inner.x.saturating_add(2);
        let content_width = row_width.saturating_sub(4);
        if content_width == 0 {
            return;
        }
        let max_x = content_x.saturating_add(content_width);
        let visible_end = scroll_offset
            .saturating_add(usize::from(inner.height))
            .min(lines.len());

        for (row_index, line) in lines
            .iter()
            .enumerate()
            .take(visible_end)
            .skip(scroll_offset)
        {
            let Some(activity) = line.activity() else {
                continue;
            };
            let Some(y_offset) = u16::try_from(row_index.saturating_sub(scroll_offset)).ok() else {
                continue;
            };
            let y = inner.y.saturating_add(y_offset);
            if y >= inner.bottom() {
                continue;
            }
            let Some(start_col) = u16::try_from(activity.start_col).ok() else {
                continue;
            };
            let x = content_x.saturating_add(start_col);
            if x >= max_x {
                continue;
            }
            let width = max_x.saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(
                activity.label.as_str(),
                activity.agent,
                Rect::new(x, y, width, 1),
                frame,
            );
        }
    }
}
