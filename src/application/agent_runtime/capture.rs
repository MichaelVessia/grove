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
    let (render_output, cleaned_without_sgr) = strip_non_sgr_control_sequences(raw_output);
    let cleaned_output = strip_mouse_fragments(&cleaned_without_sgr);
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

fn is_safe_clean_text_character(character: char) -> bool {
    matches!(character, '\n' | '\t') || !character.is_control()
}

fn is_safe_render_text_character(character: char) -> bool {
    matches!(character, '\r' | '\n' | '\t') || !character.is_control()
}

pub(crate) fn strip_mouse_fragments(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];

        if matches!(byte, b'[' | b'\x1b')
            && let Some(end) = parse_mouse_mode_sequence_end(bytes, index)
        {
            index = end;
            continue;
        }

        if matches!(byte, b'[' | b'M' | b'm')
            && let Some(end) = parse_mouse_fragment_end(bytes, index)
        {
            index = end;
            continue;
        }

        output.push(byte);
        index = index.saturating_add(1);
    }

    String::from_utf8(output).unwrap_or_default()
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

fn strip_non_sgr_control_sequences(input: &str) -> (String, String) {
    let mut render_output = String::with_capacity(input.len());
    let mut cleaned_without_sgr = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'\x1b' {
            if byte.is_ascii() {
                let character = char::from(byte);
                if character == '\r' && bytes.get(index.saturating_add(1)) == Some(&b'\n') {
                    index = index.saturating_add(1);
                    continue;
                }
                if is_safe_render_text_character(character) {
                    render_output.push(character);
                }
                if is_safe_clean_text_character(character) {
                    cleaned_without_sgr.push(character);
                }
                index = index.saturating_add(1);
                continue;
            }

            let Some(character) = input[index..].chars().next() else {
                break;
            };
            if is_safe_render_text_character(character) {
                render_output.push(character);
            }
            if is_safe_clean_text_character(character) {
                cleaned_without_sgr.push(character);
            }
            index = index.saturating_add(character.len_utf8());
            continue;
        }

        let Some(next) = bytes.get(index.saturating_add(1)).copied() else {
            break;
        };

        match next {
            b'[' => {
                let mut scan = index.saturating_add(2);
                while scan < bytes.len() {
                    let final_byte = bytes[scan];
                    scan = scan.saturating_add(1);
                    if (b'@'..=b'~').contains(&final_byte) {
                        if final_byte == b'm' {
                            render_output.push_str(&input[index..scan]);
                        }
                        index = scan;
                        break;
                    }
                }
                if scan >= bytes.len() {
                    break;
                }
            }
            b']' => {
                let mut scan = index.saturating_add(2);
                let mut terminated = false;
                while scan < bytes.len() {
                    let value = bytes[scan];
                    if value == b'\x07' {
                        index = scan.saturating_add(1);
                        terminated = true;
                        break;
                    }
                    if value == b'\x1b' && bytes.get(scan.saturating_add(1)) == Some(&b'\\') {
                        index = scan.saturating_add(2);
                        terminated = true;
                        break;
                    }
                    scan = scan.saturating_add(1);
                }
                if !terminated {
                    break;
                }
            }
            b'P' | b'X' | b'^' | b'_' => {
                let mut scan = index.saturating_add(2);
                let mut terminated = false;
                while scan < bytes.len() {
                    if bytes[scan] == b'\x1b' && bytes.get(scan.saturating_add(1)) == Some(&b'\\') {
                        index = scan.saturating_add(2);
                        terminated = true;
                        break;
                    }
                    scan = scan.saturating_add(1);
                }
                if !terminated {
                    break;
                }
            }
            b'(' | b')' | b'*' | b'+' | b'-' | b'.' | b'/' | b'#' => {
                index = index.saturating_add(2);
                if index < bytes.len() {
                    let Some(character) = input[index..].chars().next() else {
                        break;
                    };
                    index = index.saturating_add(character.len_utf8());
                }
            }
            _ => {
                index = index.saturating_add(2);
            }
        }
    }

    (render_output, cleaned_without_sgr)
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

#[cfg(test)]
mod tests {
    use super::{evaluate_capture_change, strip_mouse_fragments};
    use crate::application::agent_runtime::CaptureChange;

    #[test]
    fn capture_change_detects_mouse_fragment_noise() {
        let first = evaluate_capture_change(None, "hello\u{1b}[?1000h\u{1b}[<35;192;47M");
        assert!(first.changed_raw);
        assert!(first.changed_cleaned);

        let second = evaluate_capture_change(Some(&first.digest), "hello\u{1b}[?1000l");
        assert!(second.changed_raw);
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "hello");

        let third = evaluate_capture_change(Some(&second.digest), "hello world");
        assert!(third.changed_cleaned);
    }

    #[test]
    fn capture_change_first_capture_marks_changed() {
        let change: CaptureChange = evaluate_capture_change(None, "one");
        assert!(change.changed_raw);
        assert!(change.changed_cleaned);
    }

    #[test]
    fn capture_change_strips_ansi_control_sequences() {
        let raw = "A\u{1b}[31mB\u{1b}[39m C\u{1b}]0;title\u{7}\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "AB C\n");
        assert_eq!(change.render_output, "A\u{1b}[31mB\u{1b}[39m C\n");
    }

    #[test]
    fn capture_change_strips_sgr_and_preserves_unicode_text() {
        let raw = "start🙂\u{1b}[31m中\u{1b}[0mend\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "start🙂中end\n");
    }

    #[test]
    fn capture_change_strips_terminal_control_bytes() {
        let raw = "A\u{000e}B\u{000f}C\r\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "ABC\n");
        assert_eq!(change.render_output, "ABC\n");
    }

    #[test]
    fn capture_change_preserves_carriage_return_for_render_output() {
        let raw = "hello\rxy";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "helloxy");
        assert_eq!(change.render_output, "hello\rxy");
    }

    #[test]
    fn capture_change_drops_charset_escape_with_multibyte_suffix() {
        let raw = "A\u{1b}(🙂B";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "AB");
        assert_eq!(change.render_output, "AB");
    }

    #[test]
    fn capture_change_ignores_truncated_partial_mouse_fragments() {
        let first = evaluate_capture_change(None, "prompt [<65;103;31");
        assert_eq!(first.cleaned_output, "prompt ");

        let second = evaluate_capture_change(Some(&first.digest), "prompt [<65;103;32");
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "prompt ");
    }

    #[test]
    fn strip_mouse_fragments_removes_terminal_modes_and_preserves_normal_brackets() {
        assert_eq!(strip_mouse_fragments("value[?1002h"), "value");
        assert_eq!(strip_mouse_fragments("keep [test]"), "keep [test]");
        assert_eq!(strip_mouse_fragments("value[?9999h"), "value[?9999h");
    }

    #[test]
    fn strip_mouse_fragments_removes_boundary_prefixed_partial_sequences() {
        assert_eq!(strip_mouse_fragments("prompt M[<64;107;16M"), "prompt ");
        assert_eq!(strip_mouse_fragments("prompt m[<65;107;14"), "prompt ");
    }

    #[test]
    fn strip_mouse_fragments_removes_modes_and_partials_in_single_input() {
        let raw = "A\u{1b}[?1002hB[<65;107;16MC";
        assert_eq!(strip_mouse_fragments(raw), "ABC");
    }

    #[test]
    fn strip_mouse_fragments_preserves_non_mouse_candidate_prefixes() {
        assert_eq!(strip_mouse_fragments("Mnot-mouse"), "Mnot-mouse");
        assert_eq!(strip_mouse_fragments("text [<x;1;2"), "text [<x;1;2");
    }
}
