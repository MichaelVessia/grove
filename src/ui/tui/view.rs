use super::*;

impl GroveApp {
    pub(super) fn render_model(&self, frame: &mut Frame) {
        let view_started_at = Instant::now();
        frame.set_cursor(None);
        frame.set_cursor_visible(false);
        frame.enable_hit_testing();
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let layout = Self::view_layout_for_size(
            frame.buffer.width(),
            frame.buffer.height(),
            self.sidebar_width_pct,
            self.sidebar_hidden,
        );

        let t0 = Instant::now();
        self.render_header(frame, layout.header);
        let t1 = Instant::now();
        self.render_sidebar(frame, layout.sidebar);
        let t2 = Instant::now();
        self.render_divider(frame, layout.divider);
        self.render_preview_pane(frame, layout.preview);
        let t3 = Instant::now();
        self.render_status_line(frame, layout.status);
        self.render_create_dialog_overlay(frame, area);
        self.render_edit_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        self.render_stop_dialog_overlay(frame, area);
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
        self.event_log.log(
            LogEvent::new("frame", "timing")
                .with_data(
                    "header_ms",
                    Value::from(Self::duration_millis(t1.saturating_duration_since(t0))),
                )
                .with_data(
                    "sidebar_ms",
                    Value::from(Self::duration_millis(t2.saturating_duration_since(t1))),
                )
                .with_data(
                    "preview_ms",
                    Value::from(Self::duration_millis(t3.saturating_duration_since(t2))),
                )
                .with_data(
                    "rest_ms",
                    Value::from(Self::duration_millis(
                        draw_completed_at.saturating_duration_since(t3),
                    )),
                )
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
