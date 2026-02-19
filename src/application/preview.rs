use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::application::agent_runtime::{OutputDigest, evaluate_capture_change};

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
    pub(crate) recent_captures: VecDeque<CaptureRecord>,
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
            recent_captures: VecDeque::with_capacity(CAPTURE_RING_CAPACITY),
            last_digest: None,
        }
    }

    pub fn apply_capture(&mut self, raw_output: &str) -> CaptureUpdate {
        let change = evaluate_capture_change(self.last_digest.as_ref(), raw_output);

        let record = CaptureRecord {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
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

    pub fn scroll(&mut self, delta: i32, _now: Instant, viewport_height: usize) -> bool {
        if delta == 0 {
            return false;
        }

        let max_offset = self.max_scroll_offset(viewport_height);
        if max_offset == 0 {
            self.offset = 0;
            self.auto_scroll = true;
            return false;
        }

        if delta < 0 {
            let delta_abs = usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX);
            let next_offset = self.offset.saturating_add(delta_abs).min(max_offset);
            if next_offset == self.offset {
                return false;
            }

            self.auto_scroll = false;
            self.offset = next_offset;
            return true;
        }

        let delta_positive = usize::try_from(delta).unwrap_or(usize::MAX);
        let next_offset = self.offset.saturating_sub(delta_positive);
        if next_offset == self.offset {
            return false;
        }

        self.offset = next_offset;
        if self.offset == 0 {
            self.auto_scroll = true;
        }
        true
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
    if output.is_empty() {
        return Vec::new();
    }

    output
        .split_terminator('\n')
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
