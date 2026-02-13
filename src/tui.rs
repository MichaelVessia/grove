use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ftui::core::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    PasteEvent,
};
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::{App, Cmd, Model, ScreenMode};

use crate::adapters::{
    BootstrapData, CommandGitAdapter, CommandSystemAdapter, CommandTmuxAdapter, DiscoveryState,
    bootstrap_data,
};
use crate::agent_runtime::{
    LaunchRequest, build_launch_plan, poll_interval, session_name_for_workspace, stop_plan,
};
use crate::domain::WorkspaceStatus;
use crate::interactive::{
    InteractiveAction, InteractiveKey, InteractiveState, encode_paste_payload,
    render_cursor_overlay, tmux_send_keys_command,
};
use crate::mouse::{
    HitRegion, LayoutMetrics, clamp_sidebar_ratio, hit_test, parse_sidebar_ratio, ratio_from_drag,
    serialize_sidebar_ratio,
};
use crate::preview::PreviewState;
use crate::state::{Action, AppState, PaneFocus, UiMode, reduce};

const DEFAULT_SIDEBAR_WIDTH_PCT: u16 = 33;
const SIDEBAR_RATIO_FILENAME: &str = ".grove-sidebar-width";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorMetadata {
    cursor_visible: bool,
    cursor_col: u16,
    cursor_row: u16,
    pane_width: u16,
    pane_height: u16,
}

trait TmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()>;
    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
    ) -> std::io::Result<String>;
    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Msg {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(PasteEvent),
    Tick,
    Resize { width: u16, height: u16 },
    Noop,
}

struct CommandTmuxInput;

impl TmuxInput for CommandTmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let status = std::process::Command::new(&command[0])
            .args(&command[1..])
            .status()?;

        if status.success() {
            return Ok(());
        }

        Err(std::io::Error::other(format!(
            "tmux command failed: {}",
            command.join(" ")
        )))
    }

    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
    ) -> std::io::Result<String> {
        let output = std::process::Command::new("tmux")
            .args([
                "capture-pane",
                "-p",
                "-e",
                "-t",
                target_session,
                "-S",
                &format!("-{scrollback_lines}"),
            ])
            .output()?;

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

    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
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
}

fn parse_cursor_flag(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "on" | "true" => Some(true),
        "0" | "off" | "false" => Some(false),
        _ => None,
    }
}

fn parse_cursor_metadata(raw: &str) -> Option<CursorMetadata> {
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

fn default_sidebar_ratio_path() -> PathBuf {
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(SIDEBAR_RATIO_FILENAME),
        Err(_) => PathBuf::from(SIDEBAR_RATIO_FILENAME),
    }
}

fn load_sidebar_ratio(path: &Path) -> u16 {
    let Ok(raw) = fs::read_to_string(path) else {
        return DEFAULT_SIDEBAR_WIDTH_PCT;
    };

    parse_sidebar_ratio(&raw).unwrap_or(DEFAULT_SIDEBAR_WIDTH_PCT)
}

fn persist_sidebar_ratio(path: &Path, ratio_pct: u16) -> std::io::Result<()> {
    fs::write(path, serialize_sidebar_ratio(ratio_pct))
}

fn write_launcher_script(path: &Path, contents: &str) -> std::io::Result<()> {
    fs::write(path, contents)
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key_event) => Self::Key(key_event),
            Event::Mouse(mouse_event) => Self::Mouse(mouse_event),
            Event::Paste(paste_event) => Self::Paste(paste_event),
            Event::Tick => Self::Tick,
            Event::Resize { width, height } => Self::Resize { width, height },
            _ => Self::Noop,
        }
    }
}

struct GroveApp {
    repo_name: String,
    state: AppState,
    discovery_state: DiscoveryState,
    preview: PreviewState,
    interactive: Option<InteractiveState>,
    tmux_input: Box<dyn TmuxInput>,
    last_tmux_error: Option<String>,
    output_changing: bool,
    viewport_width: u16,
    viewport_height: u16,
    sidebar_width_pct: u16,
    sidebar_ratio_path: PathBuf,
    divider_drag_active: bool,
    copied_text: Option<String>,
}

impl GroveApp {
    fn new() -> Self {
        let bootstrap = bootstrap_data(
            &CommandGitAdapter,
            &CommandTmuxAdapter,
            &CommandSystemAdapter,
        );
        Self::from_bootstrap(bootstrap)
    }

    fn from_bootstrap(bootstrap: BootstrapData) -> Self {
        Self::from_bootstrap_with_tmux(bootstrap, Box::new(CommandTmuxInput))
    }

    fn from_bootstrap_with_tmux(bootstrap: BootstrapData, tmux_input: Box<dyn TmuxInput>) -> Self {
        Self::from_bootstrap_with_tmux_and_sidebar_path(
            bootstrap,
            tmux_input,
            default_sidebar_ratio_path(),
        )
    }

    fn from_bootstrap_with_tmux_and_sidebar_path(
        bootstrap: BootstrapData,
        tmux_input: Box<dyn TmuxInput>,
        sidebar_ratio_path: PathBuf,
    ) -> Self {
        let sidebar_width_pct = load_sidebar_ratio(&sidebar_ratio_path);
        let mut app = Self {
            repo_name: bootstrap.repo_name,
            state: AppState::new(bootstrap.workspaces),
            discovery_state: bootstrap.discovery_state,
            preview: PreviewState::new(),
            interactive: None,
            tmux_input,
            last_tmux_error: None,
            output_changing: false,
            viewport_width: 120,
            viewport_height: 40,
            sidebar_width_pct,
            sidebar_ratio_path,
            divider_drag_active: false,
            copied_text: None,
        };
        app.refresh_preview_summary();
        app
    }

    fn mode_label(&self) -> &'static str {
        if self.interactive.is_some() {
            return "Interactive";
        }

        match self.state.mode {
            UiMode::List => "List",
            UiMode::Preview => "Preview",
        }
    }

    fn focus_label(&self) -> &'static str {
        match self.state.focus {
            PaneFocus::WorkspaceList => "WorkspaceList",
            PaneFocus::Preview => "Preview",
        }
    }

    fn selected_status_hint(&self) -> &'static str {
        match self
            .state
            .selected_workspace()
            .map(|workspace| workspace.status)
        {
            Some(WorkspaceStatus::Main) => "main worktree",
            Some(WorkspaceStatus::Idle) => "idle",
            Some(WorkspaceStatus::Active) => "active",
            Some(WorkspaceStatus::Thinking) => "thinking",
            Some(WorkspaceStatus::Waiting) => "waiting",
            Some(WorkspaceStatus::Done) => "done",
            Some(WorkspaceStatus::Error) => "error",
            Some(WorkspaceStatus::Unsupported) => "unsupported",
            Some(WorkspaceStatus::Unknown) => "unknown",
            None => "none",
        }
    }

    fn status_bar_line(&self) -> String {
        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                format!("Status: discovery error ({message}) [q]quit")
            }
            DiscoveryState::Empty => "Status: no worktrees found [q]quit".to_string(),
            DiscoveryState::Ready => {
                if self.interactive.is_some() {
                    if let Some(message) = &self.last_tmux_error {
                        return format!(
                            "Status: -- INSERT -- [Esc Esc]exit [Ctrl+\\]exit | tmux error: {message}"
                        );
                    }
                    return "Status: -- INSERT -- [Esc Esc]exit [Ctrl+\\]exit".to_string();
                }

                match self.state.mode {
                    UiMode::List => format!(
                        "Status: [j/k]move [Tab]focus [Enter]preview-or-interactive [s]start [x]stop [q]quit | [mouse]click/drag/scroll | selected={}",
                        self.selected_status_hint()
                    ),
                    UiMode::Preview => format!(
                        "Status: [j/k]scroll [PgUp/PgDn]scroll [G]bottom [Esc]list [Tab]focus [s]start [x]stop [q]quit | [mouse]scroll/drag divider | autoscroll={} offset={} split={}%%",
                        if self.preview.auto_scroll {
                            "on"
                        } else {
                            "off"
                        },
                        self.preview.offset,
                        self.sidebar_width_pct,
                    ),
                }
            }
        }
    }

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "Workspace: {}\nBranch: {}\nPath: {}\nAgent: {}\nStatus: {}\nOrphaned session: {}",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display(),
                    workspace.agent.label(),
                    self.selected_status_hint(),
                    if workspace.is_orphaned { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }

    fn selected_session_for_live_preview(&self) -> Option<String> {
        let workspace = self.state.selected_workspace()?;
        if workspace.is_main {
            return None;
        }

        if matches!(
            workspace.status,
            WorkspaceStatus::Active
                | WorkspaceStatus::Thinking
                | WorkspaceStatus::Waiting
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
        ) {
            return Some(session_name_for_workspace(&workspace.name));
        }

        None
    }

    fn interactive_target_session(&self) -> Option<String> {
        self.interactive
            .as_ref()
            .map(|state| state.target_session.clone())
    }

    fn poll_interactive_cursor(&mut self, target_session: &str) {
        let Ok(raw_metadata) = self.tmux_input.capture_cursor_metadata(target_session) else {
            return;
        };
        let Some(metadata) = parse_cursor_metadata(&raw_metadata) else {
            return;
        };
        let Some(state) = self.interactive.as_mut() else {
            return;
        };

        state.update_cursor(
            metadata.cursor_row,
            metadata.cursor_col,
            metadata.cursor_visible,
            metadata.pane_height,
            metadata.pane_width,
        );
    }

    fn poll_preview(&mut self) {
        let Some(session_name) = self.selected_session_for_live_preview() else {
            self.output_changing = false;
            self.refresh_preview_summary();
            if let Some(target_session) = self.interactive_target_session() {
                self.poll_interactive_cursor(&target_session);
            }
            return;
        };

        match self.tmux_input.capture_output(&session_name, 600) {
            Ok(output) => {
                let update = self.preview.apply_capture(&output);
                self.output_changing = update.changed_cleaned;
                self.last_tmux_error = None;
            }
            Err(error) => {
                self.output_changing = false;
                self.last_tmux_error = Some(error.to_string());
                self.refresh_preview_summary();
            }
        }

        if let Some(target_session) = self.interactive_target_session() {
            self.poll_interactive_cursor(&target_session);
        }
    }

    fn selected_preview_height(&self, total_height: usize) -> usize {
        let workspace_rows = match self.discovery_state {
            DiscoveryState::Ready => self.state.workspaces.len(),
            _ => 1,
        };
        let reserved = 3 + workspace_rows + 2 + 2;
        total_height.saturating_sub(reserved).max(1)
    }

    fn scroll_preview(&mut self, delta: i32) {
        let _ = self.preview.scroll(delta, Instant::now());
    }

    fn persist_sidebar_ratio(&mut self) {
        if let Err(error) = persist_sidebar_ratio(&self.sidebar_ratio_path, self.sidebar_width_pct)
        {
            self.last_tmux_error = Some(format!("sidebar ratio persist failed: {error}"));
        }
    }

    fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.preview.reset_for_selection_change();
            self.poll_preview();
        }
    }

    fn is_quit_key(key_event: &KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Char('q')
                if key_event.kind == KeyEventKind::Press && key_event.modifiers.is_empty() =>
            {
                true
            }
            KeyCode::Char('c')
                if key_event.kind == KeyEventKind::Press
                    && key_event.modifiers.contains(Modifiers::CTRL) =>
            {
                true
            }
            _ => false,
        }
    }

    fn can_enter_interactive(&self) -> bool {
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };

        if workspace.is_main {
            return false;
        }

        matches!(
            workspace.status,
            WorkspaceStatus::Active
                | WorkspaceStatus::Thinking
                | WorkspaceStatus::Waiting
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
        )
    }

    fn enter_interactive(&mut self, now: Instant) -> bool {
        if !self.can_enter_interactive() {
            return false;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };

        let session_name = session_name_for_workspace(&workspace.name);
        self.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            session_name,
            now,
            self.viewport_height,
            self.viewport_width,
        ));
        self.last_tmux_error = None;
        self.state.mode = UiMode::Preview;
        self.state.focus = PaneFocus::Preview;
        true
    }

    fn can_start_selected_workspace(&self) -> bool {
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        if workspace.is_main || !workspace.supported_agent {
            return false;
        }

        matches!(
            workspace.status,
            WorkspaceStatus::Idle
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
                | WorkspaceStatus::Unknown
        )
    }

    fn start_selected_workspace_agent(&mut self) {
        let Some(workspace) = self.state.selected_workspace() else {
            return;
        };
        if !self.can_start_selected_workspace() {
            return;
        }

        let request = LaunchRequest {
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            agent: workspace.agent,
            prompt: None,
            skip_permissions: false,
        };
        let launch_plan = build_launch_plan(&request);

        if let Some(script) = &launch_plan.launcher_script
            && let Err(error) = write_launcher_script(&script.path, &script.contents)
        {
            self.last_tmux_error = Some(format!("launcher script write failed: {error}"));
            return;
        }

        for command in &launch_plan.pre_launch_cmds {
            if let Err(error) = self.tmux_input.execute(command) {
                self.last_tmux_error = Some(error.to_string());
                return;
            }
        }

        if let Err(error) = self.tmux_input.execute(&launch_plan.launch_cmd) {
            self.last_tmux_error = Some(error.to_string());
            return;
        }

        if let Some(selected) = self.state.selected_workspace_mut() {
            selected.status = WorkspaceStatus::Active;
            selected.is_orphaned = false;
        }
        self.last_tmux_error = None;
        self.poll_preview();
    }

    fn can_stop_selected_workspace(&self) -> bool {
        let Some(workspace) = self.state.selected_workspace() else {
            return false;
        };
        if workspace.is_main {
            return false;
        }

        matches!(
            workspace.status,
            WorkspaceStatus::Active
                | WorkspaceStatus::Thinking
                | WorkspaceStatus::Waiting
                | WorkspaceStatus::Done
                | WorkspaceStatus::Error
        )
    }

    fn stop_selected_workspace_agent(&mut self) {
        if !self.can_stop_selected_workspace() {
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            return;
        };
        let session_name = session_name_for_workspace(&workspace.name);
        let stop_commands = stop_plan(&session_name);
        for command in &stop_commands {
            if let Err(error) = self.tmux_input.execute(command) {
                self.last_tmux_error = Some(error.to_string());
                return;
            }
        }

        if self
            .interactive
            .as_ref()
            .is_some_and(|state| state.target_session == session_name)
        {
            self.interactive = None;
        }

        if let Some(selected) = self.state.selected_workspace_mut() {
            selected.status = WorkspaceStatus::Idle;
            selected.is_orphaned = false;
        }
        self.last_tmux_error = None;
        self.poll_preview();
    }

    fn map_interactive_key(key_event: KeyEvent) -> Option<InteractiveKey> {
        let ctrl = key_event.modifiers.contains(Modifiers::CTRL);
        let alt = key_event.modifiers.contains(Modifiers::ALT);

        match key_event.code {
            KeyCode::Enter => Some(InteractiveKey::Enter),
            KeyCode::Tab => Some(InteractiveKey::Tab),
            KeyCode::Backspace => Some(InteractiveKey::Backspace),
            KeyCode::Delete => Some(InteractiveKey::Delete),
            KeyCode::Up => Some(InteractiveKey::Up),
            KeyCode::Down => Some(InteractiveKey::Down),
            KeyCode::Left => Some(InteractiveKey::Left),
            KeyCode::Right => Some(InteractiveKey::Right),
            KeyCode::Home => Some(InteractiveKey::Home),
            KeyCode::End => Some(InteractiveKey::End),
            KeyCode::PageUp => Some(InteractiveKey::PageUp),
            KeyCode::PageDown => Some(InteractiveKey::PageDown),
            KeyCode::Escape => Some(InteractiveKey::Escape),
            KeyCode::F(index) => Some(InteractiveKey::Function(index)),
            KeyCode::Char(character) => {
                if ctrl && character == '\\' {
                    return Some(InteractiveKey::CtrlBackslash);
                }
                if alt && matches!(character, 'c' | 'C') {
                    return Some(InteractiveKey::AltC);
                }
                if alt && matches!(character, 'v' | 'V') {
                    return Some(InteractiveKey::AltV);
                }
                if ctrl {
                    return Some(InteractiveKey::Ctrl(character));
                }
                Some(InteractiveKey::Char(character))
            }
            _ => None,
        }
    }

    fn send_interactive_action(&mut self, action: &InteractiveAction, target_session: &str) {
        let Some(command) = tmux_send_keys_command(target_session, action) else {
            return;
        };

        match self.tmux_input.execute(&command) {
            Ok(()) => {
                self.last_tmux_error = None;
            }
            Err(error) => {
                self.last_tmux_error = Some(error.to_string());
            }
        }
    }

    fn copy_interactive_capture(&mut self, target_session: &str) {
        match self.tmux_input.capture_output(target_session, 200) {
            Ok(output) => {
                self.copied_text = Some(output);
                self.last_tmux_error = None;
            }
            Err(error) => {
                self.last_tmux_error = Some(error.to_string());
            }
        }
    }

    fn paste_cached_text(&mut self, target_session: &str, bracketed_paste: bool) {
        let Some(text) = self.copied_text.clone() else {
            self.last_tmux_error = Some("no copied text in session".to_string());
            return;
        };

        let payload = encode_paste_payload(&text, bracketed_paste);
        self.send_interactive_action(&InteractiveAction::SendLiteral(payload), target_session);
    }

    fn handle_interactive_key(&mut self, key_event: KeyEvent) {
        let Some(interactive_key) = Self::map_interactive_key(key_event) else {
            return;
        };

        let now = Instant::now();
        let (action, target_session, bracketed_paste) = {
            let Some(state) = self.interactive.as_mut() else {
                return;
            };
            let action = state.handle_key(interactive_key, now);
            let session = state.target_session.clone();
            let bracketed_paste = state.bracketed_paste;
            (action, session, bracketed_paste)
        };

        match action {
            InteractiveAction::ExitInteractive => {
                self.interactive = None;
                self.state.mode = UiMode::Preview;
                self.state.focus = PaneFocus::Preview;
            }
            InteractiveAction::CopySelection => self.copy_interactive_capture(&target_session),
            InteractiveAction::PasteClipboard => {
                self.paste_cached_text(&target_session, bracketed_paste)
            }
            InteractiveAction::Noop
            | InteractiveAction::SendNamed(_)
            | InteractiveAction::SendLiteral(_) => {
                self.send_interactive_action(&action, &target_session);
            }
        }
    }

    fn handle_paste_event(&mut self, paste_event: PasteEvent) {
        let (target_session, bracketed) = {
            let Some(state) = self.interactive.as_mut() else {
                return;
            };
            state.bracketed_paste = paste_event.bracketed;
            (state.target_session.clone(), state.bracketed_paste)
        };

        let payload = encode_paste_payload(&paste_event.text, bracketed || paste_event.bracketed);
        self.send_interactive_action(&InteractiveAction::SendLiteral(payload), &target_session);
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => reduce(&mut self.state, Action::ToggleFocus),
            KeyCode::Enter => {
                if !self.enter_interactive(Instant::now()) {
                    reduce(&mut self.state, Action::EnterPreviewMode);
                    self.poll_preview();
                }
            }
            KeyCode::Escape => reduce(&mut self.state, Action::EnterListMode),
            KeyCode::Char('s') => self.start_selected_workspace_agent(),
            KeyCode::Char('x') => self.stop_selected_workspace_agent(),
            KeyCode::PageUp => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-5);
                }
            }
            KeyCode::PageDown => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(5);
                }
            }
            KeyCode::Char('G') => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.preview.jump_to_bottom();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(1);
                } else {
                    self.move_selection(Action::MoveSelectionDown);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-1);
                } else {
                    self.move_selection(Action::MoveSelectionUp);
                }
            }
            _ => {}
        }
    }

    fn layout_metrics(&self) -> LayoutMetrics {
        LayoutMetrics {
            total_width: self.viewport_width,
            total_height: self.viewport_height,
            sidebar_width_pct: self.sidebar_width_pct,
            status_line_height: 1,
        }
    }

    fn apply_interactive_cursor_overlay(
        &self,
        visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some(interactive) = self.interactive.as_ref() else {
            return;
        };
        if self.preview.lines.is_empty() {
            return;
        }

        let pane_height = usize::from(interactive.pane_height.max(1));
        let cursor_row = usize::from(interactive.cursor_row);
        if cursor_row >= pane_height {
            return;
        }

        let preview_len = self.preview.lines.len();
        let pane_start = preview_len.saturating_sub(pane_height);
        let cursor_line = pane_start.saturating_add(cursor_row);
        if cursor_line >= preview_len {
            return;
        }

        let end = preview_len.saturating_sub(self.preview.offset);
        let start = end.saturating_sub(preview_height);
        if cursor_line < start || cursor_line >= end {
            return;
        }

        let visible_index = cursor_line - start;
        let Some(line) = visible_lines.get_mut(visible_index) else {
            return;
        };

        *line = render_cursor_overlay(
            line,
            usize::from(interactive.cursor_col),
            interactive.cursor_visible,
        );
    }

    fn select_workspace_by_mouse(&mut self, y: u16) {
        if !matches!(self.discovery_state, DiscoveryState::Ready) {
            return;
        }

        const LIST_START_ROW: u16 = 3;
        if y < LIST_START_ROW {
            return;
        }

        let row = usize::from(y - LIST_START_ROW);
        if row >= self.state.workspaces.len() {
            return;
        }

        if row != self.state.selected_index {
            self.state.selected_index = row;
            self.preview.reset_for_selection_change();
            self.poll_preview();
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let region = hit_test(self.layout_metrics(), mouse_event.x, mouse_event.y);

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => match region {
                HitRegion::Divider => {
                    self.divider_drag_active = true;
                }
                HitRegion::WorkspaceList => {
                    self.state.focus = PaneFocus::WorkspaceList;
                    self.state.mode = UiMode::List;
                    self.select_workspace_by_mouse(mouse_event.y);
                }
                HitRegion::Preview => {
                    self.state.focus = PaneFocus::Preview;
                    self.state.mode = UiMode::Preview;
                }
                HitRegion::StatusLine | HitRegion::Outside => {}
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.divider_drag_active {
                    let ratio =
                        clamp_sidebar_ratio(ratio_from_drag(self.viewport_width, mouse_event.x));
                    if ratio != self.sidebar_width_pct {
                        self.sidebar_width_pct = ratio;
                        self.persist_sidebar_ratio();
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.divider_drag_active = false;
            }
            MouseEventKind::ScrollUp => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    self.scroll_preview(-1);
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    self.scroll_preview(1);
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key_event: KeyEvent) -> bool {
        if key_event.kind != KeyEventKind::Press {
            return false;
        }

        if self.interactive.is_some() {
            self.handle_interactive_key(key_event);
            return false;
        }

        if Self::is_quit_key(&key_event) {
            return true;
        }

        self.handle_non_interactive_key(key_event);
        false
    }

    fn next_poll_interval(&self) -> Duration {
        let status = self
            .state
            .selected_workspace()
            .map_or(WorkspaceStatus::Unknown, |workspace| workspace.status);

        let since_last_key = self
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            status,
            true,
            self.state.focus == PaneFocus::Preview,
            self.interactive.is_some(),
            since_last_key,
            self.output_changing,
        )
    }

    fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab focus, Enter preview, s start, x stop, Esc list, mouse enabled)"
                .to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        ">"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{} {} {} | {} | {}{}",
                        selected,
                        workspace.status.icon(),
                        workspace.name,
                        workspace.branch,
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let mut visible_lines = self.preview.visible_lines(preview_height);
        self.apply_interactive_cursor_overlay(&mut visible_lines, preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }
}

impl Model for GroveApp {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.poll_preview();
        Cmd::batch(vec![
            Cmd::tick(self.next_poll_interval()),
            Cmd::set_mouse_capture(true),
        ])
    }

    fn update(&mut self, msg: Msg) -> Cmd<Self::Message> {
        match msg {
            Msg::Tick => {
                self.poll_preview();
                Cmd::tick(self.next_poll_interval())
            }
            Msg::Key(key_event) => {
                if self.handle_key(key_event) {
                    Cmd::Quit
                } else {
                    Cmd::tick(self.next_poll_interval())
                }
            }
            Msg::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
                Cmd::tick(self.next_poll_interval())
            }
            Msg::Paste(paste_event) => {
                self.handle_paste_event(paste_event);
                Cmd::tick(self.next_poll_interval())
            }
            Msg::Resize { width, height } => {
                self.viewport_width = width;
                self.viewport_height = height;
                if let Some(state) = self.interactive.as_mut() {
                    state.update_cursor(
                        state.cursor_row,
                        state.cursor_col,
                        state.cursor_visible,
                        height,
                        width,
                    );
                }
                Cmd::None
            }
            Msg::Noop => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let preview_height = self.selected_preview_height(usize::from(frame.buffer.height()));
        let content = self.shell_lines(preview_height).join("\n");
        Paragraph::new(content).render(area, frame);
    }
}

pub fn run() -> std::io::Result<()> {
    App::new(GroveApp::new())
        .screen_mode(ScreenMode::AltScreen)
        .with_mouse()
        .run()
}

#[cfg(test)]
mod tests {
    use super::{GroveApp, Msg, TmuxInput, parse_cursor_metadata};
    use crate::adapters::{BootstrapData, DiscoveryState};
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use ftui::Cmd;
    use ftui::core::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        PasteEvent,
    };
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    type RecordedCommands = Rc<RefCell<Vec<Vec<String>>>>;
    type RecordedCaptures = Rc<RefCell<Vec<Result<String, String>>>>;
    type RecordedCalls = Rc<RefCell<Vec<String>>>;
    type FixtureApp = (
        GroveApp,
        RecordedCommands,
        RecordedCaptures,
        RecordedCaptures,
    );
    type FixtureAppWithCalls = (
        GroveApp,
        RecordedCommands,
        RecordedCaptures,
        RecordedCaptures,
        RecordedCalls,
    );

    #[derive(Clone)]
    struct RecordingTmuxInput {
        commands: RecordedCommands,
        captures: RecordedCaptures,
        cursor_captures: RecordedCaptures,
        calls: RecordedCalls,
    }

    impl TmuxInput for RecordingTmuxInput {
        fn execute(&self, command: &[String]) -> std::io::Result<()> {
            self.commands.borrow_mut().push(command.to_vec());
            self.calls
                .borrow_mut()
                .push(format!("exec:{}", command.join(" ")));
            Ok(())
        }

        fn capture_output(
            &self,
            target_session: &str,
            scrollback_lines: usize,
        ) -> std::io::Result<String> {
            self.calls
                .borrow_mut()
                .push(format!("capture:{target_session}:{scrollback_lines}"));
            let mut captures = self.captures.borrow_mut();
            if captures.is_empty() {
                return Ok(String::new());
            }

            let next = captures.remove(0);
            match next {
                Ok(output) => Ok(output),
                Err(error) => Err(std::io::Error::other(error)),
            }
        }

        fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
            self.calls
                .borrow_mut()
                .push(format!("cursor:{target_session}"));
            let mut captures = self.cursor_captures.borrow_mut();
            if captures.is_empty() {
                return Ok("1 0 0 120 40".to_string());
            }

            let next = captures.remove(0);
            match next {
                Ok(output) => Ok(output),
                Err(error) => Err(std::io::Error::other(error)),
            }
        }
    }

    fn fixture_bootstrap(status: WorkspaceStatus) -> BootstrapData {
        BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: vec![
                Workspace::try_new(
                    "grove".to_string(),
                    PathBuf::from("/repos/grove"),
                    "main".to_string(),
                    Some(1_700_000_200),
                    AgentType::Claude,
                    WorkspaceStatus::Main,
                    true,
                )
                .expect("workspace should be valid"),
                Workspace::try_new(
                    "feature-a".to_string(),
                    PathBuf::from("/repos/grove-feature-a"),
                    "feature-a".to_string(),
                    Some(1_700_000_100),
                    AgentType::Codex,
                    status,
                    false,
                )
                .expect("workspace should be valid"),
            ],
            discovery_state: DiscoveryState::Ready,
            orphaned_sessions: Vec::new(),
        }
    }

    fn fixture_app() -> GroveApp {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture");
        GroveApp::from_bootstrap_with_tmux_and_sidebar_path(
            fixture_bootstrap(WorkspaceStatus::Idle),
            Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            sidebar_ratio_path,
        )
    }

    fn unique_sidebar_ratio_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        std::env::temp_dir().join(format!(
            "grove-sidebar-width-{label}-{}-{timestamp}.txt",
            std::process::id()
        ))
    }

    fn fixture_app_with_tmux(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
    ) -> FixtureApp {
        fixture_app_with_tmux_and_sidebar_path(
            status,
            captures,
            Vec::new(),
            unique_sidebar_ratio_path("fixture-with-tmux"),
        )
    }

    fn fixture_app_with_tmux_and_sidebar_path(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
        cursor_captures: Vec<Result<String, String>>,
        sidebar_ratio_path: PathBuf,
    ) -> FixtureApp {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captures = Rc::new(RefCell::new(captures));
        let cursor_captures = Rc::new(RefCell::new(cursor_captures));
        let tmux = RecordingTmuxInput {
            commands: commands.clone(),
            captures: captures.clone(),
            cursor_captures: cursor_captures.clone(),
            calls: Rc::new(RefCell::new(Vec::new())),
        };
        (
            GroveApp::from_bootstrap_with_tmux_and_sidebar_path(
                fixture_bootstrap(status),
                Box::new(tmux),
                sidebar_ratio_path,
            ),
            commands,
            captures,
            cursor_captures,
        )
    }

    fn fixture_app_with_tmux_and_calls(
        status: WorkspaceStatus,
        captures: Vec<Result<String, String>>,
        cursor_captures: Vec<Result<String, String>>,
    ) -> FixtureAppWithCalls {
        let sidebar_ratio_path = unique_sidebar_ratio_path("fixture-with-calls");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captures = Rc::new(RefCell::new(captures));
        let cursor_captures = Rc::new(RefCell::new(cursor_captures));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let tmux = RecordingTmuxInput {
            commands: commands.clone(),
            captures: captures.clone(),
            cursor_captures: cursor_captures.clone(),
            calls: calls.clone(),
        };

        (
            GroveApp::from_bootstrap_with_tmux_and_sidebar_path(
                fixture_bootstrap(status),
                Box::new(tmux),
                sidebar_ratio_path,
            ),
            commands,
            captures,
            cursor_captures,
            calls,
        )
    }

    #[test]
    fn key_q_maps_to_key_message() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press));
        assert_eq!(
            Msg::from(event),
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press))
        );
    }

    #[test]
    fn ctrl_c_maps_to_key_message() {
        let event = Event::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert_eq!(
            Msg::from(event),
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press)
            )
        );
    }

    #[test]
    fn tick_maps_to_tick_message() {
        assert_eq!(Msg::from(Event::Tick), Msg::Tick);
    }

    #[test]
    fn key_message_updates_model_state() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(cmd, Cmd::Tick(_)));
        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn q_quits_when_not_interactive() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn start_key_launches_selected_workspace_agent() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().as_slice(),
            &[
                vec![
                    "tmux".to_string(),
                    "new-session".to_string(),
                    "-d".to_string(),
                    "-s".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "-c".to_string(),
                    "/repos/grove-feature-a".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "set-option".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "history-limit".to_string(),
                    "10000".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "codex".to_string(),
                    "Enter".to_string(),
                ],
            ]
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Active)
        );
    }

    #[test]
    fn stop_key_stops_selected_workspace_agent() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        assert_eq!(
            commands.borrow().as_slice(),
            &[
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "C-c".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                ],
            ]
        );
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Idle)
        );
    }

    #[test]
    fn start_key_ignores_main_workspace() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
        );

        assert!(commands.borrow().is_empty());
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.status),
            Some(WorkspaceStatus::Main)
        );
    }

    #[test]
    fn enter_on_active_workspace_starts_interactive_mode() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_some());
        assert_eq!(app.mode_label(), "Interactive");
    }

    #[test]
    fn interactive_keys_forward_to_tmux_session() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
        );

        assert!(matches!(cmd, Cmd::Tick(_)));
        assert_eq!(
            commands.borrow().as_slice(),
            &[vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "q".to_string(),
            ]]
        );
    }

    #[test]
    fn double_escape_exits_interactive_mode() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
        );

        assert!(app.interactive.is_none());
        assert_eq!(
            commands.borrow().as_slice(),
            &[vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "Escape".to_string(),
            ]]
        );
    }

    #[test]
    fn interactive_key_reschedules_fast_poll_interval() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );

        match cmd {
            Cmd::Tick(interval) => assert_eq!(interval, Duration::from_millis(50)),
            _ => panic!("expected Cmd::Tick from interactive key update"),
        }
    }

    #[test]
    fn interactive_update_flow_sequences_tick_copy_paste_and_exit() {
        let (mut app, _commands, _captures, _cursor_captures, calls) =
            fixture_app_with_tmux_and_calls(
                WorkspaceStatus::Active,
                vec![
                    Ok("initial-preview".to_string()),
                    Ok("preview-output".to_string()),
                    Ok("copied-text".to_string()),
                ],
                vec![Ok("1 0 0 120 40".to_string())],
            );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        calls.borrow_mut().clear();

        ftui::Model::update(&mut app, Msg::Tick);
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('v'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('\\'))
                    .with_modifiers(Modifiers::CTRL)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert_eq!(
            calls.borrow().as_slice(),
            &[
                "capture:grove-ws-feature-a:600".to_string(),
                "cursor:grove-ws-feature-a".to_string(),
                "exec:tmux send-keys -l -t grove-ws-feature-a x".to_string(),
                "capture:grove-ws-feature-a:200".to_string(),
                "exec:tmux send-keys -l -t grove-ws-feature-a copied-text".to_string(),
            ]
        );
        assert!(app.interactive.is_none());
    }

    #[test]
    fn tick_polls_live_tmux_output_into_preview() {
        let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux(
            WorkspaceStatus::Active,
            vec![
                Ok("line one\nline two\n".to_string()),
                Ok("line one\nline two\n".to_string()),
            ],
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(&mut app, Msg::Tick);

        assert_eq!(
            app.preview.lines,
            vec!["line one".to_string(), "line two".to_string()]
        );
    }

    #[test]
    fn parse_cursor_metadata_requires_five_fields() {
        assert_eq!(
            parse_cursor_metadata("1 4 2 120 40"),
            Some(super::CursorMetadata {
                cursor_visible: true,
                cursor_col: 4,
                cursor_row: 2,
                pane_width: 120,
                pane_height: 40,
            })
        );
        assert!(parse_cursor_metadata("1 4 2 120").is_none());
        assert!(parse_cursor_metadata("invalid").is_none());
    }

    #[test]
    fn tick_polls_cursor_metadata_and_renders_overlay() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("cursor-overlay");
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Active,
                vec![
                    Ok("first\nsecond\nthird\n".to_string()),
                    Ok("first\nsecond\nthird\n".to_string()),
                ],
                vec![Ok("1 1 1 120 3".to_string())],
                sidebar_ratio_path,
            );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(&mut app, Msg::Tick);

        let rendered = app.shell_lines(8).join("\n");
        assert_eq!(
            app.interactive.as_ref().map(|state| (
                state.cursor_row,
                state.cursor_col,
                state.pane_height
            )),
            Some((1, 1, 3))
        );
        assert!(rendered.contains("\u{1b}[7m"), "{rendered}");
    }

    #[test]
    fn divider_ratio_persists_across_app_instances() {
        let sidebar_ratio_path = unique_sidebar_ratio_path("persist");
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Idle,
                Vec::new(),
                Vec::new(),
                sidebar_ratio_path.clone(),
            );

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                33,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                52,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 52);
        assert_eq!(
            fs::read_to_string(&sidebar_ratio_path).expect("ratio file should be written"),
            "52"
        );

        let (app_reloaded, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux_and_sidebar_path(
                WorkspaceStatus::Idle,
                Vec::new(),
                Vec::new(),
                sidebar_ratio_path.clone(),
            );

        assert_eq!(app_reloaded.sidebar_width_pct, 52);
        let _ = fs::remove_file(sidebar_ratio_path);
    }

    #[test]
    fn mouse_click_on_list_selects_workspace() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                5,
                4,
            )),
        );

        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn mouse_drag_on_divider_updates_sidebar_ratio() {
        let mut app = fixture_app();

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                33,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                55,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 55);

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Up(MouseButton::Left),
                55,
                8,
            )),
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Drag(MouseButton::Left),
                20,
                8,
            )),
        );

        assert_eq!(app.sidebar_width_pct, 55);
    }

    #[test]
    fn mouse_scroll_in_preview_scrolls_output() {
        let mut app = fixture_app();
        app.preview.lines = (1..=30).map(|value| value.to_string()).collect();
        app.preview.offset = 0;
        app.preview.auto_scroll = true;

        ftui::Model::update(
            &mut app,
            Msg::Resize {
                width: 100,
                height: 40,
            },
        );
        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, 90, 10)),
        );

        assert!(app.preview.offset > 0);
        assert!(!app.preview.auto_scroll);
    }

    #[test]
    fn bracketed_paste_event_forwards_wrapped_literal() {
        let (mut app, commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(&mut app, Msg::Paste(PasteEvent::bracketed("hello\nworld")));

        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "\u{1b}[200~hello\nworld\u{1b}[201~".to_string(),
            ])
        );
    }

    #[test]
    fn alt_copy_then_alt_paste_uses_captured_text() {
        let (mut app, commands, captures, _cursor_captures) = fixture_app_with_tmux(
            WorkspaceStatus::Active,
            vec![Ok(String::new()), Ok("copy me".to_string())],
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );

        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('c'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );
        ftui::Model::update(
            &mut app,
            Msg::Key(
                KeyEvent::new(KeyCode::Char('v'))
                    .with_modifiers(Modifiers::ALT)
                    .with_kind(KeyEventKind::Press),
            ),
        );

        assert!(captures.borrow().is_empty());
        assert_eq!(
            commands.borrow().last(),
            Some(&vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "copy me".to_string(),
            ])
        );
    }

    #[test]
    fn shell_contains_list_preview_and_status_placeholders() {
        let app = fixture_app();
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("Workspaces"));
        assert!(content.contains("Preview Pane"));
        assert!(content.contains("Status:"));
        assert!(content.contains("feature-a | feature-a | /repos/grove-feature-a"));
        assert!(content.contains("Workspace: grove"));
    }

    #[test]
    fn shell_renders_discovery_error_state() {
        let app = GroveApp::from_bootstrap(BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Error("fatal: not a git repository".to_string()),
            orphaned_sessions: Vec::new(),
        });
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("discovery failed"));
        assert!(content.contains("discovery error"));
    }

    #[test]
    fn preview_mode_keys_scroll_and_jump_to_bottom() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.mode, crate::state::UiMode::Preview);

        let was_auto_scroll = app.preview.auto_scroll;
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );
        assert!(was_auto_scroll);
        assert!(!app.preview.auto_scroll);
        assert!(app.preview.offset > 0);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('G')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.preview.offset, 0);
        assert!(app.preview.auto_scroll);
    }
}
