use std::io::Write;
use std::process::{Command, Stdio};

use arboard::Clipboard;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

use crate::infrastructure::process::stderr_trimmed;

pub(in crate::ui::tui) trait ClipboardAccess {
    fn read_text(&mut self) -> Result<String, String>;
    fn write_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Default)]
pub(in crate::ui::tui) struct SystemClipboardAccess {
    clipboard: Option<Clipboard>,
}

impl SystemClipboardAccess {
    fn osc52_sequence(text: &str, inside_tmux: bool) -> Vec<u8> {
        let payload = BASE64_STANDARD.encode(text.as_bytes());
        if inside_tmux {
            format!("\x1bPtmux;\x1b\x1b]52;c;{payload}\x07\x1b\\").into_bytes()
        } else {
            format!("\x1b]52;c;{payload}\x07").into_bytes()
        }
    }

    fn combine_write_results(
        platform_result: Result<(), String>,
        arboard_result: Result<(), String>,
        osc52_result: Result<(), String>,
    ) -> Result<(), String> {
        if platform_result.is_ok() || arboard_result.is_ok() || osc52_result.is_ok() {
            return Ok(());
        }

        let mut errors = Vec::new();
        if let Err(error) = platform_result {
            errors.push(error);
        }
        if let Err(error) = arboard_result {
            errors.push(format!("arboard: {error}"));
        }
        if let Err(error) = osc52_result {
            errors.push(format!("osc52: {error}"));
        }
        Err(errors.join("; "))
    }

    fn write_text_with_arboard(&mut self, text: &str) -> Result<(), String> {
        self.clipboard()?
            .set_text(text.to_string())
            .map_err(|error| error.to_string())
    }

    fn write_osc52_to(
        writer: &mut impl Write,
        text: &str,
        inside_tmux: bool,
    ) -> Result<(), String> {
        writer
            .write_all(Self::osc52_sequence(text, inside_tmux).as_slice())
            .map_err(|error| format!("failed to write OSC52 sequence: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("failed to flush OSC52 sequence: {error}"))
    }

    fn write_osc52(text: &str) -> Result<(), String> {
        let mut stdout = std::io::stdout();
        Self::write_osc52_to(&mut stdout, text, std::env::var_os("TMUX").is_some())
    }

    fn clipboard(&mut self) -> Result<&mut Clipboard, String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new().map_err(|error| error.to_string())?);
        }

        self.clipboard
            .as_mut()
            .ok_or_else(|| "clipboard unavailable".to_string())
    }

    fn run_write_command(program: &str, args: &[&str], text: &str) -> Result<(), String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("{program}: {error}"))?;

        let Some(mut stdin) = child.stdin.take() else {
            return Err(format!("{program}: failed to open stdin"));
        };
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("{program}: {error}"))?;
        drop(stdin);

        let status = child
            .wait()
            .map_err(|error| format!("{program}: {error}"))?;
        if status.success() {
            return Ok(());
        }

        Err(format!("{program}: exited with status {status}"))
    }

    fn run_read_command(program: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|error| format!("{program}: {error}"))?;
        if !output.status.success() {
            let stderr = stderr_trimmed(&output);
            if stderr.is_empty() {
                return Err(format!("{program}: exited with status {}", output.status));
            }
            return Err(format!("{program}: {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn write_text_with_platform_command(text: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            Self::run_write_command("pbcopy", &[], text)
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
            let candidates: &[(&str, &[&str])] = if wayland {
                &[
                    ("wl-copy", &[]),
                    ("xclip", &["-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--input"]),
                ]
            } else {
                &[
                    ("xclip", &["-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--input"]),
                    ("wl-copy", &[]),
                ]
            };

            let mut errors: Vec<String> = Vec::new();
            for (program, args) in candidates {
                match Self::run_write_command(program, args, text) {
                    Ok(()) => return Ok(()),
                    Err(error) => errors.push(error),
                }
            }

            Err(errors.join("; "))
        }

        #[cfg(not(any(target_os = "macos", unix)))]
        {
            Err("platform clipboard command unavailable".to_string())
        }
    }

    fn read_text_with_platform_command() -> Result<String, String> {
        #[cfg(target_os = "macos")]
        {
            Self::run_read_command("pbpaste", &[])
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
            let candidates: &[(&str, &[&str])] = if wayland {
                &[
                    ("wl-paste", &["--no-newline"]),
                    ("xclip", &["-o", "-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--output"]),
                ]
            } else {
                &[
                    ("xclip", &["-o", "-selection", "clipboard"]),
                    ("xsel", &["--clipboard", "--output"]),
                    ("wl-paste", &["--no-newline"]),
                ]
            };

            let mut errors: Vec<String> = Vec::new();
            for (program, args) in candidates {
                match Self::run_read_command(program, args) {
                    Ok(text) => return Ok(text),
                    Err(error) => errors.push(error),
                }
            }

            Err(errors.join("; "))
        }

        #[cfg(not(any(target_os = "macos", unix)))]
        {
            Err("platform clipboard command unavailable".to_string())
        }
    }
}

impl ClipboardAccess for SystemClipboardAccess {
    fn read_text(&mut self) -> Result<String, String> {
        match Self::read_text_with_platform_command() {
            Ok(text) => Ok(text),
            Err(command_error) => self
                .clipboard()?
                .get_text()
                .map_err(|error| format!("{command_error}; arboard: {error}")),
        }
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        let platform_result = Self::write_text_with_platform_command(text);
        let arboard_result = match &platform_result {
            Ok(()) => Ok(()),
            Err(_) => self.write_text_with_arboard(text),
        };
        let osc52_result = Self::write_osc52(text);

        Self::combine_write_results(platform_result, arboard_result, osc52_result)
    }
}

#[cfg(test)]
mod tests {
    use super::SystemClipboardAccess;

    #[test]
    fn osc52_sequence_formats_plain_payload() {
        let rendered = SystemClipboardAccess::osc52_sequence("hello", false);

        assert_eq!(rendered, b"\x1b]52;c;aGVsbG8=\x07");
    }

    #[test]
    fn osc52_sequence_wraps_tmux_passthrough() {
        let rendered = SystemClipboardAccess::osc52_sequence("hello", true);

        assert_eq!(rendered, b"\x1bPtmux;\x1b\x1b]52;c;aGVsbG8=\x07\x1b\\");
    }

    #[test]
    fn clipboard_write_succeeds_when_only_osc52_succeeds() {
        let result = SystemClipboardAccess::combine_write_results(
            Err("platform failed".to_string()),
            Err("arboard failed".to_string()),
            Ok(()),
        );

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn clipboard_write_returns_all_errors_when_every_path_fails() {
        let result = SystemClipboardAccess::combine_write_results(
            Err("platform failed".to_string()),
            Err("arboard failed".to_string()),
            Err("osc52 failed".to_string()),
        );

        assert_eq!(
            result,
            Err("platform failed; arboard: arboard failed; osc52: osc52 failed".to_string())
        );
    }

    #[test]
    fn write_osc52_to_writes_unicode_payload() {
        let mut output = Vec::new();

        SystemClipboardAccess::write_osc52_to(&mut output, "😀", false)
            .expect("osc52 write should succeed");

        assert_eq!(output, b"\x1b]52;c;8J+YgA==\x07");
    }
}
