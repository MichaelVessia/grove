use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;

use crate::domain::WorkspaceStatus;

use super::shared;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionLookupKey {
    database_path: PathBuf,
    workspace_path: PathBuf,
}

#[derive(Debug, Clone)]
struct SessionLookupCacheEntry {
    checked_at: Instant,
    session: SessionMetadata,
}

#[derive(Debug, Clone)]
struct SessionMetadata {
    session_id: String,
    time_updated_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatabaseFileState {
    modified_at: Option<SystemTime>,
    length_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatabaseState {
    database: DatabaseFileState,
    wal: DatabaseFileState,
}

#[derive(Debug, Clone)]
struct SessionDirectoryEntry {
    session_id: String,
    directory: PathBuf,
    time_updated_ms: i64,
}

#[derive(Debug, Clone)]
struct SessionDirectoryCacheEntry {
    cached_at: Instant,
    state: DatabaseState,
    sessions: Vec<SessionDirectoryEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageRole {
    Assistant,
    User,
    Other,
}

#[derive(Debug, Deserialize)]
struct MessageData<'a> {
    #[serde(borrow)]
    role: Option<&'a str>,
}

fn session_lookup_cache() -> &'static Mutex<HashMap<SessionLookupKey, SessionLookupCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<SessionLookupKey, SessionLookupCacheEntry>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn session_directory_cache() -> &'static Mutex<HashMap<PathBuf, SessionDirectoryCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, SessionDirectoryCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn extract_resume_command(output: &str) -> Option<String> {
    let mut found = None;
    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }

        for index in 0..tokens.len().saturating_sub(1) {
            if tokens[index] != "opencode" {
                continue;
            }

            let mode = tokens[index + 1];
            if mode == "-c" || mode == "--continue" {
                found = Some("opencode --continue".to_string());
                continue;
            }
            if mode != "-s" && mode != "--session" {
                continue;
            }
            if index + 2 >= tokens.len() {
                continue;
            }

            let Some(session_id) = super::normalize_resume_session_id(tokens[index + 2]) else {
                continue;
            };
            found = Some(format!("opencode -s {session_id}"));
        }
    }

    found
}

pub(super) fn infer_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    let database_path = database_path_in_home(home_dir);
    let session = find_session_for_path_cached(&database_path, workspace_path)?;
    session_skip_permissions_mode(&database_path, &session.session_id)
}

pub(super) fn infer_resume_command_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    let database_path = database_path_in_home(home_dir);
    let session = find_session_for_path_cached(&database_path, workspace_path)?;
    Some(format!("opencode -s {}", session.session_id))
}

pub(super) fn detect_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    let database_path = database_path_in_home(home_dir);
    let session = find_session_for_path_cached(&database_path, workspace_path)?;

    if is_timestamp_recently_updated_ms(session.time_updated_ms, activity_threshold) {
        return Some(WorkspaceStatus::Active);
    }

    let (_, role, _) = get_last_message_entry(&database_path, &session.session_id)?;
    match role {
        MessageRole::Assistant => Some(WorkspaceStatus::Waiting),
        MessageRole::User => Some(WorkspaceStatus::Active),
        MessageRole::Other => None,
    }
}

pub(super) fn latest_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    let database_path = database_path_in_home(home_dir);
    let session = find_session_for_path_cached(&database_path, workspace_path)?;
    let (message_id, role, message_updated_ms) =
        get_last_message_entry(&database_path, &session.session_id)?;
    if role != MessageRole::Assistant {
        return None;
    }

    Some(format!(
        "{}:{}:{message_id}:{message_updated_ms}",
        database_path.display(),
        session.session_id
    ))
}

fn session_skip_permissions_mode(database_path: &Path, session_id: &str) -> Option<bool> {
    let connection = open_database(database_path)?;
    for ordering in ["DESC", "ASC"] {
        let query = format!(
            "SELECT data FROM message WHERE session_id = ? ORDER BY time_created {ordering} LIMIT 32"
        );
        let mut statement = connection.prepare(&query).ok()?;
        let rows = statement
            .query_map([session_id], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .ok()?;
        for row in rows.flatten() {
            if let Some(skip_permissions) = shared::text_skip_permissions_mode(&row) {
                return Some(skip_permissions);
            }
        }
    }

    None
}

fn database_path_in_home(home_dir: &Path) -> PathBuf {
    let xdg_data_dir = std::env::var_os("XDG_DATA_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let data_dir = match dirs::home_dir() {
        Some(actual_home) if actual_home == home_dir => {
            xdg_data_dir.unwrap_or_else(|| home_dir.join(".local").join("share"))
        }
        _ => home_dir.join(".local").join("share"),
    };
    data_dir.join("opencode").join("opencode.db")
}

fn find_session_for_path_cached(
    database_path: &Path,
    workspace_path: &Path,
) -> Option<SessionMetadata> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let key = SessionLookupKey {
        database_path: database_path.to_path_buf(),
        workspace_path: workspace_path.clone(),
    };
    let now = Instant::now();

    if let Ok(cache) = session_lookup_cache().lock()
        && let Some(entry) = cache.get(&key)
        && now.saturating_duration_since(entry.checked_at)
            < super::super::OPENCODE_SESSION_LOOKUP_REFRESH_INTERVAL
    {
        return Some(entry.session.clone());
    }

    let session = find_session_for_path(database_path, workspace_path.as_path());
    if let Ok(mut cache) = session_lookup_cache().lock() {
        if let Some(session) = session.as_ref() {
            cache.insert(
                key,
                SessionLookupCacheEntry {
                    checked_at: now,
                    session: session.clone(),
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

    session
}

fn find_session_for_path(database_path: &Path, workspace_path: &Path) -> Option<SessionMetadata> {
    if !database_path.exists() {
        return None;
    }

    let workspace_path = shared::absolute_path(workspace_path)?;
    let state = database_state(database_path)?;

    if let Ok(cache) = session_directory_cache().lock()
        && let Some(entry) = cache.get(database_path)
        && entry.state == state
    {
        return find_matching_session(entry.sessions.as_slice(), workspace_path.as_path());
    }

    let sessions = load_sessions(database_path)?;
    let matched = find_matching_session(sessions.as_slice(), workspace_path.as_path());

    if let Ok(mut cache) = session_directory_cache().lock() {
        let cached_at = Instant::now();
        cache.insert(
            database_path.to_path_buf(),
            SessionDirectoryCacheEntry {
                cached_at,
                state,
                sessions,
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

    matched
}

fn get_last_message_entry(
    database_path: &Path,
    session_id: &str,
) -> Option<(String, MessageRole, i64)> {
    let connection = open_database(database_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, data, time_updated FROM message WHERE session_id = ? ORDER BY time_created DESC LIMIT 1",
        )
        .ok()?;
    let row = statement
        .query_row([session_id], |row| {
            let message_id: String = row.get(0)?;
            let data: String = row.get(1)?;
            let updated_ms: i64 = row.get(2)?;
            Ok((message_id, data, updated_ms))
        })
        .ok()?;
    let role = parse_message_role(&row.1)?;
    Some((row.0, role, row.2))
}

fn open_database(path: &Path) -> Option<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}

fn is_timestamp_recently_updated_ms(updated_ms: i64, threshold: Duration) -> bool {
    let Some(now_ms) = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
    else {
        return false;
    };
    let Some(threshold_ms) = i64::try_from(threshold.as_millis()).ok() else {
        return false;
    };
    now_ms.saturating_sub(updated_ms).max(0) < threshold_ms
}

fn parse_message_role(data: &str) -> Option<MessageRole> {
    let parsed = serde_json::from_str::<MessageData<'_>>(data).ok()?;
    let role = parsed.role?;
    if role == "assistant" {
        return Some(MessageRole::Assistant);
    }
    if role == "user" {
        return Some(MessageRole::User);
    }
    Some(MessageRole::Other)
}

#[cfg(test)]
fn reset_cache_for_test() {
    if let Ok(mut cache) = session_lookup_cache().lock() {
        cache.clear();
    }
    if let Ok(mut cache) = session_directory_cache().lock() {
        cache.clear();
    }
}

fn load_sessions(database_path: &Path) -> Option<Vec<SessionDirectoryEntry>> {
    let connection = open_database(database_path)?;
    let mut statement = connection
        .prepare("SELECT id, directory, time_updated FROM session ORDER BY time_updated DESC")
        .ok()?;
    let rows = statement
        .query_map([], |row| {
            let session_id: String = row.get(0)?;
            let directory: String = row.get(1)?;
            let time_updated_ms: i64 = row.get(2)?;
            Ok(SessionDirectoryEntry {
                session_id,
                directory: PathBuf::from(directory),
                time_updated_ms,
            })
        })
        .ok()?;
    Some(rows.flatten().collect())
}

fn find_matching_session(
    sessions: &[SessionDirectoryEntry],
    workspace_path: &Path,
) -> Option<SessionMetadata> {
    for session in sessions {
        if !shared::cwd_matches(session.directory.as_path(), workspace_path) {
            continue;
        }
        return Some(SessionMetadata {
            session_id: session.session_id.clone(),
            time_updated_ms: session.time_updated_ms,
        });
    }
    None
}

fn database_state(database_path: &Path) -> Option<DatabaseState> {
    let database = file_state(database_path)?;
    let wal_path = opencode_wal_path(database_path);
    let wal = file_state(wal_path.as_path()).unwrap_or(DatabaseFileState {
        modified_at: None,
        length_bytes: 0,
    });
    Some(DatabaseState { database, wal })
}

fn opencode_wal_path(database_path: &Path) -> PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push("-wal");
    PathBuf::from(path)
}

fn file_state(path: &Path) -> Option<DatabaseFileState> {
    let metadata = fs::metadata(path).ok()?;
    let modified_at = metadata.modified().ok();
    Some(DatabaseFileState {
        modified_at,
        length_bytes: metadata.len(),
    })
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

    fn create_opencode_db(database_path: &Path, workspace_path: &Path, session_id: &str) {
        let connection = Connection::open(database_path).expect("database should be created");
        connection
            .execute(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY,
                    directory TEXT NOT NULL,
                    time_updated INTEGER NOT NULL
                )",
                [],
            )
            .expect("session table should be created");
        connection
            .execute(
                "CREATE TABLE message (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    data TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL
                )",
                [],
            )
            .expect("message table should be created");
        connection
            .execute(
                "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                (
                    session_id,
                    workspace_path.to_string_lossy().to_string(),
                    1_i64,
                ),
            )
            .expect("session row should be inserted");
    }

    fn insert_session_row(
        database_path: &Path,
        session_id: &str,
        workspace_path: &Path,
        updated_ms: i64,
    ) {
        let connection = Connection::open(database_path).expect("database should open");
        connection
            .execute(
                "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                (
                    session_id,
                    workspace_path.to_string_lossy().to_string(),
                    updated_ms,
                ),
            )
            .expect("session row should be inserted");
    }

    #[test]
    fn session_lookup_cache_prunes_oldest_entries_when_over_limit() {
        reset_cache_for_test();
        let database_path = PathBuf::from("/tmp/opencode-cache-size.db");
        let now = Instant::now();
        let key_oldest = SessionLookupKey {
            database_path: database_path.clone(),
            workspace_path: PathBuf::from("/tmp/ws-oldest"),
        };
        let key_old = SessionLookupKey {
            database_path: database_path.clone(),
            workspace_path: PathBuf::from("/tmp/ws-old"),
        };
        let key_new = SessionLookupKey {
            database_path,
            workspace_path: PathBuf::from("/tmp/ws-new"),
        };

        let mut cache = session_lookup_cache()
            .lock()
            .expect("session cache lock should succeed");
        cache.insert(
            key_oldest.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(3),
                session: SessionMetadata {
                    session_id: "s-oldest".to_string(),
                    time_updated_ms: 1,
                },
            },
        );
        cache.insert(
            key_old.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(2),
                session: SessionMetadata {
                    session_id: "s-old".to_string(),
                    time_updated_ms: 2,
                },
            },
        );
        cache.insert(
            key_new.clone(),
            SessionLookupCacheEntry {
                checked_at: now - Duration::from_secs(1),
                session: SessionMetadata {
                    session_id: "s-new".to_string(),
                    time_updated_ms: 3,
                },
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
        reset_cache_for_test();
        let root = unique_test_dir("opencode-session-ttl");
        let workspace_path = root.join("workspace");
        let stale_workspace_path = root.join("stale-workspace");
        fs::create_dir_all(&workspace_path).expect("workspace directory should be created");
        fs::create_dir_all(&stale_workspace_path).expect("stale workspace should be created");
        let database_path = root.join("opencode.db");
        create_opencode_db(&database_path, &workspace_path, "session-1");

        let stale_key = SessionLookupKey {
            database_path: database_path.clone(),
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
                    session: SessionMetadata {
                        session_id: "stale".to_string(),
                        time_updated_ms: 0,
                    },
                },
            );

        let found = find_session_for_path_cached(&database_path, &workspace_path);

        assert_eq!(
            found.map(|session| session.session_id),
            Some("session-1".to_string())
        );
        let cache = session_lookup_cache()
            .lock()
            .expect("session cache lock should succeed");
        assert!(!cache.contains_key(&stale_key));
    }

    #[test]
    fn session_directory_cache_invalidates_when_database_state_changes() {
        reset_cache_for_test();
        let root = unique_test_dir("opencode-session-directory-cache");
        let workspace_path = root.join("workspace");
        fs::create_dir_all(&workspace_path).expect("workspace directory should be created");
        let database_path = root.join("opencode.db");
        create_opencode_db(&database_path, &workspace_path, "session-old");

        let first = find_session_for_path(&database_path, &workspace_path)
            .expect("first lookup should resolve");
        assert_eq!(first.session_id, "session-old");

        insert_session_row(&database_path, "session-new", &workspace_path, 2);

        let second = find_session_for_path(&database_path, &workspace_path)
            .expect("lookup after update should resolve");
        assert_eq!(second.session_id, "session-new");
    }
}
