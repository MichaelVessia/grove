use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::agent_runtime::{OutputDigest, evaluate_capture_change};

const SCROLL_DEBOUNCE_MS: u64 = 40;
const SCROLL_BURST_DEBOUNCE_MS: u64 = 120;
const CAPTURE_RING_CAPACITY: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureRecord {
    pub ts: u64,
    pub raw_output: String,
    pub cleaned_output: String,
    pub render_output: String,
    pub digest: OutputDigest,
    pub changed_raw: bool,
    pub changed_cleaned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewState {
    pub(crate) lines: Vec<String>,
    pub(crate) render_lines: Vec<String>,
    pub(crate) offset: usize,
    pub(crate) auto_scroll: bool,
    pub(crate) scroll_burst_count: u32,
    pub(crate) recent_captures: VecDeque<CaptureRecord>,
    last_scroll_time: Option<Instant>,
    last_digest: Option<OutputDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureUpdate {
    pub changed_raw: bool,
    pub changed_cleaned: bool,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            render_lines: Vec::new(),
            offset: 0,
            auto_scroll: true,
            scroll_burst_count: 0,
            recent_captures: VecDeque::with_capacity(CAPTURE_RING_CAPACITY),
            last_scroll_time: None,
            last_digest: None,
        }
    }

    pub fn apply_capture(&mut self, raw_output: &str) -> CaptureUpdate {
        let change = evaluate_capture_change(self.last_digest.as_ref(), raw_output);

        let record = CaptureRecord {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis() as u64,
            raw_output: raw_output.to_owned(),
            cleaned_output: change.cleaned_output.clone(),
            render_output: change.render_output.clone(),
            digest: change.digest.clone(),
            changed_raw: change.changed_raw,
            changed_cleaned: change.changed_cleaned,
        };
        if self.recent_captures.len() >= CAPTURE_RING_CAPACITY {
            self.recent_captures.pop_front();
        }
        self.recent_captures.push_back(record);

        self.last_digest = Some(change.digest);

        if change.changed_cleaned {
            self.lines = split_output_lines(&change.cleaned_output);
            self.offset = self.offset.min(self.lines.len());
            if self.auto_scroll {
                self.offset = 0;
            }
        }
        if change.changed_raw {
            self.render_lines = split_output_lines(&change.render_output);
        }

        CaptureUpdate {
            changed_raw: change.changed_raw,
            changed_cleaned: change.changed_cleaned,
        }
    }

    fn max_scroll_offset_for(total_lines: usize, height: usize) -> usize {
        if height == 0 {
            return 0;
        }

        total_lines.saturating_sub(height)
    }

    pub fn max_scroll_offset(&self, height: usize) -> usize {
        Self::max_scroll_offset_for(self.lines.len(), height)
    }

    pub fn scroll(&mut self, delta: i32, now: Instant, viewport_height: usize) -> bool {
        if delta == 0 {
            return false;
        }

        let max_offset = self.max_scroll_offset(viewport_height);
        if max_offset == 0 {
            self.offset = 0;
            self.auto_scroll = true;
            return false;
        }

        if let Some(last_scroll_time) = self.last_scroll_time {
            let since_last = now.saturating_duration_since(last_scroll_time);
            if since_last < Duration::from_millis(SCROLL_DEBOUNCE_MS) {
                self.scroll_burst_count = self.scroll_burst_count.saturating_add(1);
                let burst_debounce = if self.scroll_burst_count > 4 {
                    SCROLL_BURST_DEBOUNCE_MS
                } else {
                    SCROLL_DEBOUNCE_MS
                };

                if since_last < Duration::from_millis(burst_debounce) {
                    return false;
                }
            } else {
                self.scroll_burst_count = 1;
            }
        } else {
            self.scroll_burst_count = 1;
        }

        self.last_scroll_time = Some(now);

        if delta < 0 {
            let next_offset = self
                .offset
                .saturating_add(delta.unsigned_abs() as usize)
                .min(max_offset);
            if next_offset == self.offset {
                return false;
            }

            self.auto_scroll = false;
            self.offset = next_offset;
            return true;
        }

        if delta > 0 {
            let next_offset = self.offset.saturating_sub(delta as usize);
            if next_offset == self.offset {
                return false;
            }

            self.offset = next_offset;
            if self.offset == 0 {
                self.auto_scroll = true;
            }
            return true;
        }

        false
    }

    pub fn jump_to_bottom(&mut self) {
        self.offset = 0;
        self.auto_scroll = true;
    }

    #[cfg(test)]
    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        if height == 0 || self.lines.is_empty() {
            return Vec::new();
        }

        let max_offset = self.max_scroll_offset(height);
        let clamped_offset = self.offset.min(max_offset);
        let end = self.lines.len().saturating_sub(clamped_offset);
        let start = end.saturating_sub(height);
        self.lines[start..end].to_vec()
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn split_output_lines(output: &str) -> Vec<String> {
    let trimmed = output.trim_end_matches('\n');
    if trimmed.is_empty() {
        return Vec::new();
    }

    trimmed.lines().map(ToOwned::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{PreviewState, split_output_lines};

    #[test]
    fn split_output_lines_trims_final_newline() {
        assert_eq!(
            split_output_lines("a\nb\n"),
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(split_output_lines("\n").is_empty());
    }

    #[test]
    fn capture_ignores_mouse_noise_in_clean_diff() {
        let mut state = PreviewState::new();

        let first = state.apply_capture("hello\u{1b}[?1000h\u{1b}[<35;192;47M");
        assert!(first.changed_raw);
        assert!(first.changed_cleaned);
        assert_eq!(state.lines, vec!["hello".to_string()]);
        assert_eq!(state.render_lines, vec!["hello".to_string()]);

        let second = state.apply_capture("hello\u{1b}[?1000l");
        assert!(second.changed_raw);
        assert!(!second.changed_cleaned);
        assert_eq!(state.lines, vec!["hello".to_string()]);
        assert_eq!(state.render_lines, vec!["hello".to_string()]);
    }

    #[test]
    fn scroll_up_pauses_autoscroll_and_scroll_down_resumes_at_bottom() {
        let mut state = PreviewState::new();
        state.lines = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];

        let base = Instant::now();
        assert!(state.scroll(-2, base, 2));
        assert!(!state.auto_scroll);
        assert_eq!(state.offset, 2);

        assert!(state.scroll(1, base + Duration::from_millis(200), 2));
        assert!(!state.auto_scroll);
        assert_eq!(state.offset, 1);

        assert!(state.scroll(1, base + Duration::from_millis(400), 2));
        assert!(state.auto_scroll);
        assert_eq!(state.offset, 0);
    }

    #[test]
    fn scroll_up_clamps_offset_to_available_lines() {
        let mut state = PreviewState::new();
        state.lines = vec!["1".to_string(), "2".to_string()];

        assert!(state.scroll(-10, Instant::now(), 1));
        assert_eq!(state.offset, 1);
    }

    #[test]
    fn apply_capture_clamps_existing_offset_when_output_shrinks() {
        let mut state = PreviewState::new();
        state.lines = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
        state.offset = 3;
        state.auto_scroll = false;

        state.apply_capture("line-a\nline-b");
        assert_eq!(state.lines.len(), 2);
        assert_eq!(state.offset, 2);
    }

    #[test]
    fn scroll_burst_guard_drops_rapid_bursts() {
        let mut state = PreviewState::new();
        state.lines = (1..=20).map(|value| value.to_string()).collect();
        let base = Instant::now();

        assert!(state.scroll(-1, base, 5));
        assert!(!state.scroll(-1, base + Duration::from_millis(1), 5));
        assert!(!state.scroll(-1, base + Duration::from_millis(2), 5));
        assert!(!state.scroll(-1, base + Duration::from_millis(3), 5));
        assert!(!state.scroll(-1, base + Duration::from_millis(4), 5));
        assert!(state.scroll(-1, base + Duration::from_millis(50), 5));
        assert!(state.scroll(-1, base + Duration::from_millis(130), 5));
    }

    #[test]
    fn scroll_is_noop_when_content_fits_viewport() {
        let mut state = PreviewState::new();
        state.lines = vec!["1".to_string(), "2".to_string()];

        assert!(!state.scroll(-1, Instant::now(), 4));
        assert_eq!(state.offset, 0);
        assert!(state.auto_scroll);
    }

    #[test]
    fn visible_lines_respects_offset_from_bottom() {
        let mut state = PreviewState::new();
        state.lines = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
        ];
        state.offset = 1;

        let visible = state.visible_lines(2);
        assert_eq!(visible, vec!["3".to_string(), "4".to_string()]);
    }

    #[test]
    fn capture_record_ring_buffer_caps_at_10() {
        let mut state = PreviewState::new();

        for i in 0..12 {
            state.apply_capture(&format!("output-{i}"));
        }

        assert_eq!(state.recent_captures.len(), 10);
        assert!(
            state
                .recent_captures
                .front()
                .unwrap()
                .raw_output
                .contains("output-2")
        );
        assert!(
            state
                .recent_captures
                .back()
                .unwrap()
                .raw_output
                .contains("output-11")
        );
    }

    #[test]
    fn capture_record_contains_expected_fields() {
        let mut state = PreviewState::new();
        state.apply_capture("hello world");

        assert_eq!(state.recent_captures.len(), 1);
        let record = state.recent_captures.front().unwrap();
        assert_eq!(record.raw_output, "hello world");
        assert!(record.changed_raw);
        assert!(record.changed_cleaned);
        assert!(record.ts > 0);
        assert!(record.digest.raw_len > 0);
    }
}
