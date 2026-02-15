use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;

use crate::application::agent_runtime::{ZELLIJ_CAPTURE_COLS, ZELLIJ_CAPTURE_ROWS};
use frankenterm_core::{Cell, Color, SgrAttrs, SgrFlags, TerminalEngine};

const RESET_SGR: &str = "\u{1b}[0m";

#[derive(Debug, Default)]
pub(crate) struct ZellijPreviewEmulator {
    sessions: HashMap<String, SessionTerminal>,
}

impl ZellijPreviewEmulator {
    pub(crate) fn capture_from_log(
        &mut self,
        session: &str,
        log_path: &Path,
        pane_size: Option<(u16, u16)>,
        scrollback_lines: usize,
    ) -> io::Result<String> {
        let source = match fs::read(log_path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
            Err(error) => {
                return Err(io::Error::other(format!(
                    "zellij capture log read failed for '{session}': {error}"
                )));
            }
        };

        let log_size = parse_script_header_size(&source);
        let (cols, rows) = pane_size
            .or(log_size)
            .unwrap_or((ZELLIJ_CAPTURE_COLS, ZELLIJ_CAPTURE_ROWS));
        let terminal = self
            .sessions
            .entry(session.to_string())
            .or_insert_with(|| SessionTerminal::new(cols, rows));
        terminal.ensure_size(cols, rows);
        terminal.ingest(&source);
        Ok(terminal.render(scrollback_lines))
    }
}

#[derive(Debug)]
struct SessionTerminal {
    engine: TerminalEngine,
    consumed_bytes: usize,
    consumed_prefix_hash: u64,
    cols: u16,
    rows: u16,
}

impl SessionTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            engine: TerminalEngine::new(cols, rows),
            consumed_bytes: 0,
            consumed_prefix_hash: 0,
            cols,
            rows,
        }
    }

    fn ensure_size(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.engine.resize(cols, rows);
    }

    fn reset(&mut self) {
        self.engine = TerminalEngine::new(self.cols.max(1), self.rows.max(1));
        self.consumed_bytes = 0;
        self.consumed_prefix_hash = 0;
    }

    fn ingest(&mut self, source: &[u8]) {
        if source.len() < self.consumed_bytes {
            self.reset();
        }
        if self.consumed_bytes > 0 {
            let existing_prefix = &source[..self.consumed_bytes];
            let prefix_hash = hash_bytes(existing_prefix);
            if prefix_hash != self.consumed_prefix_hash {
                self.reset();
            }
        }
        if source.len() == self.consumed_bytes {
            return;
        }

        let chunk = &source[self.consumed_bytes..];
        let sanitized = sanitize_log_chunk(chunk, self.consumed_bytes == 0);
        if !sanitized.is_empty() {
            self.engine.feed_bytes(&sanitized);
        }
        self.consumed_bytes = source.len();
        self.consumed_prefix_hash = hash_bytes(source);
    }

    fn render(&self, scrollback_lines: usize) -> String {
        if scrollback_lines == 0 {
            return String::new();
        }

        let mut lines: Vec<String> = Vec::new();
        for line in self.engine.scrollback().iter() {
            lines.push(render_cells(&line.cells));
        }
        for row in 0..self.engine.rows() {
            if let Some(cells) = self.engine.grid().row_cells(row) {
                lines.push(render_cells(cells));
            }
        }

        while lines.last().is_some_and(|line| line.is_empty()) {
            let _ = lines.pop();
        }

        if lines.len() > scrollback_lines {
            let start = lines.len().saturating_sub(scrollback_lines);
            lines = lines.split_off(start);
        }

        lines.join("\n")
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn parse_script_header_size(source: &[u8]) -> Option<(u16, u16)> {
    let newline_index = source.iter().position(|byte| *byte == b'\n')?;
    let header = std::str::from_utf8(&source[..newline_index]).ok()?;
    if !header.starts_with("Script started on ") {
        return None;
    }
    let cols = parse_header_dimension(header, "COLUMNS")?;
    let rows = parse_header_dimension(header, "LINES")?;
    Some((cols.max(1), rows.max(1)))
}

fn parse_header_dimension(header: &str, key: &str) -> Option<u16> {
    let needle = format!("{key}=\"");
    let start = header.find(&needle)?.saturating_add(needle.len());
    let rest = &header[start..];
    let end = rest.find('"')?;
    let raw = &rest[..end];
    let parsed = raw.parse::<i32>().ok()?;
    if parsed <= 0 {
        return None;
    }
    u16::try_from(parsed).ok()
}

fn sanitize_log_chunk(chunk: &[u8], is_first_chunk: bool) -> Vec<u8> {
    let mut start = 0usize;
    if is_first_chunk && chunk.starts_with(b"Script started on ") {
        let Some(index) = chunk.iter().position(|byte| *byte == b'\n') else {
            return Vec::new();
        };
        start = index.saturating_add(1);
    }

    let mut end = chunk.len();
    let script_done_marker = b"\nScript done on ";
    let mut exit_code: Option<String> = None;
    if let Some(relative_index) = find_subslice(&chunk[start..], script_done_marker) {
        let done_line_start = start.saturating_add(relative_index).saturating_add(1);
        exit_code = parse_script_done_exit_code(&chunk[done_line_start..]);
        end = start.saturating_add(relative_index);
    } else if chunk[start..].starts_with(b"Script done on ") {
        exit_code = parse_script_done_exit_code(&chunk[start..]);
        end = start;
    }

    if start >= end {
        if let Some(code) = exit_code {
            return format!("exited with code {code}\n").into_bytes();
        }
        return Vec::new();
    }

    let mut sanitized: Vec<u8> = chunk[start..end]
        .iter()
        .copied()
        .filter(|byte| *byte != 0)
        .collect();
    if let Some(code) = exit_code {
        if sanitized.last().is_some_and(|byte| *byte != b'\n') {
            sanitized.push(b'\n');
        }
        sanitized.extend_from_slice(format!("exited with code {code}\n").as_bytes());
    }
    sanitized
}

fn parse_script_done_exit_code(source: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(source).ok()?;
    let line = text.lines().next()?;
    let key = "COMMAND_EXIT_CODE=\"";
    let start = line.find(key)?.saturating_add(key.len());
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn render_cells(cells: &[Cell]) -> String {
    let Some(last_column) = last_non_blank_column(cells) else {
        return String::new();
    };

    let mut line = String::new();
    let mut previous_attrs = SgrAttrs::default();

    for column in 0..=last_column {
        let Some(cell) = cells.get(column) else {
            break;
        };
        if cell.is_wide_continuation() {
            continue;
        }

        if cell.attrs != previous_attrs {
            line.push_str(&sgr_sequence(cell.attrs));
            previous_attrs = cell.attrs;
        }

        line.push(cell.content());
        for mark in cell.combining_marks() {
            line.push(*mark);
        }
    }

    if previous_attrs != SgrAttrs::default() {
        line.push_str(RESET_SGR);
    }

    line
}

fn last_non_blank_column(cells: &[Cell]) -> Option<usize> {
    for (index, cell) in cells.iter().enumerate().rev() {
        if cell.is_wide_continuation() {
            continue;
        }
        if cell.content() != ' ' || !cell.combining_marks().is_empty() {
            return Some(index);
        }
    }
    None
}

fn sgr_sequence(attrs: SgrAttrs) -> String {
    if attrs == SgrAttrs::default() {
        return RESET_SGR.to_string();
    }

    let mut params: Vec<String> = vec!["0".to_string()];
    push_sgr_flags(&mut params, attrs.flags);
    push_sgr_color(&mut params, true, attrs.fg);
    push_sgr_color(&mut params, false, attrs.bg);

    format!("\u{1b}[{}m", params.join(";"))
}

fn push_sgr_flags(params: &mut Vec<String>, flags: SgrFlags) {
    if flags.contains(SgrFlags::BOLD) {
        params.push("1".to_string());
    }
    if flags.contains(SgrFlags::DIM) {
        params.push("2".to_string());
    }
    if flags.contains(SgrFlags::ITALIC) {
        params.push("3".to_string());
    }
    if flags.contains(SgrFlags::UNDERLINE) {
        params.push("4".to_string());
    }
    if flags.contains(SgrFlags::BLINK) {
        params.push("5".to_string());
    }
    if flags.contains(SgrFlags::INVERSE) {
        params.push("7".to_string());
    }
    if flags.contains(SgrFlags::HIDDEN) {
        params.push("8".to_string());
    }
    if flags.contains(SgrFlags::STRIKETHROUGH) {
        params.push("9".to_string());
    }
    if flags.contains(SgrFlags::DOUBLE_UNDERLINE) {
        params.push("21".to_string());
    }
    if flags.contains(SgrFlags::OVERLINE) {
        params.push("53".to_string());
    }
}

fn push_sgr_color(params: &mut Vec<String>, foreground: bool, color: Color) {
    match color {
        Color::Default => {}
        Color::Named(index) => {
            let code = if foreground {
                named_fg_code(index)
            } else {
                named_bg_code(index)
            };
            params.push(code.to_string());
        }
        Color::Indexed(index) => {
            if foreground {
                params.push("38".to_string());
            } else {
                params.push("48".to_string());
            }
            params.push("5".to_string());
            params.push(index.to_string());
        }
        Color::Rgb(red, green, blue) => {
            if foreground {
                params.push("38".to_string());
            } else {
                params.push("48".to_string());
            }
            params.push("2".to_string());
            params.push(red.to_string());
            params.push(green.to_string());
            params.push(blue.to_string());
        }
    }
}

fn named_fg_code(index: u8) -> u16 {
    if index < 8 {
        return 30 + u16::from(index);
    }
    if index < 16 {
        return 90 + u16::from(index - 8);
    }
    39
}

fn named_bg_code(index: u8) -> u16 {
    if index < 8 {
        return 40 + u16::from(index);
    }
    if index < 16 {
        return 100 + u16::from(index - 8);
    }
    49
}

#[cfg(test)]
mod tests;
