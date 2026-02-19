use std::io::Write;

use crate::infrastructure::process::{
    execute_command as execute_process_command, stderr_or_status, stderr_trimmed,
};

pub(in crate::ui::tui) trait TmuxInput {
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

pub(in crate::ui::tui) struct CommandTmuxInput;

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
        true
    }
}

impl CommandTmuxInput {
    pub(in crate::ui::tui) fn execute_command(command: &[String]) -> std::io::Result<()> {
        execute_process_command(command)
    }

    pub(in crate::ui::tui) fn capture_session_output(
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
            let stderr = stderr_trimmed(&output);
            return Err(std::io::Error::other(format!(
                "tmux capture-pane failed for '{target_session}': {stderr}"
            )));
        }

        String::from_utf8(output.stdout).map_err(|error| {
            std::io::Error::other(format!("tmux output utf8 decode failed: {error}"))
        })
    }

    pub(in crate::ui::tui) fn capture_session_cursor_metadata(
        target_session: &str,
    ) -> std::io::Result<String> {
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
            let stderr = stderr_trimmed(&output);
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
            Ok(output) => Some(stderr_or_status(&output)),
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
            stderr_or_status(&paste_output),
        )))
    }
}
