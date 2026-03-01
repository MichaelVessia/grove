use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::domain::WorkspaceStatus;

pub(super) fn session_file_skip_permissions_mode(path: &Path, max_lines: usize) -> Option<bool> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok).take(max_lines) {
        if let Some(skip_permissions) = text_skip_permissions_mode(&line) {
            return Some(skip_permissions);
        }
    }

    None
}

pub(super) fn text_skip_permissions_mode(value: &str) -> Option<bool> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("approval policy is currently never")
        || lower.contains("<approval_policy>never</approval_policy>")
        || lower.contains("\"approval_policy\":\"never\"")
        || lower.contains("\"approval_policy\": \"never\"")
        || lower.contains("\"permissionmode\":\"bypasspermissions\"")
        || lower.contains("\"permissionmode\": \"bypasspermissions\"")
    {
        return Some(true);
    }
    if lower.contains("approval policy is currently on-request")
        || lower.contains("<approval_policy>on-request</approval_policy>")
        || lower.contains("\"approval_policy\":\"on-request\"")
        || lower.contains("\"approval_policy\": \"on-request\"")
        || lower.contains("\"permissionmode\":\"default\"")
        || lower.contains("\"permissionmode\": \"default\"")
    {
        return Some(false);
    }

    None
}

pub(super) fn absolute_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    let current = std::env::current_dir().ok()?;
    Some(current.join(path))
}

pub(super) fn is_file_recently_modified(path: &Path, threshold: Duration) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified_at| modified_at.elapsed().ok())
        .is_some_and(|age| age < threshold)
}

pub(super) fn any_file_recently_modified(dir: &Path, suffix: &str, threshold: Duration) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    entries.flatten().any(|entry| {
        entry.file_type().is_ok_and(|ft| ft.is_file())
            && entry.file_name().to_string_lossy().ends_with(suffix)
            && is_file_recently_modified(&entry.path(), threshold)
    })
}

pub(super) fn find_recent_jsonl_files(
    dir: &Path,
    exclude_prefix: Option<&str>,
) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).ok()?;
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.ends_with(".jsonl") {
            continue;
        }
        if exclude_prefix.is_some_and(|prefix| file_name.starts_with(prefix)) {
            continue;
        }
        let modified = match entry.metadata().and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified,
            Err(_) => continue,
        };
        files.push((entry.path(), modified));
    }

    files.sort_by(|left, right| right.1.cmp(&left.1));
    Some(files.into_iter().map(|(path, _)| path).collect())
}

pub(super) fn read_tail_lines(path: &Path, max_bytes: usize) -> Option<Vec<String>> {
    let mut file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    if size == 0 {
        return Some(Vec::new());
    }

    let max_bytes_u64 = u64::try_from(max_bytes).ok()?;
    let start = size.saturating_sub(max_bytes_u64);
    file.seek(SeekFrom::Start(start)).ok()?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;

    let mut lines: Vec<String> = String::from_utf8_lossy(&bytes)
        .lines()
        .map(|line| line.to_string())
        .collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    Some(lines)
}

pub(super) fn get_last_message_status_jsonl(
    path: &Path,
    type_field: &str,
    user_value: &str,
    assistant_value: &str,
    tail_bytes: usize,
) -> Option<WorkspaceStatus> {
    let lines = read_tail_lines(path, tail_bytes)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(message_type) = value.get(type_field).and_then(|value| value.as_str()) else {
            continue;
        };
        if message_type == user_value {
            return Some(WorkspaceStatus::Active);
        }
        if message_type == assistant_value {
            return Some(WorkspaceStatus::Waiting);
        }
    }
    None
}

pub(super) fn marker_for_session_line(path: &Path, line: &str) -> Option<String> {
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()?;
    let modified_ms = modified.duration_since(UNIX_EPOCH).ok()?.as_millis();
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    let line_hash = hasher.finish();
    Some(format!("{}:{modified_ms}:{line_hash}", path.display()))
}

pub(super) fn get_last_message_marker_jsonl(
    path: &Path,
    type_field: &str,
    user_value: &str,
    assistant_value: &str,
    tail_bytes: usize,
) -> Option<(bool, String)> {
    let lines = read_tail_lines(path, tail_bytes)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(message_type) = value.get(type_field).and_then(|value| value.as_str()) else {
            continue;
        };
        if message_type == user_value {
            let marker = marker_for_session_line(path, trimmed)?;
            return Some((false, marker));
        }
        if message_type == assistant_value {
            let marker = marker_for_session_line(path, trimmed)?;
            return Some((true, marker));
        }
    }
    None
}

pub(super) fn cwd_matches(cwd: &Path, workspace_path: &Path) -> bool {
    let cwd = match absolute_path(cwd) {
        Some(path) => path,
        None => return false,
    };
    cwd == workspace_path || cwd.starts_with(workspace_path)
}

pub(super) fn prune_by_oldest<K, V, T, C, O>(
    cache: &mut HashMap<K, V>,
    max_entries: usize,
    ttl: Option<Duration>,
    checked_at: C,
    oldest_key: O,
) where
    K: Clone + Eq + Hash,
    T: Ord,
    C: Fn(&V) -> Instant,
    O: Fn(&V) -> T,
{
    if let Some(ttl) = ttl {
        let now = Instant::now();
        cache.retain(|_, entry| now.saturating_duration_since(checked_at(entry)) < ttl);
    }

    let overflow = cache.len().saturating_sub(max_entries);
    if overflow == 0 {
        return;
    }

    let mut entries: Vec<(T, K)> = cache
        .iter()
        .map(|(key, value)| (oldest_key(value), key.clone()))
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (_, key) in entries.into_iter().take(overflow) {
        cache.remove(&key);
    }
}
