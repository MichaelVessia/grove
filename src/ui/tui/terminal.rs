use std::io::Write;
use std::process::{Command, Stdio};

use arboard::Clipboard;

use super::CursorMetadata;

pub(super) trait TmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()>;
    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String>;
    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String>;
    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()>;
    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()>;

    fn supports_background_send(&self) -> bool {
        false
    }

    fn supports_background_poll(&self) -> bool {
        false
    }

    fn supports_background_launch(&self) -> bool {
        false
    }
}

pub(super) trait ClipboardAccess {
    fn read_text(&mut self) -> Result<String, String>;
    fn write_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Default)]
pub(super) struct SystemClipboardAccess {
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
            return Self::run_write_command("pbcopy", &[], text);
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
            return Self::run_read_command("pbpaste", &[]);
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

pub(super) struct CommandTmuxInput;

impl TmuxInput for CommandTmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()> {
        Self::execute_command(command)
    }

    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        Self::capture_session_output(target_session, scrollback_lines, include_escape_sequences)
    }

    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
        Self::capture_session_cursor_metadata(target_session)
    }

    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        Self::resize_target_session(target_session, target_width, target_height)
    }

    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
        Self::paste_target_session_buffer(target_session, text)
    }

    fn supports_background_send(&self) -> bool {
        true
    }

    fn supports_background_poll(&self) -> bool {
        true
    }

    fn supports_background_launch(&self) -> bool {
        false
    }
}

impl CommandTmuxInput {
    fn stderr_or_status(output: &std::process::Output) -> String {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return stderr;
        }

        format!("exit status {}", output.status)
    }

    pub(super) fn execute_command(command: &[String]) -> std::io::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let output = std::process::Command::new(&command[0])
            .args(&command[1..])
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "command failed: {}; {}",
            command.join(" "),
            Self::stderr_or_status(&output),
        )))
    }

    pub(super) fn capture_session_output(
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        let mut args = vec!["capture-pane".to_string(), "-p".to_string()];
        if include_escape_sequences {
            args.push("-e".to_string());
        }
        args.push("-t".to_string());
        args.push(target_session.to_string());
        args.push("-S".to_string());
        args.push(format!("-{scrollback_lines}"));

        let output = std::process::Command::new("tmux").args(args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(std::io::Error::other(format!(
                "tmux capture-pane failed for '{target_session}': {stderr}"
            )));
        }

        String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("tmux output utf8 decode failed: {error}"))
        })
    }

    pub(super) fn capture_session_cursor_metadata(target_session: &str) -> std::io::Result<String> {
        let output = std::process::Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                target_session,
                "#{cursor_flag} #{cursor_x} #{cursor_y} #{pane_width} #{pane_height}",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(std::io::Error::other(format!(
                "tmux cursor metadata failed for '{target_session}': {stderr}"
            )));
        }

        String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("tmux cursor metadata utf8 decode failed: {error}"))
        })
    }

    fn resize_target_session(
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        if target_width == 0 || target_height == 0 {
            return Ok(());
        }

        let width = target_width.to_string();
        let height = target_height.to_string();

        let set_manual_output = std::process::Command::new("tmux")
            .args(["set-option", "-t", target_session, "window-size", "manual"])
            .output();
        let set_manual_error = match set_manual_output {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(Self::stderr_or_status(&output)),
            Err(error) => Some(error.to_string()),
        };

        let resize_window = std::process::Command::new("tmux")
            .args([
                "resize-window",
                "-t",
                target_session,
                "-x",
                &width,
                "-y",
                &height,
            ])
            .output()?;
        if resize_window.status.success() {
            return Ok(());
        }

        let resize_pane = std::process::Command::new("tmux")
            .args([
                "resize-pane",
                "-t",
                target_session,
                "-x",
                &width,
                "-y",
                &height,
            ])
            .output()?;
        if resize_pane.status.success() {
            return Ok(());
        }

        let resize_window_error = String::from_utf8_lossy(&resize_window.stderr)
            .trim()
            .to_string();
        let resize_pane_error = String::from_utf8_lossy(&resize_pane.stderr)
            .trim()
            .to_string();
        let set_manual_suffix =
            set_manual_error.map_or_else(String::new, |error| format!("; set-option={error}"));
        Err(std::io::Error::other(format!(
            "tmux resize failed for '{target_session}': resize-window={resize_window_error}; resize-pane={resize_pane_error}{set_manual_suffix}"
        )))
    }

    fn paste_target_session_buffer(target_session: &str, text: &str) -> std::io::Result<()> {
        let mut load_buffer = std::process::Command::new("tmux");
        load_buffer.arg("load-buffer").arg("-");
        load_buffer.stdin(std::process::Stdio::piped());
        let mut load_child = load_buffer.spawn()?;
        if let Some(stdin) = load_child.stdin.as_mut() {
            stdin.write_all(text.as_bytes())?;
        }
        let load_status = load_child.wait()?;
        if !load_status.success() {
            return Err(std::io::Error::other(format!(
                "tmux load-buffer failed for '{target_session}': exit status {load_status}"
            )));
        }

        let paste_output = std::process::Command::new("tmux")
            .args(["paste-buffer", "-t", target_session])
            .output()?;
        if paste_output.status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "tmux paste-buffer failed: {}",
            Self::stderr_or_status(&paste_output),
        )))
    }
}

fn parse_cursor_flag(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "on" | "true" => Some(true),
        "0" | "off" | "false" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_cursor_metadata(raw: &str) -> Option<CursorMetadata> {
    let mut fields = raw.split_whitespace();
    let cursor_visible = parse_cursor_flag(fields.next()?)?;
    let cursor_col = fields.next()?.parse::<u16>().ok()?;
    let cursor_row = fields.next()?.parse::<u16>().ok()?;
    let pane_width = fields.next()?.parse::<u16>().ok()?;
    let pane_height = fields.next()?.parse::<u16>().ok()?;
    if fields.next().is_some() {
        return None;
    }

    Some(CursorMetadata {
        cursor_visible,
        cursor_col,
        cursor_row,
        pane_width,
        pane_height,
    })
}
