use std::io::Write;
use std::process::{Command, Stdio};

use arboard::Clipboard;

pub(in crate::ui::tui) trait ClipboardAccess {
    fn read_text(&mut self) -> Result<String, String>;
    fn write_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Default)]
pub(in crate::ui::tui) struct SystemClipboardAccess {
    clipboard: Option<Clipboard>,
}

impl SystemClipboardAccess {
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
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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
        match Self::write_text_with_platform_command(text) {
            Ok(()) => Ok(()),
            Err(command_error) => self
                .clipboard()?
                .set_text(text.to_string())
                .map_err(|error| format!("{command_error}; arboard: {error}")),
        }
    }
}
