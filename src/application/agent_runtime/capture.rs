use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{CaptureChange, OutputDigest};

pub fn tmux_capture_error_indicates_missing_session(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("can't find pane")
        || lower.contains("can't find session")
        || lower.contains("no server running")
        || lower.contains("no sessions")
        || lower.contains("failed to connect to server")
        || lower.contains("no active session")
        || lower.contains("session not found")
}

pub(crate) fn evaluate_capture_change(
    previous: Option<&OutputDigest>,
    raw_output: &str,
) -> CaptureChange {
    let render_output = strip_non_sgr_control_sequences(raw_output);
    let cleaned_output = strip_mouse_fragments(&strip_sgr_sequences(&render_output));
    let digest = OutputDigest {
        raw_hash: content_hash(raw_output),
        raw_len: raw_output.len(),
        cleaned_hash: content_hash(&cleaned_output),
    };

    match previous {
        None => CaptureChange {
            digest,
            changed_raw: true,
            changed_cleaned: true,
            cleaned_output,
            render_output,
        },
        Some(previous_digest) => CaptureChange {
            changed_raw: previous_digest.raw_hash != digest.raw_hash
                || previous_digest.raw_len != digest.raw_len,
            changed_cleaned: previous_digest.cleaned_hash != digest.cleaned_hash,
            digest,
            cleaned_output,
            render_output,
        },
    }
}

fn is_safe_text_character(character: char) -> bool {
    matches!(character, '\n' | '\t') || !character.is_control()
}

pub(crate) fn strip_mouse_fragments(input: &str) -> String {
    let cleaned = strip_mouse_mode_sequences(input);
    strip_partial_mouse_sequences(&cleaned)
}

const MOUSE_MODE_SEQUENCES: [&[u8]; 28] = [
    b"\x1b[?1000h",
    b"\x1b[?1000l",
    b"[?1000h",
    b"[?1000l",
    b"\x1b[?1002h",
    b"\x1b[?1002l",
    b"[?1002h",
    b"[?1002l",
    b"\x1b[?1003h",
    b"\x1b[?1003l",
    b"[?1003h",
    b"[?1003l",
    b"\x1b[?1005h",
    b"\x1b[?1005l",
    b"[?1005h",
    b"[?1005l",
    b"\x1b[?1006h",
    b"\x1b[?1006l",
    b"[?1006h",
    b"[?1006l",
    b"\x1b[?1015h",
    b"\x1b[?1015l",
    b"[?1015h",
    b"[?1015l",
    b"\x1b[?2004h",
    b"\x1b[?2004l",
    b"[?2004h",
    b"[?2004l",
];

fn strip_mouse_mode_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = parse_mouse_mode_sequence_end(bytes, index) {
            index = end;
            continue;
        }

        output.push(bytes[index]);
        index = index.saturating_add(1);
    }

    String::from_utf8(output).unwrap_or_default()
}

fn parse_mouse_mode_sequence_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes
        .get(start)
        .is_none_or(|byte| *byte != b'[' && *byte != b'\x1b')
    {
        return None;
    }

    for pattern in MOUSE_MODE_SEQUENCES {
        if bytes[start..].starts_with(pattern) {
            return Some(start.saturating_add(pattern.len()));
        }
    }

    None
}

fn strip_non_sgr_control_sequences(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            if is_safe_text_character(character) {
                cleaned.push(character);
            }
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                let mut csi = String::from("\u{1b}[");
                if let Some(final_char) = consume_csi_sequence(&mut chars, &mut csi)
                    && final_char == 'm'
                {
                    cleaned.push_str(&csi);
                }
            }
            ']' => consume_osc_sequence(&mut chars),
            'P' | 'X' | '^' | '_' => consume_st_sequence(&mut chars),
            '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' => {
                let _ = chars.next();
            }
            _ => {}
        }
    }

    cleaned
}

fn strip_sgr_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'\x1b' {
            if bytes.get(index.saturating_add(1)) == Some(&b'[') {
                index = index.saturating_add(2);
                while index < bytes.len() {
                    let value = bytes[index];
                    index = index.saturating_add(1);
                    if (b'@'..=b'~').contains(&value) {
                        break;
                    }
                }
                continue;
            }

            index = index.saturating_add(1);
            continue;
        }

        output.push(bytes[index]);
        index = index.saturating_add(1);
    }

    String::from_utf8(output).unwrap_or_default()
}

fn consume_csi_sequence<I>(chars: &mut std::iter::Peekable<I>, buffer: &mut String) -> Option<char>
where
    I: Iterator<Item = char>,
{
    for character in chars.by_ref() {
        buffer.push(character);
        if ('\u{40}'..='\u{7e}').contains(&character) {
            return Some(character);
        }
    }

    None
}

fn consume_osc_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{7}' {
            return;
        }

        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn consume_st_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn strip_partial_mouse_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if matches!(byte, b'[' | b'M' | b'm')
            && let Some(end) = parse_mouse_fragment_end(bytes, index)
        {
            index = end;
            continue;
        }

        output.push(byte);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_default()
}

fn parse_mouse_fragment_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) == Some(&b'[') && bytes.get(start.saturating_add(1)) == Some(&b'<') {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(2));
    }
    if matches!(bytes.get(start), Some(b'M' | b'm'))
        && bytes.get(start.saturating_add(1)) == Some(&b'[')
        && bytes.get(start.saturating_add(2)) == Some(&b'<')
    {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(3));
    }

    None
}

fn parse_sgr_mouse_tail(bytes: &[u8], mut index: usize) -> Option<usize> {
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if matches!(bytes.get(index), Some(b'M' | b'm')) {
        index = index.saturating_add(1);
    }

    Some(index)
}

fn consume_ascii_digits(bytes: &[u8], mut start: usize) -> Option<usize> {
    let initial = start;
    while matches!(bytes.get(start), Some(b'0'..=b'9')) {
        start = start.saturating_add(1);
    }

    if start == initial { None } else { Some(start) }
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
