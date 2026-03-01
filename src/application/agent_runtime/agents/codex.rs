use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use crate::domain::WorkspaceStatus;

use super::shared;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionLookupKey {
    sessions_dir: PathBuf,
    workspace_path: PathBuf,
}

#[derive(Debug, Clone)]
struct SessionLookupCacheEntry {
    checked_at: Instant,
    session_file: PathBuf,
}

#[derive(Debug, Clone)]
struct MessageStatusCacheEntry {
    modified_at: SystemTime,
    status: Option<WorkspaceStatus>,
}

fn session_lookup_cache() -> &'static Mutex<HashMap<SessionLookupKey, SessionLookupCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<SessionLookupKey, SessionLookupCacheEntry>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn message_status_cache() -> &'static Mutex<HashMap<PathBuf, MessageStatusCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, MessageStatusCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn extract_resume_command(output: &str) -> Option<String> {
    let mut found = None;
    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }

        for index in 0..tokens.len().saturating_sub(2) {
            if tokens[index] != "codex" {
                continue;
            }

            let mode = tokens[index + 1];
            if mode != "resume" && mode != "--resume" {
                continue;
            }

            let Some(session_id) = super::normalize_codex_resume_session_id(tokens[index + 2])
            else {
                continue;
            };
            found = Some(format!("codex resume {session_id}"));
        }
    }

    found
}

pub(super) fn infer_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    let sessions_dir = home_dir.join(".codex").join("sessions");
    let session_file = find_session_for_path_cached(&sessions_dir, workspace_path)?;
    session_skip_permissions_mode(&session_file)
}

pub(super) fn session_skip_permissions_mode(path: &Path) -> Option<bool> {
    shared::session_file_skip_permissions_mode(path, 24)
}

pub(super) fn detect_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    let sessions_dir = home_dir.join(".codex").join("sessions");
    let session_file = find_session_for_path_cached(&sessions_dir, workspace_path)?;

    if shared::is_file_recently_modified(&session_file, activity_threshold) {
        return Some(WorkspaceStatus::Active);
    }

    get_last_message_status_cached(&session_file)
}

pub(super) fn latest_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    let sessions_dir = home_dir.join(".codex").join("sessions");
    let session_file = find_session_for_path_cached(&sessions_dir, workspace_path)?;
    let (is_assistant, marker) = get_last_message_marker(&session_file)?;
    is_assistant.then_some(marker)
}

fn find_session_for_path_cached(sessions_dir: &Path, workspace_path: &Path) -> Option<PathBuf> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let key = SessionLookupKey {
        sessions_dir: sessions_dir.to_path_buf(),
        workspace_path: workspace_path.clone(),
    };
    let now = Instant::now();

    if let Ok(cache) = session_lookup_cache().lock()
        && let Some(entry) = cache.get(&key)
        && now.saturating_duration_since(entry.checked_at)
            < super::super::CODEX_SESSION_LOOKUP_REFRESH_INTERVAL
        && entry.session_file.exists()
    {
        return Some(entry.session_file.clone());
    }

    let session_file = find_session_for_path(sessions_dir, &workspace_path);
    if let Ok(mut cache) = session_lookup_cache().lock() {
        if let Some(session_file) = session_file.as_ref() {
            cache.insert(
                key,
                SessionLookupCacheEntry {
                    checked_at: now,
                    session_file: session_file.clone(),
                },
            );
            shared::prune_by_oldest(
                &mut cache,
                super::super::SESSION_LOOKUP_CACHE_MAX_ENTRIES,
                Some(super::super::SESSION_LOOKUP_EVICTION_TTL),
                |entry| entry.checked_at,
                |entry| entry.checked_at,
            );
        } else {
            cache.remove(&key);
        }
    }

    session_file
}

fn get_last_message_status_cached(path: &Path) -> Option<WorkspaceStatus> {
    let modified_at = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()?;
    if let Ok(cache) = message_status_cache().lock()
        && let Some(entry) = cache.get(path)
        && entry.modified_at == modified_at
    {
        return entry.status;
    }

    let status = get_last_message_status(path);
    if let Ok(mut cache) = message_status_cache().lock() {
        let inserted_at = Instant::now();
        cache.insert(
            path.to_path_buf(),
            MessageStatusCacheEntry {
                modified_at,
                status,
            },
        );
        shared::prune_by_oldest(
            &mut cache,
            super::super::MESSAGE_STATUS_CACHE_MAX_ENTRIES,
            None,
            |_| inserted_at,
            |entry| entry.modified_at,
        );
    }

    status
}

#[cfg(test)]
fn reset_caches_for_test() {
    if let Ok(mut cache) = session_lookup_cache().lock() {
        cache.clear();
    }
    if let Ok(mut cache) = message_status_cache().lock() {
        cache.clear();
    }
}

fn find_session_for_path(sessions_dir: &Path, workspace_path: &Path) -> Option<PathBuf> {
    let mut pending = vec![sessions_dir.to_path_buf()];
    let mut best_path: Option<PathBuf> = None;
    let mut best_time: Option<SystemTime> = None;

    while let Some(dir) = pending.pop() {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file()
                || path
                    .extension()
                    .is_none_or(|extension| extension != "jsonl")
            {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let Some(cwd) = get_session_cwd(&path) else {
                continue;
            };
            if !shared::cwd_matches(&cwd, workspace_path) {
                continue;
            }
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            let replace = match best_time {
                Some(current_best) => modified > current_best,
                None => true,
            };
            if replace {
                best_time = Some(modified);
                best_path = Some(path);
            }
        }
    }

    best_path
}

fn get_session_cwd(path: &Path) -> Option<PathBuf> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("session_meta") {
            continue;
        }
        let cwd = value
            .get("payload")
            .and_then(|payload| payload.get("cwd"))
            .and_then(|cwd| cwd.as_str())?;
        return Some(PathBuf::from(cwd));
    }
    None
}

fn get_last_message_status(path: &Path) -> Option<WorkspaceStatus> {
    let lines = shared::read_tail_lines(path, super::super::SESSION_STATUS_TAIL_BYTES)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("response_item") {
            continue;
        }
        if value
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(|value| value.as_str())
            != Some("message")
        {
            continue;
        }
        match value
            .get("payload")
            .and_then(|payload| payload.get("role"))
            .and_then(|value| value.as_str())
        {
            Some("assistant") => return Some(WorkspaceStatus::Waiting),
            Some("user") => return Some(WorkspaceStatus::Active),
            _ => continue,
        }
    }
    None
}

fn get_last_message_marker(path: &Path) -> Option<(bool, String)> {
    let lines = shared::read_tail_lines(path, super::super::SESSION_STATUS_TAIL_BYTES)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("response_item") {
            continue;
        }
        if value
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(|value| value.as_str())
            != Some("message")
        {
            continue;
        }
        match value
            .get("payload")
            .and_then(|payload| payload.get("role"))
            .and_then(|value| value.as_str())
        {
            Some("assistant") => {
                let marker = shared::marker_for_session_line(path, trimmed)?;
                return Some((true, marker));
            }
            Some("user") => {
                let marker = shared::marker_for_session_line(path, trimmed)?;
                return Some((false, marker));
            }
            _ => continue,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", process::id()));
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }

    fn write_codex_session_file(path: &Path, cwd: &Path) {
        let line = format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\"}}}}\n",
            cwd.display()
        );
        fs::write(path, line).expect("session file should be written");
    }

    fn write_waiting_message(path: &Path) {
        fs::write(
            path,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\"}}\n",
        )
        .expect("message file should be written");
    }

    #[test]
    fn session_lookup_cache_prunes_oldest_entries_when_over_limit() {
        reset_caches_for_test();
        let sessions_dir = PathBuf::from("/tmp/codex-cache-size");
        let now = Instant::now();
        let key_oldest = SessionLookupKey {
            sessions_dir: sessions_dir.clone(),
            workspace_path: PathBuf::from("/tmp/ws-oldest"),
        };
        let key_old = SessionLookupKey {
            sessions_dir: sessions_dir.clone(),
            workspace_path: PathBuf::from("/tmp/ws-old"),
        };
        let key_new = SessionLookupKey {
            sessions_dir,
            workspace_path: PathBuf::from("/tmp/ws-new"),
        };

        let mut cache = session_lookup_cache()
            .lock()
            .expect("session cache lock should succeed");
        cache.insert(
            key_oldest.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(3),
                session_file: PathBuf::from("/tmp/a.jsonl"),
            },
        );
        cache.insert(
            key_old.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(2),
                session_file: PathBuf::from("/tmp/b.jsonl"),
            },
        );
        cache.insert(
            key_new.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(1),
                session_file: PathBuf::from("/tmp/c.jsonl"),
            },
        );

        shared::prune_by_oldest(
            &mut cache,
            2,
            Some(Duration::from_secs(60)),
            |entry| entry.checked_at,
            |entry| entry.checked_at,
        );

        assert_eq!(cache.len(), 2);
        assert!(!cache.contains_key(&key_oldest));
        assert!(cache.contains_key(&key_old));
        assert!(cache.contains_key(&key_new));
    }

    #[test]
    fn session_lookup_cache_ttl_evicts_stale_entries_and_recomputes() {
        reset_caches_for_test();
        let root = unique_test_dir("codex-session-ttl");
        let sessions_dir = root.join("sessions");
        let workspace_path = root.join("workspace");
        let stale_workspace_path = root.join("stale-workspace");
        fs::create_dir_all(&sessions_dir).expect("sessions directory should be created");
        fs::create_dir_all(&workspace_path).expect("workspace directory should be created");
        fs::create_dir_all(&stale_workspace_path).expect("stale workspace should be created");
        let session_file = sessions_dir.join("session.jsonl");
        write_codex_session_file(&session_file, &workspace_path);

        let stale_key = SessionLookupKey {
            sessions_dir: sessions_dir.clone(),
            workspace_path: stale_workspace_path,
        };
        session_lookup_cache()
            .lock()
            .expect("session cache lock should succeed")
            .insert(
                stale_key.clone(),
                SessionLookupCacheEntry {
                    checked_at: Instant::now()
                        - super::super::super::SESSION_LOOKUP_EVICTION_TTL
                        - Duration::from_secs(1),
                    session_file: root.join("stale.jsonl"),
                },
            );

        let found = find_session_for_path_cached(&sessions_dir, &workspace_path);

        assert_eq!(found, Some(session_file));
        let cache = session_lookup_cache()
            .lock()
            .expect("session cache lock should succeed");
        assert!(!cache.contains_key(&stale_key));
    }

    #[test]
    fn message_status_cache_keeps_old_entries_when_within_size_cap() {
        reset_caches_for_test();
        let root = unique_test_dir("codex-message-cap");
        let session_a = root.join("session-a.jsonl");
        let session_b = root.join("session-b.jsonl");
        write_waiting_message(&session_a);
        write_waiting_message(&session_b);

        assert_eq!(
            get_last_message_status_cached(&session_a),
            Some(WorkspaceStatus::Waiting)
        );
        {
            let mut cache = message_status_cache()
                .lock()
                .expect("message status cache lock should succeed");
            let entry = cache
                .get_mut(&session_a)
                .expect("session-a should be cached");
            entry.modified_at = UNIX_EPOCH;
        }

        assert_eq!(
            get_last_message_status_cached(&session_b),
            Some(WorkspaceStatus::Waiting)
        );
        let cache = message_status_cache()
            .lock()
            .expect("message status cache lock should succeed");
        assert!(cache.contains_key(&session_a));
        assert!(cache.contains_key(&session_b));
    }
}
