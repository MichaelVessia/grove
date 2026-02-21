use crate::infrastructure::tmux as shared_tmux;

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
        shared_tmux::resize_session(target_session, target_width, target_height)
    }

    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
        shared_tmux::paste_buffer(target_session, text)
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
        shared_tmux::execute_command(command)
    }

    pub(in crate::ui::tui) fn capture_session_output(
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        shared_tmux::capture_session_output(
            target_session,
            scrollback_lines,
            include_escape_sequences,
        )
    }

    pub(in crate::ui::tui) fn capture_session_cursor_metadata(
        target_session: &str,
    ) -> std::io::Result<String> {
        shared_tmux::capture_cursor_metadata(target_session)
    }
}
