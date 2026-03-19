use std::collections::VecDeque;
use std::time::Duration;

use super::*;
use crate::application::agent_runtime::{poll_interval, session_name_for_workspace_ref};
use crate::domain::Workspace;
use crate::domain::WorkspaceStatus;
use crate::infrastructure::process_metrics::ProcessMetricsSnapshot;

#[derive(Debug, Clone)]
pub(super) struct DurationWindow {
    limit: usize,
    intervals: VecDeque<Duration>,
}

impl DurationWindow {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            intervals: VecDeque::new(),
        }
    }

    pub(super) fn push(&mut self, interval: Duration) {
        if self.intervals.len() == self.limit {
            let _ = self.intervals.pop_front();
        }
        self.intervals.push_back(interval);
    }

    pub(super) fn summary(&self) -> Option<DurationSummary> {
        if self.intervals.is_empty() {
            return None;
        }

        let mut millis = self
            .intervals
            .iter()
            .map(|duration| duration.as_secs_f64() * 1000.0)
            .collect::<Vec<f64>>();
        millis.sort_by(|left, right| left.total_cmp(right));

        let sum = millis.iter().sum::<f64>();
        let average_ms = sum / millis.len() as f64;
        let p95_index = millis
            .len()
            .saturating_mul(95)
            .div_ceil(100)
            .saturating_sub(1)
            .min(millis.len().saturating_sub(1));
        let p95_ms = millis[p95_index];

        Some(DurationSummary { average_ms, p95_ms })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DurationSummary {
    pub(super) average_ms: f64,
    pub(super) p95_ms: f64,
}

impl DurationSummary {
    pub(super) fn per_second(self) -> f64 {
        if self.average_ms > 0.0 {
            1000.0 / self.average_ms
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SessionPollContext<'a> {
    pub(super) selected: bool,
    pub(super) preview_stream_selected: bool,
    pub(super) live_preview_selected: bool,
    pub(super) polled_in_background: bool,
    pub(super) output_changing: bool,
    pub(super) waiting_prompt: Option<&'a str>,
}

pub(super) fn session_poll_reason(context: SessionPollContext<'_>) -> String {
    if context.selected && context.preview_stream_selected {
        return "selected preview stream".to_string();
    }
    if context.selected && context.live_preview_selected {
        return "selected live preview, excluded from background polling".to_string();
    }
    if context.selected && context.output_changing {
        return "selected workspace, output changing".to_string();
    }
    if context.selected {
        return "selected workspace, preview cadence".to_string();
    }
    if context.waiting_prompt.is_some() {
        return "background status poll, waiting prompt detected".to_string();
    }
    if context.output_changing {
        return "background status poll, output changing".to_string();
    }
    if context.polled_in_background {
        return "background status poll".to_string();
    }

    "not polled".to_string()
}

pub(super) fn scheduler_reason(source: Option<&str>, trigger: Option<&str>) -> String {
    match (source, trigger) {
        (Some(source), Some(trigger)) => format!("source {source}, trigger {trigger}"),
        (Some(source), None) => format!("source {source}"),
        (None, Some(trigger)) => format!("trigger {trigger}"),
        (None, None) => "scheduler warming up".to_string(),
    }
}

pub(super) fn format_duration(duration: Option<Duration>) -> String {
    duration
        .map(|value| format!("{} ms", value.as_millis()))
        .unwrap_or_else(|| "unavailable".to_string())
}

pub(super) fn workspace_status_label(status: WorkspaceStatus) -> &'static str {
    match status {
        WorkspaceStatus::Main => "main",
        WorkspaceStatus::Idle => "idle",
        WorkspaceStatus::Active => "active",
        WorkspaceStatus::Thinking => "thinking",
        WorkspaceStatus::Waiting => "waiting",
        WorkspaceStatus::Done => "done",
        WorkspaceStatus::Error => "error",
        WorkspaceStatus::Unknown => "unknown",
        WorkspaceStatus::Unsupported => "unsupported",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionPerformanceRow {
    pub(super) label: String,
    pub(super) status: &'static str,
    pub(super) cadence: String,
    pub(super) role: &'static str,
    pub(super) reason: String,
}

impl GroveApp {
    pub(super) fn record_redraw_cadence(&self, started_at: Instant) {
        let previous = self
            .performance
            .last_redraw_started_at
            .replace(Some(started_at));
        let Some(previous_started_at) = previous else {
            return;
        };

        self.performance
            .redraw_timing
            .borrow_mut()
            .push(started_at.saturating_duration_since(previous_started_at));
    }

    pub(super) fn record_render_cost(&self, draw_duration: Duration, view_duration: Duration) {
        self.performance
            .draw_timing
            .borrow_mut()
            .push(draw_duration);
        self.performance
            .view_timing
            .borrow_mut()
            .push(view_duration);
    }

    pub(super) fn refresh_process_metrics(&self, now: Instant) {
        let refresh_interval = Duration::from_secs(1);
        if self
            .performance
            .last_process_refresh_at
            .borrow()
            .is_some_and(|last| now.saturating_duration_since(last) < refresh_interval)
        {
            return;
        }

        let snapshot = self.performance.process_sampler.borrow_mut().refresh();
        self.performance.process_metrics.replace(snapshot);
        self.performance.last_process_refresh_at.replace(Some(now));
    }

    pub(super) fn redraw_timing_summary(&self) -> Option<DurationSummary> {
        self.performance.redraw_timing.borrow().summary()
    }

    pub(super) fn draw_timing_summary(&self) -> Option<DurationSummary> {
        self.performance.draw_timing.borrow().summary()
    }

    pub(super) fn view_timing_summary(&self) -> Option<DurationSummary> {
        self.performance.view_timing.borrow().summary()
    }

    pub(super) fn process_metrics_snapshot(&self) -> ProcessMetricsSnapshot {
        self.performance.process_metrics.borrow().clone()
    }

    pub(super) fn scheduler_reason_summary(&self) -> String {
        scheduler_reason(
            self.polling.next_tick_source.as_deref(),
            self.polling.next_tick_trigger.as_deref(),
        )
    }

    pub(super) fn selected_preview_source_summary(&self) -> &'static str {
        let Some(session_name) = self.polling.preview_stream.target_session.as_deref() else {
            return "inactive";
        };
        if self.preview_stream_is_healthy_for_session(session_name) {
            return "stream";
        }
        match self.polling.preview_stream.source {
            PreviewStreamSource::Connecting => "connecting",
            PreviewStreamSource::Stream => "stream",
            PreviewStreamSource::Disconnected => "disconnected",
        }
    }

    fn session_poll_interval(&self, workspace: &Workspace, is_selected: bool) -> Duration {
        let since_last_key = self
            .session
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            workspace.status,
            is_selected,
            is_selected && self.state.focus == PaneFocus::Preview,
            is_selected && self.session.interactive.is_some(),
            since_last_key,
            if is_selected {
                self.polling.output_changing
            } else {
                self.polling
                    .workspace_output_changing
                    .get(workspace.path.as_path())
                    .copied()
                    .unwrap_or(false)
            },
        )
    }

    pub(super) fn session_performance_rows(&self) -> Vec<SessionPerformanceRow> {
        let selected_path = self
            .state
            .selected_workspace()
            .map(|workspace| &workspace.path);
        let selected_live_preview = self.polling.last_live_preview_session.as_deref();
        let mut rows = Vec::new();

        for workspace in &self.state.workspaces {
            let is_selected = selected_path == Some(&workspace.path);
            let session_name = session_name_for_workspace_ref(workspace);
            let preview_stream_selected = is_selected
                && self.polling.preview_stream.target_session.as_deref()
                    == Some(session_name.as_str());
            let preview_stream_healthy = preview_stream_selected
                && self.preview_stream_is_healthy_for_session(session_name.as_str());
            let live_preview_selected = !is_selected
                && !preview_stream_healthy
                && selected_live_preview == Some(session_name.as_str());
            let polled_in_background = workspace.supported_agent && workspace.status.has_session();
            let waiting_prompt = self
                .polling
                .workspace_waiting_prompts
                .get(workspace.path.as_path())
                .map(String::as_str);
            let output_changing = if is_selected {
                self.polling.output_changing
            } else {
                self.polling
                    .workspace_output_changing
                    .get(workspace.path.as_path())
                    .copied()
                    .unwrap_or(false)
            };
            let reason = session_poll_reason(SessionPollContext {
                selected: is_selected,
                preview_stream_selected,
                live_preview_selected,
                polled_in_background,
                output_changing,
                waiting_prompt,
            });
            let cadence = if preview_stream_selected {
                self.selected_preview_source_summary().to_string()
            } else if live_preview_selected {
                "excluded".to_string()
            } else if is_selected || polled_in_background {
                format_duration(Some(self.session_poll_interval(workspace, is_selected)))
            } else {
                "n/a".to_string()
            };
            let role = if is_selected {
                "selected"
            } else {
                "background"
            };

            rows.push(SessionPerformanceRow {
                label: workspace.name.clone(),
                status: workspace_status_label(workspace.status),
                cadence,
                role,
                reason,
            });
        }

        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_window_computes_recent_average_and_p95() {
        let mut window = DurationWindow::new(8);
        for ms in [16_u64, 17, 16, 18, 16] {
            window.push(Duration::from_millis(ms));
        }

        let summary = window.summary().expect("summary");
        assert_eq!(summary.p95_ms, 18.0);
    }

    #[test]
    fn session_poll_reason_describes_selected_live_preview_exclusion() {
        let reason = session_poll_reason(SessionPollContext {
            selected: true,
            preview_stream_selected: false,
            live_preview_selected: true,
            polled_in_background: false,
            output_changing: false,
            waiting_prompt: None,
        });

        assert!(reason.contains("selected"));
        assert!(reason.contains("excluded"));
    }
}
