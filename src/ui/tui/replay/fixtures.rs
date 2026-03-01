use super::*;

pub fn emit_replay_fixture(trace_path: &Path, name: &str) -> io::Result<PathBuf> {
    let sanitized = sanitize_fixture_name(name);
    if sanitized.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture name must contain at least one alphanumeric character",
        ));
    }

    let fixture_dir = PathBuf::from(REPLAY_FIXTURE_DIRECTORY);
    fs::create_dir_all(&fixture_dir)?;
    let target = fixture_dir.join(format!("{sanitized}.jsonl"));
    fs::copy(trace_path, &target)?;
    Ok(target)
}

pub(crate) fn replay_config_path() -> PathBuf {
    std::env::temp_dir().join(format!("grove-replay-config-{}.toml", std::process::id()))
}

pub(crate) fn sanitize_fixture_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
            continue;
        }
        if ch == '-' || ch == '_' {
            sanitized.push(ch);
        }
    }
    sanitized
}
