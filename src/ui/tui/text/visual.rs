use ftui::text::{display_width as text_display_width, graphemes as text_graphemes};

pub(in crate::ui::tui) fn line_visual_width(line: &str) -> usize {
    text_display_width(line)
}

pub(in crate::ui::tui) fn visual_substring(
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

pub(in crate::ui::tui) fn visual_grapheme_at(
    line: &str,
    target_col: usize,
) -> Option<(String, usize, usize)> {
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

pub(in crate::ui::tui) fn truncate_for_log(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(in crate::ui::tui) fn truncate_to_display_width(value: &str, max_width: usize) -> String {
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

pub(in crate::ui::tui) fn pad_or_truncate_to_display_width(value: &str, width: usize) -> String {
    let mut out = truncate_to_display_width(value, width);
    let used = text_display_width(out.as_str());
    if used < width {
        out.push_str(&" ".repeat(width.saturating_sub(used)));
    }
    out
}
