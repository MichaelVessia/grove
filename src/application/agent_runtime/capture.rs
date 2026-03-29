use std::borrow::Cow;
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
    let normalized_output = normalize_colon_delimited_sgr_sequences(raw_output);
    let (render_output, cleaned_output) = strip_non_sgr_control_sequences(&normalized_output);
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

fn has_colon_delimited_sgr(bytes: &[u8]) -> bool {
    let mut index = 0usize;
    while index.saturating_add(1) < bytes.len() {
        if bytes[index] == b'\x1b' && bytes[index.saturating_add(1)] == b'[' {
            let mut scan = index.saturating_add(2);
            while scan < bytes.len() {
                let byte = bytes[scan];
                if byte == b':' {
                    while scan < bytes.len() {
                        if (b'@'..=b'~').contains(&bytes[scan]) {
                            if bytes[scan] == b'm' {
                                return true;
                            }
                            break;
                        }
                        scan = scan.saturating_add(1);
                    }
                    break;
                }
                if (b'@'..=b'~').contains(&byte) {
                    break;
                }
                scan = scan.saturating_add(1);
            }
            index = if scan < bytes.len() {
                scan.saturating_add(1)
            } else {
                bytes.len()
            };
        } else {
            index = index.saturating_add(1);
        }
    }
    false
}

fn normalize_colon_delimited_sgr_sequences(input: &str) -> Cow<'_, str> {
    let bytes = input.as_bytes();
    if !has_colon_delimited_sgr(bytes) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'\x1b' && bytes.get(index.saturating_add(1)) == Some(&b'[') {
            let mut scan = index.saturating_add(2);
            while scan < bytes.len() {
                let final_byte = bytes[scan];
                if (b'@'..=b'~').contains(&final_byte) {
                    let params = &input[index.saturating_add(2)..scan];
                    if final_byte == b'm' && params.contains(':') {
                        output.push_str("\x1b[");
                        output.push_str(&normalize_sgr_params(params));
                        output.push('m');
                    } else {
                        output.push_str(&input[index..scan.saturating_add(1)]);
                    }
                    index = scan.saturating_add(1);
                    break;
                }
                scan = scan.saturating_add(1);
            }
            if scan >= bytes.len() {
                output.push_str(&input[index..]);
                break;
            }
            continue;
        }

        let Some(character) = input[index..].chars().next() else {
            break;
        };
        output.push(character);
        index = index.saturating_add(character.len_utf8());
    }

    Cow::Owned(output)
}

fn normalize_sgr_params(params: &str) -> String {
    params
        .split(';')
        .flat_map(|segment| segment.split(':').filter(|value| !value.is_empty()))
        .collect::<Vec<_>>()
        .join(";")
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

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{evaluate_capture_change, normalize_colon_delimited_sgr_sequences};
    use crate::application::agent_runtime::CaptureChange;

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
    fn capture_change_normalizes_colon_delimited_sgr_sequences() {
        let raw = "A\u{1b}[1;38:2::255:0:0mB\u{1b}[48:5:196mC\u{1b}[0m";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "ABC");
        assert_eq!(
            change.render_output,
            "A\u{1b}[1;38;2;255;0;0mB\u{1b}[48;5;196mC\u{1b}[0m"
        );
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
    fn normalize_colon_delimited_sgr_sequences_rewrites_linux_style_colors() {
        assert_eq!(
            normalize_colon_delimited_sgr_sequences("\u{1b}[1;38:2::255:0:0mboom\u{1b}[0m"),
            "\u{1b}[1;38;2;255;0;0mboom\u{1b}[0m"
        );
        assert_eq!(
            normalize_colon_delimited_sgr_sequences("\u{1b}[48:5:196m!\u{1b}[0m"),
            "\u{1b}[48;5;196m!\u{1b}[0m"
        );
    }
}
