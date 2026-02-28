use super::*;

impl GroveApp {
    pub(super) fn add_selection_point_snapshot_fields(
        &self,
        mut event: LogEvent,
        key_prefix: &str,
        point: TextSelectionPoint,
    ) -> LogEvent {
        let raw_line = self.preview.lines.get(point.line).cloned();
        let clean_line = self.preview_plain_line(point.line);
        let render_line = self.preview.render_lines.get(point.line).cloned();

        if let Some(line) = raw_line {
            event = event.with_data(
                format!("{key_prefix}line_raw_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        if let Some(line) = clean_line {
            event = event
                .with_data(
                    format!("{key_prefix}line_clean_preview"),
                    Value::from(truncate_for_log(&line, 120)),
                )
                .with_data(
                    format!("{key_prefix}line_visual_width"),
                    Value::from(usize_to_u64(line_visual_width(&line))),
                )
                .with_data(
                    format!("{key_prefix}line_context"),
                    Value::from(truncate_for_log(
                        &visual_substring(
                            &line,
                            point.col.saturating_sub(16),
                            Some(point.col.saturating_add(16)),
                        ),
                        120,
                    )),
                );

            if let Some((grapheme, start_col, end_col)) = visual_grapheme_at(&line, point.col) {
                event = event
                    .with_data(
                        format!("{key_prefix}grapheme"),
                        Value::from(truncate_for_log(&grapheme, 16)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_start_col"),
                        Value::from(usize_to_u64(start_col)),
                    )
                    .with_data(
                        format!("{key_prefix}grapheme_end_col"),
                        Value::from(usize_to_u64(end_col)),
                    );
            }
        }

        if let Some(line) = render_line {
            event = event.with_data(
                format!("{key_prefix}line_render_preview"),
                Value::from(truncate_for_log(&line, 120)),
            );
        }

        event
    }

    pub(super) fn log_preview_drag_started(
        &self,
        x: u16,
        y: u16,
        point: Option<TextSelectionPoint>,
    ) {
        let mut event = LogEvent::new("selection", "preview_drag_started")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("mode", Value::from(self.state.mode.name()))
            .with_data("focus", Value::from(self.state.focus.name()))
            .with_data(
                "preview_offset",
                Value::from(usize_to_u64(self.preview.offset)),
            );

        if let Some(viewport) = self.preview_content_viewport() {
            event = event
                .with_data("output_x", Value::from(viewport.output_x))
                .with_data("output_y", Value::from(viewport.output_y))
                .with_data(
                    "visible_start",
                    Value::from(usize_to_u64(viewport.visible_start)),
                )
                .with_data(
                    "visible_end",
                    Value::from(usize_to_u64(viewport.visible_end)),
                );
        }

        if let Some(point) = point {
            event = event
                .with_data("line", Value::from(usize_to_u64(point.line)))
                .with_data("col", Value::from(usize_to_u64(point.col)));
            event = self.add_selection_point_snapshot_fields(event, "", point);
            if let Some(line) = self.preview_plain_line(point.line) {
                event = event.with_data("line_preview", Value::from(truncate_for_log(&line, 120)));
            }
            if let Some(render_line) = self.preview.render_lines.get(point.line) {
                event = event.with_data(
                    "render_line_preview",
                    Value::from(truncate_for_log(render_line, 120)),
                );
            }
        }

        self.event_log.log(event);
    }

    pub(super) fn log_preview_drag_finished(
        &self,
        x: u16,
        y: u16,
        point: Option<TextSelectionPoint>,
    ) {
        let mut event = LogEvent::new("selection", "preview_drag_finished")
            .with_data("x", Value::from(x))
            .with_data("y", Value::from(y))
            .with_data("mapped", Value::from(point.is_some()))
            .with_data(
                "has_selection",
                Value::from(self.preview_selection.has_selection()),
            )
            .with_data("interactive", Value::from(self.interactive.is_some()));

        if let Some(point) = point {
            event = event
                .with_data("release_line", Value::from(usize_to_u64(point.line)))
                .with_data("release_col", Value::from(usize_to_u64(point.col)));
            event = self.add_selection_point_snapshot_fields(event, "release_", point);
        }

        if let Some(anchor) = self.preview_selection.anchor {
            event = event
                .with_data("anchor_line", Value::from(usize_to_u64(anchor.line)))
                .with_data("anchor_col", Value::from(usize_to_u64(anchor.col)));
            event = self.add_selection_point_snapshot_fields(event, "anchor_", anchor);
        }

        if let Some(start) = self.preview_selection.start {
            event = event
                .with_data("start_line", Value::from(usize_to_u64(start.line)))
                .with_data("start_col", Value::from(usize_to_u64(start.col)));
            event = self.add_selection_point_snapshot_fields(event, "start_", start);
        }
        if let Some(end) = self.preview_selection.end {
            event = event
                .with_data("end_line", Value::from(usize_to_u64(end.line)))
                .with_data("end_col", Value::from(usize_to_u64(end.col)));
            event = self.add_selection_point_snapshot_fields(event, "end_", end);
        }

        if let Some(lines) = self.selected_preview_text_lines() {
            let text = lines.join("\n");
            event = event
                .with_data(
                    "selected_line_count",
                    Value::from(usize_to_u64(lines.len())),
                )
                .with_data(
                    "selected_char_count",
                    Value::from(usize_to_u64(text.chars().count())),
                )
                .with_data(
                    "selected_preview",
                    Value::from(truncate_for_log(&text, 240)),
                );
        }

        self.event_log.log(event);
    }
}
