use std::time::Instant;

use ftui::Style;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use serde_json::Value;

use crate::infrastructure::event_log::Event as LogEvent;

use super::panes::PaneRole;
use super::{DIVIDER_WIDTH, GroveApp};

impl GroveApp {
    pub(super) fn render_model(&self, frame: &mut Frame) {
        let view_started_at = Instant::now();
        frame.set_cursor(None);
        frame.set_cursor_visible(false);
        frame.enable_hit_testing();
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let theme = self.active_ui_theme();
        Paragraph::new("")
            .style(Style::new().bg(theme.base))
            .render(area, frame);

        let Some(pane_layout) = self.panes.solve(area) else {
            // Viewport too small to solve constraints. Skip pane rendering but
            // still update hit grid so the rest of the system stays consistent.
            self.last_hit_grid.replace(frame.hit_grid.clone());
            return;
        };
        let header_rect = self
            .panes
            .rect_for_role(&pane_layout, PaneRole::Header)
            .unwrap_or_default();
        let workspace_list_rect = self
            .panes
            .rect_for_role(&pane_layout, PaneRole::WorkspaceList)
            .unwrap_or_default();
        let preview_rect = self
            .panes
            .rect_for_role(&pane_layout, PaneRole::Preview)
            .unwrap_or_default();
        let status_rect = self
            .panes
            .rect_for_role(&pane_layout, PaneRole::Status)
            .unwrap_or_default();

        // Carve divider from the preview pane's left edge at the workspace_list/preview boundary.
        // When sidebar is hidden, collapse workspace_list to zero and give space to preview.
        let (sidebar_rect, divider_rect, preview_rect) = if self.sidebar_hidden {
            let full_preview = Rect::new(
                workspace_list_rect.x,
                workspace_list_rect.y,
                workspace_list_rect.width + preview_rect.width,
                preview_rect.height,
            );
            (Rect::default(), Rect::default(), full_preview)
        } else if preview_rect.width > DIVIDER_WIDTH {
            let divider = Rect::new(
                preview_rect.x,
                preview_rect.y,
                DIVIDER_WIDTH,
                preview_rect.height,
            );
            let adjusted_preview = Rect::new(
                preview_rect.x + DIVIDER_WIDTH,
                preview_rect.y,
                preview_rect.width - DIVIDER_WIDTH,
                preview_rect.height,
            );
            (workspace_list_rect, divider, adjusted_preview)
        } else {
            (workspace_list_rect, Rect::default(), preview_rect)
        };

        self.render_header(frame, header_rect);
        self.render_sidebar(frame, sidebar_rect);
        self.render_divider(frame, divider_rect);
        self.render_preview_pane(frame, preview_rect);
        self.render_status_line(frame, status_rect);
        self.render_create_dialog_overlay(frame, area);
        self.render_edit_dialog_overlay(frame, area);
        self.render_rename_tab_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        self.render_stop_dialog_overlay(frame, area);
        self.render_confirm_dialog_overlay(frame, area);
        self.render_session_cleanup_dialog_overlay(frame, area);
        self.render_delete_dialog_overlay(frame, area);
        self.render_merge_dialog_overlay(frame, area);
        self.render_update_from_base_dialog_overlay(frame, area);
        self.render_settings_dialog_overlay(frame, area);
        self.render_project_dialog_overlay(frame, area);
        self.render_keybind_help_overlay(frame, area);
        self.render_command_palette_overlay(frame, area);
        self.render_toasts(frame, area);
        let draw_completed_at = Instant::now();
        self.last_hit_grid.replace(frame.hit_grid.clone());
        let frame_log_started_at = Instant::now();
        self.log_frame_render(frame);
        let view_completed_at = Instant::now();
        self.telemetry.event_log.log(
            LogEvent::new("frame", "timing")
                .with_data(
                    "draw_ms",
                    Value::from(Self::duration_millis(
                        draw_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data(
                    "frame_log_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(frame_log_started_at),
                    )),
                )
                .with_data(
                    "view_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data("degradation", Value::from(frame.degradation.as_str()))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
    }
}
