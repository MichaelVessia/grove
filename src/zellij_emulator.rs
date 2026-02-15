use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;

use frankenterm_core::{Cell, Color, SgrAttrs, SgrFlags, TerminalEngine};

const DEFAULT_COLS: u16 = 120;
const DEFAULT_ROWS: u16 = 40;
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
        let (cols, rows) = pane_size.unwrap_or((DEFAULT_COLS, DEFAULT_ROWS));
        let terminal = self
            .sessions
            .entry(session.to_string())
            .or_insert_with(|| SessionTerminal::new(cols, rows));
        terminal.ensure_size(cols, rows);

        let source = match fs::read(log_path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
            Err(error) => {
                return Err(io::Error::other(format!(
                    "zellij capture log read failed for '{session}': {error}"
                )));
            }
        };

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
    if let Some(relative_index) = find_subslice(&chunk[start..], script_done_marker) {
        end = start.saturating_add(relative_index);
    } else if chunk[start..].starts_with(b"Script done on ") {
        end = start;
    }

    if start >= end {
        return Vec::new();
    }

    chunk[start..end]
        .iter()
        .copied()
        .filter(|byte| *byte != 0)
        .collect()
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
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{ZellijPreviewEmulator, sanitize_log_chunk};

    fn unique_log_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos();
        std::env::temp_dir().join(format!(
            "grove-zellij-emulator-{label}-{}-{timestamp}.log",
            std::process::id()
        ))
    }

    #[test]
    fn sanitize_log_chunk_strips_script_wrappers_and_nul() {
        let sanitized = sanitize_log_chunk(
            b"Script started on now\n\0\x1b[31mred\x1b[0m\nScript done on now\n",
            true,
        );
        assert_eq!(sanitized, b"\x1b[31mred\x1b[0m");
    }

    #[test]
    fn emulator_capture_is_incremental_and_resets_after_truncate() {
        let path = unique_log_path("incremental");
        fs::write(
            &path,
            b"Script started on now\n\x1b[32mhello\x1b[0m\nScript done on now\n",
        )
        .expect("log should write");

        let mut emulator = ZellijPreviewEmulator::default();
        let first = emulator
            .capture_from_log("grove-ws-test", &path, Some((80, 24)), 200)
            .expect("capture should succeed");
        assert!(first.contains("hello"));
        assert!(first.contains("\u{1b}[0;32m"));

        fs::write(
            &path,
            b"Script started on now\n\x1b[31mreset\x1b[0m\nScript done on now\n",
        )
        .expect("truncated log should write");
        let second = emulator
            .capture_from_log("grove-ws-test", &path, Some((80, 24)), 200)
            .expect("capture should succeed after truncate");
        assert!(second.contains("reset"));
        assert!(!second.contains("hello"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn emulator_capture_reads_only_appended_bytes() {
        let path = unique_log_path("append");
        fs::write(&path, b"Script started on now\nline one\n").expect("initial log should write");

        let mut emulator = ZellijPreviewEmulator::default();
        let first = emulator
            .capture_from_log("grove-ws-append", &path, Some((80, 24)), 200)
            .expect("capture should succeed");
        assert!(first.contains("line one"));

        fs::write(&path, b"Script started on now\nline one\nline two\n")
            .expect("appended log should write");
        let second = emulator
            .capture_from_log("grove-ws-append", &path, Some((80, 24)), 200)
            .expect("capture should succeed");
        assert!(second.contains("line one"));
        assert!(second.contains("line two"));

        let _ = fs::remove_file(path);
    }
}
