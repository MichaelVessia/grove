use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use serde::Deserialize;

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

#[derive(Debug, Clone)]
struct SessionCwdCacheEntry {
    cached_at: Instant,
    modified_at: SystemTime,
    cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SessionMetaLine<'a> {
    #[serde(borrow, rename = "type")]
    line_type: &'a str,
    #[serde(borrow)]
    payload: Option<SessionMetaPayload<'a>>,
}

#[derive(Debug, Deserialize)]
struct SessionMetaPayload<'a> {
    #[serde(borrow)]
    cwd: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct ResponseItemLine<'a> {
    #[serde(borrow, rename = "type")]
    line_type: &'a str,
    #[serde(borrow)]
    payload: Option<ResponsePayload<'a>>,
}

#[derive(Debug, Deserialize)]
struct ResponsePayload<'a> {
    #[serde(borrow, rename = "type")]
    payload_type: Option<&'a str>,
    #[serde(borrow)]
    role: Option<&'a str>,
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

fn session_cwd_cache() -> &'static Mutex<HashMap<PathBuf, SessionCwdCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, SessionCwdCacheEntry>>> = OnceLock::new();
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
    if let Ok(mut cache) = session_cwd_cache().lock() {
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
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(_) => continue,
            };

            let Some(cwd) = get_session_cwd_cached(&path, modified) else {
                continue;
            };
            if !shared::cwd_matches(&cwd, workspace_path) {
                continue;
            }
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

fn get_session_cwd_cached(path: &Path, modified_at: SystemTime) -> Option<PathBuf> {
    if let Ok(cache) = session_cwd_cache().lock()
        && let Some(entry) = cache.get(path)
        && entry.modified_at == modified_at
    {
        return entry.cwd.clone();
    }

    let cwd = get_session_cwd(path);
    if let Ok(mut cache) = session_cwd_cache().lock() {
        let cached_at = Instant::now();
        cache.insert(
            path.to_path_buf(),
            SessionCwdCacheEntry {
                cached_at,
                modified_at,
                cwd: cwd.clone(),
            },
        );
        shared::prune_by_oldest(
            &mut cache,
            super::super::SESSION_LOOKUP_CACHE_MAX_ENTRIES,
            Some(super::super::SESSION_LOOKUP_EVICTION_TTL),
            |entry| entry.cached_at,
            |entry| entry.cached_at,
        );
    }

    cwd
}

fn get_session_cwd(path: &Path) -> Option<PathBuf> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let Some(cwd) = parse_session_meta_cwd(&line) else {
            continue;
        };
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
        match parse_response_message_role(trimmed) {
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
        match parse_response_message_role(trimmed) {
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

fn parse_session_meta_cwd(line: &str) -> Option<&str> {
    let parsed = serde_json::from_str::<SessionMetaLine<'_>>(line).ok()?;
    if parsed.line_type != "session_meta" {
        return None;
    }
    parsed.payload.and_then(|payload| payload.cwd)
}

fn parse_response_message_role(line: &str) -> Option<&str> {
    let parsed = serde_json::from_str::<ResponseItemLine<'_>>(line).ok()?;
    if parsed.line_type != "response_item" {
        return None;
    }
    let payload = parsed.payload?;
    if payload.payload_type != Some("message") {
        return None;
    }
    payload.role
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant, UNIX_EPOCH};

    use super::*;

    use crate::test_support::unique_test_dir;

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

    #[test]
    fn session_cwd_cache_reloads_when_file_modification_changes() {
        reset_caches_for_test();
        let root = unique_test_dir("codex-cwd-cache");
        let workspace_a = root.join("workspace-a");
        let workspace_b = root.join("workspace-b");
        fs::create_dir_all(&workspace_a).expect("workspace-a should exist");
        fs::create_dir_all(&workspace_b).expect("workspace-b should exist");
        let session_file = root.join("session.jsonl");
        write_codex_session_file(&session_file, &workspace_a);

        let modified_a = fs::metadata(&session_file)
            .and_then(|metadata| metadata.modified())
            .expect("modified time should be available");
        let cached_a = get_session_cwd_cached(&session_file, modified_a)
            .expect("cwd should be cached for first version");
        assert_eq!(cached_a, workspace_a);

        std::thread::sleep(Duration::from_millis(10));
        write_codex_session_file(&session_file, &workspace_b);
        let modified_b = fs::metadata(&session_file)
            .and_then(|metadata| metadata.modified())
            .expect("modified time should be available");
        let cached_b = get_session_cwd_cached(&session_file, modified_b)
            .expect("cwd should refresh after modification");
        assert_eq!(cached_b, workspace_b);
    }
}
