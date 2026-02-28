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
