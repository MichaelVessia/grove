use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ftui::text::display_width;
use ftui_pty::virtual_terminal::{CellStyle, VirtualTerminal};

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
pub(crate) struct PreviewParsedStyle {
    pub(crate) foreground_rgb: Option<(u8, u8, u8)>,
    pub(crate) background_rgb: Option<(u8, u8, u8)>,
    pub(crate) bold: bool,
    pub(crate) dim: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) blink: bool,
    pub(crate) reverse: bool,
    pub(crate) strikethrough: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewParsedSpan {
    pub(crate) text: String,
    pub(crate) style: PreviewParsedStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewParsedLine {
    pub(crate) spans: Vec<PreviewParsedSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewState {
    pub(crate) lines: Vec<String>,
    pub(crate) parsed_lines: Vec<PreviewParsedLine>,
    pub(crate) render_lines: Vec<String>,
    pub(crate) recent_captures: VecDeque<CaptureRecord>,
    last_digest: Option<OutputDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureUpdate {
    pub changed_raw: bool,
    pub changed_cleaned: bool,
    pub cleaned_output: String,
    pub digest: OutputDigest,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            parsed_lines: Vec::new(),
            render_lines: Vec::new(),
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

        let result_digest = change.digest.clone();
        self.last_digest = Some(change.digest);

        if change.changed_raw {
            self.render_lines = split_output_lines(&change.render_output);
            let snapshot = parse_preview_snapshot(&self.render_lines);
            self.lines = snapshot.plain_lines;
            self.parsed_lines = snapshot.parsed_lines;
        } else if change.changed_cleaned {
            self.lines = split_output_lines(&change.cleaned_output);
        }

        CaptureUpdate {
            changed_raw: change.changed_raw,
            changed_cleaned: change.changed_cleaned,
            cleaned_output: change.cleaned_output,
            digest: result_digest,
        }
    }

    pub(crate) fn active_plain_lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn active_plain_line(&self, line_idx: usize) -> Option<&String> {
        self.lines.get(line_idx)
    }

    pub(crate) fn active_parsed_lines(&self) -> &[PreviewParsedLine] {
        &self.parsed_lines
    }

    pub(crate) fn reset_selected_session_state(&mut self) {
        self.lines.clear();
        self.parsed_lines.clear();
        self.render_lines.clear();
        self.recent_captures.clear();
        self.last_digest = None;
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

    let split_lines: Vec<&str> = output.split_terminator('\n').collect();
    let last_index = split_lines.len().saturating_sub(1);
    let ends_with_newline = output.ends_with('\n');

    split_lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index < last_index || ends_with_newline {
                line.strip_suffix('\r').unwrap_or(line).to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect()
}

struct PreviewSnapshot {
    plain_lines: Vec<String>,
    parsed_lines: Vec<PreviewParsedLine>,
}

fn parse_preview_snapshot(lines: &[String]) -> PreviewSnapshot {
    if lines.is_empty() {
        return PreviewSnapshot {
            plain_lines: Vec::new(),
            parsed_lines: Vec::new(),
        };
    }

    let height = u16::try_from(lines.len().max(1)).unwrap_or(u16::MAX);
    let width = u16::try_from(
        lines
            .iter()
            .map(|line| display_width(line.as_str()))
            .max()
            .unwrap_or(0)
            .max(1),
    )
    .unwrap_or(u16::MAX);
    let mut terminal = VirtualTerminal::new(width, height);
    let joined = lines.join("\r\n");
    terminal.feed_str(&joined);

    PreviewSnapshot {
        plain_lines: (0..height).map(|row| terminal.row_text(row)).collect(),
        parsed_lines: (0..height)
            .map(|row| parse_preview_line(&terminal, row))
            .collect(),
    }
}

fn parse_preview_line(terminal: &VirtualTerminal, row: u16) -> PreviewParsedLine {
    let mut spans = Vec::new();
    let row_text = terminal.row_text(row);
    let cell_count = display_width(row_text.as_str());
    let mut active_text = String::new();
    let mut active_style: Option<PreviewParsedStyle> = None;

    for column in 0..cell_count {
        let x = u16::try_from(column).unwrap_or(u16::MAX);
        let Some(cell) = terminal.cell_at(x, row) else {
            break;
        };
        if cell.ch == '\0' {
            continue;
        }
        let next_style = preview_style_from_cell(&cell.style);
        match &active_style {
            Some(style) if *style == next_style => active_text.push(cell.ch),
            Some(style) => {
                spans.push(PreviewParsedSpan {
                    text: std::mem::take(&mut active_text),
                    style: style.clone(),
                });
                active_text.push(cell.ch);
                active_style = Some(next_style);
            }
            None => {
                active_text.push(cell.ch);
                active_style = Some(next_style);
            }
        }
    }

    if let Some(style) = active_style {
        spans.push(PreviewParsedSpan {
            text: active_text,
            style,
        });
    }

    PreviewParsedLine { spans }
}

fn preview_style_from_cell(style: &CellStyle) -> PreviewParsedStyle {
    PreviewParsedStyle {
        foreground_rgb: style.fg.map(|color| (color.r, color.g, color.b)),
        background_rgb: style.bg.map(|color| (color.r, color.g, color.b)),
        bold: style.bold,
        dim: style.dim,
        italic: style.italic,
        underline: style.underline,
        blink: style.blink,
        reverse: style.reverse,
        strikethrough: style.strikethrough,
    }
}

#[cfg(test)]
mod tests {
    use super::{PreviewState, split_output_lines};

    #[test]
    fn split_output_lines_preserves_trailing_blank_rows() {
        assert_eq!(
            split_output_lines("a\nb\n"),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(split_output_lines("\n"), vec!["".to_string()]);
        assert_eq!(
            split_output_lines("a\n\n\n"),
            vec!["a".to_string(), "".to_string(), "".to_string()]
        );
    }

    #[test]
    fn split_output_lines_normalizes_crlf_line_endings() {
        assert_eq!(
            split_output_lines("alpha\r\nbeta\r\n"),
            vec!["alpha".to_string(), "beta".to_string()]
        );
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
    fn raw_only_control_sequence_change_does_not_rewrite_visible_preview() {
        let mut state = PreviewState::new();

        let first = state.apply_capture("hello");
        assert!(first.changed_raw);
        assert!(first.changed_cleaned);
        assert_eq!(state.lines, vec!["hello".to_string()]);

        let second = state.apply_capture("hello\u{1b}[2J");
        assert!(second.changed_raw);
        assert!(!second.changed_cleaned);
        assert_eq!(state.lines, vec!["hello".to_string()]);
        assert_eq!(state.parsed_lines.len(), 1);
        assert_eq!(state.parsed_lines[0].spans.len(), 1);
        assert_eq!(state.parsed_lines[0].spans[0].text, "hello".to_string());
    }

    #[test]
    fn apply_capture_replaces_lines_when_clean_output_changes() {
        let mut state = PreviewState::new();
        state.apply_capture("1\n2\n3\n4\n5");

        assert_eq!(
            state.lines,
            vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
            ]
        );
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

    #[test]
    fn apply_capture_builds_plain_and_styled_preview_from_ansi() {
        let mut state = PreviewState::new();

        state.apply_capture("a\u{1b}[31mb\u{1b}[0mc");

        assert_eq!(state.lines, vec!["abc".to_string()]);
        assert_eq!(state.parsed_lines.len(), 1);
        let line = &state.parsed_lines[0];
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].text, "b");
        assert!(line.spans[1].style.foreground_rgb.is_some());
    }

    #[test]
    fn apply_capture_builds_plain_and_styled_preview_from_colon_delimited_ansi() {
        let mut state = PreviewState::new();

        state.apply_capture("a\u{1b}[38:2::255:0:0mb\u{1b}[0mc");

        assert_eq!(state.lines, vec!["abc".to_string()]);
        assert_eq!(state.parsed_lines.len(), 1);
        let line = &state.parsed_lines[0];
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].text, "b");
        assert_eq!(line.spans[1].style.foreground_rgb, Some((255, 0, 0)));
    }

    #[test]
    fn apply_capture_carries_style_across_lines_until_reset() {
        let mut state = PreviewState::new();

        state.apply_capture("a\u{1b}[31mb\nc\n\u{1b}[0md");

        assert_eq!(
            state.lines,
            vec!["ab".to_string(), "c".to_string(), "d".to_string()]
        );
        assert_eq!(state.parsed_lines.len(), 3);
        assert!(
            state.parsed_lines[1].spans[0]
                .style
                .foreground_rgb
                .is_some()
        );
        assert_eq!(state.parsed_lines[2].spans[0].style.foreground_rgb, None);
    }

    #[test]
    fn apply_capture_uses_terminal_text_for_carriage_return_overwrite() {
        let mut state = PreviewState::new();

        state.apply_capture("hello\rxy");

        assert_eq!(state.lines, vec!["xyllo".to_string()]);
    }

    #[test]
    fn apply_capture_preserves_text_after_wide_characters() {
        let mut state = PreviewState::new();

        state.apply_capture("A中B");

        assert_eq!(state.lines, vec!["A中B".to_string()]);
        assert_eq!(state.parsed_lines[0].spans[0].text, "A中B");
    }

    #[test]
    fn apply_capture_builds_parsed_lines_for_plain_multiline_text() {
        let mut state = PreviewState::new();

        state.apply_capture("first\nsecond\nthird\n");

        assert_eq!(
            state.lines,
            vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string(),
            ]
        );
        assert_eq!(
            state
                .parsed_lines
                .iter()
                .map(|line| line
                    .spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>())
                .collect::<Vec<_>>(),
            vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string(),
            ]
        );
    }

    #[test]
    fn apply_capture_with_empty_output_keeps_parsed_lines_empty() {
        let mut state = PreviewState::new();

        state.apply_capture("");

        assert!(state.lines.is_empty());
        assert!(state.parsed_lines.is_empty());
        assert!(state.render_lines.is_empty());
    }
}
