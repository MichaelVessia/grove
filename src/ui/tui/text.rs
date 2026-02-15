use ftui::Style;
use ftui::text::{
    Line as FtLine, Span as FtSpan, display_width as text_display_width,
    graphemes as text_graphemes,
};

pub(super) fn line_visual_width(line: &str) -> usize {
    text_display_width(line)
}

pub(super) fn visual_substring(
    line: &str,
    start_col: usize,
    end_col_inclusive: Option<usize>,
) -> String {
    let mut out = String::new();
    let end_col_exclusive = end_col_inclusive.map(|end| end.saturating_add(1));
    let mut visual_col = 0usize;

    for grapheme in text_graphemes(line) {
        if end_col_exclusive.is_some_and(|end| visual_col >= end) {
            break;
        }

        let width = line_visual_width(grapheme);
        let next_col = visual_col.saturating_add(width);
        let intersects = if width == 0 {
            visual_col >= start_col
        } else {
            next_col > start_col
        };

        if intersects {
            out.push_str(grapheme);
        }

        visual_col = next_col;
    }

    out
}

pub(super) fn visual_grapheme_at(line: &str, target_col: usize) -> Option<(String, usize, usize)> {
    let mut visual_col = 0usize;
    for grapheme in text_graphemes(line) {
        let width = line_visual_width(grapheme);
        let start_col = visual_col;
        let end_col = if width == 0 {
            start_col
        } else {
            start_col.saturating_add(width.saturating_sub(1))
        };

        if (width == 0 && target_col == start_col) || (width > 0 && target_col <= end_col) {
            return Some((grapheme.to_string(), start_col, end_col));
        }

        visual_col = visual_col.saturating_add(width);
    }

    None
}

pub(super) fn truncate_for_log(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(super) fn truncate_to_display_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text_display_width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let target_width = max_width.saturating_sub(1);
    for grapheme in text_graphemes(value) {
        let grapheme_width = line_visual_width(grapheme);
        if width.saturating_add(grapheme_width) > target_width {
            break;
        }
        out.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    out.push('…');
    out
}

pub(super) fn pad_or_truncate_to_display_width(value: &str, width: usize) -> String {
    let mut out = truncate_to_display_width(value, width);
    let used = text_display_width(out.as_str());
    if used < width {
        out.push_str(&" ".repeat(width.saturating_sub(used)));
    }
    out
}

fn clip_to_display_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text_display_width(value) <= max_width {
        return value.to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    for grapheme in text_graphemes(value) {
        let grapheme_width = line_visual_width(grapheme);
        if width.saturating_add(grapheme_width) > max_width {
            break;
        }
        out.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    out
}

fn spans_display_width(spans: &[FtSpan<'_>]) -> usize {
    spans
        .iter()
        .map(|span| text_display_width(span.content.as_ref()))
        .sum()
}

fn truncate_spans_to_width(spans: &[FtSpan<'_>], max_width: usize) -> Vec<FtSpan<'static>> {
    if max_width == 0 {
        return Vec::new();
    }

    let mut rendered: Vec<FtSpan<'static>> = Vec::new();
    let mut used = 0usize;
    for span in spans {
        if used >= max_width {
            break;
        }

        let remaining = max_width.saturating_sub(used);
        let rendered_text = clip_to_display_width(span.content.as_ref(), remaining);
        if rendered_text.is_empty() {
            continue;
        }

        let rendered_span = match span.style {
            Some(style) => FtSpan::styled(rendered_text, style),
            None => FtSpan::raw(rendered_text),
        };
        used = used.saturating_add(text_display_width(rendered_span.content.as_ref()));
        rendered.push(rendered_span);
    }

    rendered
}

pub(super) fn chrome_bar_line(
    width: usize,
    base_style: Style,
    left: Vec<FtSpan<'static>>,
    center: Vec<FtSpan<'static>>,
    right: Vec<FtSpan<'static>>,
) -> FtLine {
    if width == 0 {
        return FtLine::raw("");
    }

    let right = truncate_spans_to_width(&right, width);
    let right_width = spans_display_width(&right);
    let right_start = width.saturating_sub(right_width);

    let center = truncate_spans_to_width(&center, width);
    let center_width = spans_display_width(&center);
    let center_start = width.saturating_sub(center_width) / 2;
    let center_can_render =
        center_width > 0 && center_start.saturating_add(center_width) <= right_start;

    let left_max_width = if center_can_render {
        center_start
    } else {
        right_start
    };
    let left = truncate_spans_to_width(&left, left_max_width);
    let left_width = spans_display_width(&left);

    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    spans.extend(left);
    let mut cursor = left_width;

    if center_can_render {
        if center_start > cursor {
            spans.push(FtSpan::styled(
                " ".repeat(center_start.saturating_sub(cursor)),
                base_style,
            ));
        }
        spans.extend(center);
        cursor = center_start.saturating_add(center_width);
    }

    if right_start > cursor {
        spans.push(FtSpan::styled(
            " ".repeat(right_start.saturating_sub(cursor)),
            base_style,
        ));
    }
    spans.extend(right);
    cursor = right_start.saturating_add(right_width);

    if width > cursor {
        spans.push(FtSpan::styled(
            " ".repeat(width.saturating_sub(cursor)),
            base_style,
        ));
    }

    FtLine::from_spans(spans)
}

pub(super) fn keybind_hint_spans(
    hints: &str,
    base_style: Style,
    key_style: Style,
    sep_style: Style,
) -> Vec<FtSpan<'static>> {
    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    for (chunk_index, chunk) in hints.split(", ").enumerate() {
        if chunk_index > 0 {
            spans.push(FtSpan::styled(", ", sep_style));
        }

        if let Some(split_index) = chunk.rfind(' ') {
            let key = &chunk[..split_index];
            let action = &chunk[split_index..];
            if !key.is_empty() {
                spans.push(FtSpan::styled(key.to_string(), key_style));
            }
            if !action.is_empty() {
                spans.push(FtSpan::styled(action.to_string(), base_style));
            }
            continue;
        }

        spans.push(FtSpan::styled(chunk.to_string(), key_style));
    }

    spans
}

pub(super) fn ansi_line_to_plain_text(line: &str) -> String {
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            plain.push(character);
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                for value in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        break;
                    }
                }
            }
            ']' => {
                while let Some(value) = chars.next() {
                    if value == '\u{7}' {
                        break;
                    }
                    if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                        break;
                    }
                }
            }
            'P' | 'X' | '^' | '_' => {
                while let Some(value) = chars.next() {
                    if value == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                        break;
                    }
                }
            }
            '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' => {
                let _ = chars.next();
            }
            _ => {}
        }
    }

    plain
}
