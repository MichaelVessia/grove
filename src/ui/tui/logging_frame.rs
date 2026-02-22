use super::*;

impl GroveApp {
    fn frame_lines_hash(lines: &[String]) -> u64 {
        let mut hasher = DefaultHasher::new();
        lines.hash(&mut hasher);
        hasher.finish()
    }

    fn frame_buffer_lines(frame: &mut Frame) -> Vec<String> {
        let height = frame.buffer.height();
        let mut lines = Vec::with_capacity(usize::from(height));
        for y in 0..height {
            let mut row = String::with_capacity(usize::from(frame.buffer.width()));
            for x in 0..frame.buffer.width() {
                let Some(cell) = frame.buffer.get(x, y).copied() else {
                    continue;
                };
                if cell.is_continuation() {
                    continue;
                }
                if let Some(value) = cell.content.as_char() {
                    row.push(value);
                    continue;
                }
                if let Some(grapheme_id) = cell.content.grapheme_id()
                    && let Some(grapheme) = frame.pool.get(grapheme_id)
                {
                    row.push_str(grapheme);
                    continue;
                }
                row.push(' ');
            }
            lines.push(row.trim_end_matches(' ').to_string());
        }

        lines
    }

    pub(super) fn log_frame_render(&self, frame: &mut Frame) {
        let Some(app_start_ts) = self.debug_record_start_ts else {
            return;
        };

        let lines = Self::frame_buffer_lines(frame);
        let frame_hash = Self::frame_lines_hash(&lines);
        let previous_hash = {
            let mut hash_ref = self.last_frame_hash.borrow_mut();
            let previous = *hash_ref;
            *hash_ref = frame_hash;
            previous
        };
        let frame_changed = frame_hash != previous_hash;
        let non_empty_line_count = lines.iter().filter(|line| !line.is_empty()).count();
        let mut seq = self.frame_render_seq.borrow_mut();
        *seq = seq.saturating_add(1);
        let seq_value = *seq;
        drop(seq);

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default();
        let interactive_session = self
            .interactive
            .as_ref()
            .map(|state| state.target_session.clone())
            .unwrap_or_default();
        let pending_input_depth = self.pending_input_depth();
        let oldest_pending_input_seq = self
            .pending_interactive_inputs
            .front()
            .map(|trace| trace.seq)
            .unwrap_or(0);
        let oldest_pending_input_age_ms = self.oldest_pending_input_age_ms(Instant::now());

        let mut frame_fields = vec![
            ("seq".to_string(), Value::from(seq_value)),
            ("app_start_ts".to_string(), Value::from(app_start_ts)),
            ("width".to_string(), Value::from(frame.buffer.width())),
            ("height".to_string(), Value::from(frame.buffer.height())),
            (
                "line_count".to_string(),
                Value::from(usize_to_u64(lines.len())),
            ),
            (
                "non_empty_line_count".to_string(),
                Value::from(usize_to_u64(non_empty_line_count)),
            ),
            ("frame_hash".to_string(), Value::from(frame_hash)),
            (
                "degradation".to_string(),
                Value::from(frame.degradation.as_str()),
            ),
            ("mode".to_string(), Value::from(self.mode_label())),
            ("focus".to_string(), Value::from(self.focus_label())),
            (
                "selected_workspace".to_string(),
                Value::from(selected_workspace),
            ),
            (
                "interactive_session".to_string(),
                Value::from(interactive_session),
            ),
            (
                "sidebar_width_pct".to_string(),
                Value::from(self.sidebar_width_pct),
            ),
            (
                "preview_offset".to_string(),
                Value::from(usize_to_u64(self.preview.offset)),
            ),
            (
                "preview_auto_scroll".to_string(),
                Value::from(self.preview.auto_scroll),
            ),
            (
                "output_changing".to_string(),
                Value::from(self.output_changing),
            ),
            (
                "pending_input_depth".to_string(),
                Value::from(pending_input_depth),
            ),
            (
                "oldest_pending_input_seq".to_string(),
                Value::from(oldest_pending_input_seq),
            ),
            (
                "oldest_pending_input_age_ms".to_string(),
                Value::from(oldest_pending_input_age_ms),
            ),
            (
                "frame_cursor_visible".to_string(),
                Value::from(frame.cursor_visible),
            ),
            (
                "frame_cursor_has_position".to_string(),
                Value::from(frame.cursor_position.is_some()),
            ),
        ];
        if let Some((cursor_col, cursor_row)) = frame.cursor_position {
            frame_fields.push(("frame_cursor_col".to_string(), Value::from(cursor_col)));
            frame_fields.push(("frame_cursor_row".to_string(), Value::from(cursor_row)));
        }
        if let Some(interactive) = self.interactive.as_ref() {
            frame_fields.push((
                "interactive_cursor_visible".to_string(),
                Value::from(interactive.cursor_visible),
            ));
            frame_fields.push((
                "interactive_cursor_row".to_string(),
                Value::from(interactive.cursor_row),
            ));
            frame_fields.push((
                "interactive_cursor_col".to_string(),
                Value::from(interactive.cursor_col),
            ));
            frame_fields.push((
                "interactive_pane_width".to_string(),
                Value::from(interactive.pane_width),
            ));
            frame_fields.push((
                "interactive_pane_height".to_string(),
                Value::from(interactive.pane_height),
            ));

            let layout = Self::view_layout_for_size(
                frame.buffer.width(),
                frame.buffer.height(),
                self.sidebar_width_pct,
                false,
            );
            let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
            let preview_height = usize::from(
                preview_inner
                    .height
                    .saturating_sub(PREVIEW_METADATA_ROWS)
                    .max(1),
            );
            let cursor_target = self.interactive_cursor_target(preview_height);
            frame_fields.push((
                "interactive_cursor_in_viewport".to_string(),
                Value::from(cursor_target.is_some()),
            ));
            if let Some((visible_index, target_col, target_visible)) = cursor_target {
                frame_fields.push((
                    "interactive_cursor_visible_index".to_string(),
                    Value::from(usize_to_u64(visible_index)),
                ));
                frame_fields.push((
                    "interactive_cursor_target_col".to_string(),
                    Value::from(usize_to_u64(target_col)),
                ));
                frame_fields.push((
                    "interactive_cursor_target_visible".to_string(),
                    Value::from(target_visible),
                ));
            }
        }
        frame_fields.push(("frame_changed".to_string(), Value::from(frame_changed)));
        if frame_changed {
            frame_fields.push((
                "frame_lines".to_string(),
                Value::Array(lines.into_iter().map(Value::from).collect()),
            ));
        }
        self.log_event_with_fields("frame", "rendered", frame_fields);
    }
}
