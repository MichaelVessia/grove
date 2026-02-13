use std::time::{Duration, Instant};

use crate::agent_runtime::{OutputDigest, evaluate_capture_change};

const SCROLL_DEBOUNCE_MS: u64 = 40;
const SCROLL_BURST_DEBOUNCE_MS: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewState {
    pub lines: Vec<String>,
    pub render_lines: Vec<String>,
    pub offset: usize,
    pub auto_scroll: bool,
    pub scroll_burst_count: u32,
    last_scroll_time: Option<Instant>,
    last_digest: Option<OutputDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureUpdate {
    pub changed_raw: bool,
    pub changed_cleaned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashMessage {
    pub text: String,
    pub is_error: bool,
    pub expires_at: Instant,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            render_lines: Vec::new(),
            offset: 0,
            auto_scroll: true,
            scroll_burst_count: 0,
            last_scroll_time: None,
            last_digest: None,
        }
    }

    pub fn apply_capture(&mut self, raw_output: &str) -> CaptureUpdate {
        let change = evaluate_capture_change(self.last_digest.as_ref(), raw_output);
        self.last_digest = Some(change.digest);

        if change.changed_cleaned {
            self.lines = split_output_lines(&change.cleaned_output);
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

    pub fn scroll(&mut self, delta: i32, now: Instant) -> bool {
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
            self.auto_scroll = false;
            self.offset = self.offset.saturating_add(delta.unsigned_abs() as usize);
            return true;
        }

        if delta > 0 {
            self.offset = self.offset.saturating_sub(delta as usize);
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

    pub fn reset_for_selection_change(&mut self) {
        self.jump_to_bottom();
    }

    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        if height == 0 || self.lines.is_empty() {
            return Vec::new();
        }

        let end = self.lines.len().saturating_sub(self.offset);
        let start = end.saturating_sub(height);
        self.lines[start..end].to_vec()
    }

    pub fn visible_render_lines(&self, height: usize) -> Vec<String> {
        if height == 0 || self.render_lines.is_empty() {
            return Vec::new();
        }

        let end = self.render_lines.len().saturating_sub(self.offset);
        let start = end.saturating_sub(height);
        self.render_lines[start..end].to_vec()
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn split_output_lines(output: &str) -> Vec<String> {
    let trimmed = output.trim_end_matches('\n');
    if trimmed.is_empty() {
        return Vec::new();
    }

    trimmed.lines().map(ToOwned::to_owned).collect()
}

pub fn new_flash_message(text: impl Into<String>, is_error: bool, now: Instant) -> FlashMessage {
    FlashMessage {
        text: text.into(),
        is_error,
        expires_at: now + Duration::from_secs(3),
    }
}

pub fn clear_expired_flash_message(flash: &mut Option<FlashMessage>, now: Instant) -> bool {
    if flash
        .as_ref()
        .is_some_and(|message| message.expires_at <= now)
    {
        *flash = None;
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{PreviewState, clear_expired_flash_message, new_flash_message, split_output_lines};

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
        assert!(state.scroll(-2, base));
        assert!(!state.auto_scroll);
        assert_eq!(state.offset, 2);

        assert!(state.scroll(1, base + Duration::from_millis(200)));
        assert!(!state.auto_scroll);
        assert_eq!(state.offset, 1);

        assert!(state.scroll(1, base + Duration::from_millis(400)));
        assert!(state.auto_scroll);
        assert_eq!(state.offset, 0);
    }

    #[test]
    fn scroll_burst_guard_drops_rapid_bursts() {
        let mut state = PreviewState::new();
        let base = Instant::now();

        assert!(state.scroll(-1, base));
        assert!(!state.scroll(-1, base + Duration::from_millis(1)));
        assert!(!state.scroll(-1, base + Duration::from_millis(2)));
        assert!(!state.scroll(-1, base + Duration::from_millis(3)));
        assert!(!state.scroll(-1, base + Duration::from_millis(4)));
        assert!(state.scroll(-1, base + Duration::from_millis(50)));
        assert!(state.scroll(-1, base + Duration::from_millis(130)));
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
    fn visible_render_lines_respects_offset_from_bottom() {
        let mut state = PreviewState::new();
        state.render_lines = vec![
            "r1".to_string(),
            "r2".to_string(),
            "r3".to_string(),
            "r4".to_string(),
            "r5".to_string(),
        ];
        state.offset = 1;

        let visible = state.visible_render_lines(2);
        assert_eq!(visible, vec!["r3".to_string(), "r4".to_string()]);
    }

    #[test]
    fn flash_message_auto_expires_after_three_seconds() {
        let base = Instant::now();
        let mut flash = Some(new_flash_message("ok", false, base));

        assert!(!clear_expired_flash_message(
            &mut flash,
            base + Duration::from_secs(2)
        ));
        assert!(flash.is_some());

        assert!(clear_expired_flash_message(
            &mut flash,
            base + Duration::from_secs(3)
        ));
        assert!(flash.is_none());
    }
}
