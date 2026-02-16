use ftui::text::{Line as FtLine, Span as FtSpan};
use ftui::{PackedRgba, Style};

use super::colors::{ansi_16_color, ansi_256_color};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct AnsiStyleState {
    fg: Option<PackedRgba>,
    bg: Option<PackedRgba>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    strikethrough: bool,
}

impl AnsiStyleState {
    fn into_style(self) -> Option<Style> {
        let mut style = Style::new();

        if let Some(fg) = self.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }
        if self.bold {
            style = style.bold();
        }
        if self.dim {
            style = style.dim();
        }
        if self.italic {
            style = style.italic();
        }
        if self.underline {
            style = style.underline();
        }
        if self.blink {
            style = style.blink();
        }
        if self.reverse {
            style = style.reverse();
        }
        if self.strikethrough {
            style = style.strikethrough();
        }

        if style == Style::new() {
            return None;
        }

        Some(style)
    }
}

fn parse_sgr_extended_color(params: &[i32], start: usize) -> Option<(PackedRgba, usize)> {
    let mode = *params.get(start)?;
    match mode {
        5 => {
            let value = *params.get(start.saturating_add(1))?;
            let palette = u8::try_from(value).ok()?;
            Some((ansi_256_color(palette), 2))
        }
        2 => {
            let r = u8::try_from(*params.get(start.saturating_add(1))?).ok()?;
            let g = u8::try_from(*params.get(start.saturating_add(2))?).ok()?;
            let b = u8::try_from(*params.get(start.saturating_add(3))?).ok()?;
            Some((PackedRgba::rgb(r, g, b), 4))
        }
        _ => None,
    }
}

fn apply_sgr_codes(raw_params: &str, state: &mut AnsiStyleState) {
    let params: Vec<i32> = if raw_params.is_empty() {
        vec![0]
    } else {
        raw_params
            .split(';')
            .map(|value| {
                if value.is_empty() {
                    0
                } else {
                    value.parse::<i32>().unwrap_or(-1)
                }
            })
            .collect()
    };

    let mut index = 0usize;
    while index < params.len() {
        match params[index] {
            0 => *state = AnsiStyleState::default(),
            1 => state.bold = true,
            2 => state.dim = true,
            3 => state.italic = true,
            4 => state.underline = true,
            5 => state.blink = true,
            7 => state.reverse = true,
            9 => state.strikethrough = true,
            22 => {
                state.bold = false;
                state.dim = false;
            }
            23 => state.italic = false,
            24 => state.underline = false,
            25 => state.blink = false,
            27 => state.reverse = false,
            29 => state.strikethrough = false,
            30..=37 => {
                if let Ok(code) = u8::try_from(params[index] - 30) {
                    state.fg = Some(ansi_16_color(code));
                }
            }
            90..=97 => {
                if let Ok(code) = u8::try_from(params[index] - 90) {
                    state.fg = Some(ansi_16_color(code.saturating_add(8)));
                }
            }
            40..=47 => {
                if let Ok(code) = u8::try_from(params[index] - 40) {
                    state.bg = Some(ansi_16_color(code));
                }
            }
            100..=107 => {
                if let Ok(code) = u8::try_from(params[index] - 100) {
                    state.bg = Some(ansi_16_color(code.saturating_add(8)));
                }
            }
            38 => {
                if let Some((color, consumed)) =
                    parse_sgr_extended_color(&params, index.saturating_add(1))
                {
                    state.fg = Some(color);
                    index = index.saturating_add(consumed);
                }
            }
            48 => {
                if let Some((color, consumed)) =
                    parse_sgr_extended_color(&params, index.saturating_add(1))
                {
                    state.bg = Some(color);
                    index = index.saturating_add(consumed);
                }
            }
            39 => state.fg = None,
            49 => state.bg = None,
            _ => {}
        }

        index = index.saturating_add(1);
    }
}

pub(in crate::ui::tui) fn ansi_line_to_styled_line(line: &str) -> FtLine {
    let mut spans: Vec<FtSpan<'static>> = Vec::new();
    let mut buffer = String::new();
    let mut state = AnsiStyleState::default();
    let mut chars = line.chars().peekable();

    let flush = |buffer: &mut String, spans: &mut Vec<FtSpan<'static>>, state: AnsiStyleState| {
        if buffer.is_empty() {
            return;
        }
        let content = std::mem::take(buffer);
        if let Some(style) = state.into_style() {
            spans.push(FtSpan::styled(content, style));
        } else {
            spans.push(FtSpan::raw(content));
        }
    };

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            buffer.push(character);
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                let mut params = String::new();
                let mut final_char: Option<char> = None;
                for value in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        final_char = Some(value);
                        break;
                    }
                    params.push(value);
                }
                if final_char == Some('m') {
                    flush(&mut buffer, &mut spans, state);
                    apply_sgr_codes(&params, &mut state);
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

    flush(&mut buffer, &mut spans, state);

    if spans.is_empty() {
        return FtLine::raw("");
    }

    FtLine::from_spans(spans)
}
