use ftui::layout::{Constraint, Flex};
use ftui::widgets::StatefulWidget;
use ftui::widgets::table::{Row, Table, TableState};

use super::view_prelude::*;
use crate::ui::tui::performance::{SessionPerformanceRow, format_duration};
use std::time::Instant;

#[derive(Debug, Clone)]
struct PerformanceModalContent {
    summary: FtText<'static>,
    scheduler: FtText<'static>,
    sessions: Vec<SessionPerformanceRow>,
    theme: ftui::ResolvedTheme,
}

impl Widget for PerformanceModalContent {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new()
            .bg(packed(self.theme.background))
            .fg(packed(self.theme.text));
        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title("Performance")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(packed(self.theme.accent)).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let sections = Flex::vertical()
            .constraints([
                Constraint::Fixed(8),
                Constraint::Min(8),
                Constraint::Fixed(1),
            ])
            .split(inner);
        let top = Flex::horizontal()
            .constraints([Constraint::Percentage(52.0), Constraint::Percentage(48.0)])
            .split(sections[0]);

        self.render_text_panel(frame, top[0], "Summary", self.summary.clone());
        self.render_text_panel(frame, top[1], "Scheduler", self.scheduler.clone());
        self.render_sessions_table(frame, sections[1]);
        Paragraph::new("Close: Esc")
            .style(
                Style::new()
                    .fg(packed(self.theme.border))
                    .bg(packed(self.theme.background)),
            )
            .render(sections[2], frame);
    }
}

impl PerformanceModalContent {
    fn render_text_panel(&self, frame: &mut Frame, area: Rect, title: &str, body: FtText<'static>) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title(title)
            .borders(Borders::ALL)
            .style(
                Style::new()
                    .bg(packed(self.theme.background))
                    .fg(packed(self.theme.text)),
            )
            .border_style(Style::new().fg(packed(self.theme.border)));
        let inner = block.inner(area);
        block.render(area, frame);
        if inner.is_empty() {
            return;
        }

        Paragraph::new(body)
            .style(
                Style::new()
                    .bg(packed(self.theme.background))
                    .fg(packed(self.theme.text)),
            )
            .render(inner, frame);
    }

    fn render_sessions_table(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Sessions")
            .borders(Borders::ALL)
            .style(
                Style::new()
                    .bg(packed(self.theme.background))
                    .fg(packed(self.theme.text)),
            )
            .border_style(Style::new().fg(packed(self.theme.border)));
        let inner = block.inner(area);
        block.render(area, frame);
        if inner.is_empty() {
            return;
        }

        if self.sessions.is_empty() {
            Paragraph::new("No known workspaces")
                .style(
                    Style::new()
                        .bg(packed(self.theme.background))
                        .fg(packed(self.theme.border)),
                )
                .render(inner, frame);
            return;
        }

        let header = Row::new(["Workspace", "Status", "Cadence", "Role", "Reason"])
            .style(Style::new().fg(packed(self.theme.primary)).bold());
        let rows = self.sessions.iter().map(|row| {
            Row::new([
                row.label.clone(),
                row.status.to_string(),
                row.cadence.clone(),
                row.role.to_string(),
                row.reason.clone(),
            ])
        });
        let widths = [
            Constraint::Percentage(20.0),
            Constraint::Fixed(10),
            Constraint::Fixed(10),
            Constraint::Fixed(10),
            Constraint::Min(20),
        ];
        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .style(
                Style::new()
                    .bg(packed(self.theme.background))
                    .fg(packed(self.theme.text)),
            );
        let mut state = TableState::default();
        StatefulWidget::render(&table, inner, frame, &mut state);
    }
}

impl GroveApp {
    pub(super) fn render_performance_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        if self.performance_dialog().is_none() {
            return;
        }
        if area.width < 72 || area.height < 22 {
            return;
        }

        let dialog_width = area.width.saturating_sub(6).min(160);
        let dialog_height = area.height.saturating_sub(4).clamp(22, 44);
        let theme = self.active_ui_theme();
        let now = Instant::now();
        let redraw_summary = self.redraw_timing_summary();
        let draw_summary = self.draw_timing_summary();
        let view_summary = self.view_timing_summary();
        let process_metrics = self.process_metrics_snapshot();
        let next_tick = self
            .polling
            .next_tick_due_at
            .map(|due_at| due_at.saturating_duration_since(now));
        let next_poll = self
            .polling
            .next_poll_due_at
            .map(|due_at| due_at.saturating_duration_since(now));
        let next_visual = self
            .polling
            .next_visual_due_at
            .map(|due_at| due_at.saturating_duration_since(now));

        let summary = FtText::from_lines(vec![
            FtLine::raw("Runtime inspection for Grove"),
            FtLine::raw(format!("CPU      {}", process_metrics.cpu_display())),
            FtLine::raw(format!("Memory   {}", process_metrics.memory_display())),
            FtLine::raw(format!(
                "Redraw   {}",
                redraw_summary
                    .map(|summary| {
                        format!(
                            "{:.1}/sec, avg {:.1} ms, p95 {:.1} ms",
                            summary.per_second(),
                            summary.average_ms,
                            summary.p95_ms
                        )
                    })
                    .unwrap_or_else(|| "warming up".to_string())
            )),
            FtLine::raw(format!(
                "Draw     {}",
                draw_summary
                    .map(|summary| format!(
                        "avg {:.1} ms, p95 {:.1} ms",
                        summary.average_ms, summary.p95_ms
                    ))
                    .unwrap_or_else(|| "warming up".to_string())
            )),
            FtLine::raw(format!(
                "View     {}",
                view_summary
                    .map(|summary| format!(
                        "avg {:.1} ms, p95 {:.1} ms",
                        summary.average_ms, summary.p95_ms
                    ))
                    .unwrap_or_else(|| "warming up".to_string())
            )),
        ]);
        let scheduler = FtText::from_lines(vec![
            FtLine::raw(self.scheduler_reason_summary()),
            FtLine::raw(format!(
                "PreviewSource  {}",
                self.selected_preview_source_summary()
            )),
            FtLine::raw(format!("NextTick    {}", format_duration(next_tick))),
            FtLine::raw(format!("NextPoll    {}", format_duration(next_poll))),
            FtLine::raw(format!("NextVisual  {}", format_duration(next_visual))),
        ]);
        let content = PerformanceModalContent {
            summary,
            scheduler,
            sessions: self.session_performance_rows(),
            theme,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(packed(theme.background), 0.55))
            .hit_id(HitId::new(HIT_ID_PERFORMANCE_DIALOG))
            .render(area, frame);
    }
}
