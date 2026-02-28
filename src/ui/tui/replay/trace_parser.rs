fn parse_replay_trace(path: &Path) -> io::Result<ReplayTrace> {
    let raw = fs::read_to_string(path)?;

    let mut bootstrap = None;
    let mut messages = Vec::new();
    let mut states = HashMap::new();
    let mut frame_hashes: HashMap<u64, VecDeque<u64>> = HashMap::new();

    for (index, line) in raw.lines().enumerate() {
        let line_number = index.saturating_add(1);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed = serde_json::from_str::<LoggedLine>(trimmed).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSON at line {line_number}: {error}"),
            )
        })?;

        if parsed.event == "replay" && parsed.kind == "bootstrap" {
            let Some(version) = parsed.data.get("schema_version").and_then(Value::as_u64) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing schema_version at line {line_number}"),
                ));
            };
            if version != REPLAY_SCHEMA_VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "unsupported replay schema version {version} at line {line_number}, expected {REPLAY_SCHEMA_VERSION}"
                    ),
                ));
            }
            let Some(snapshot_value) = parsed.data.get("bootstrap") else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing bootstrap payload at line {line_number}"),
                ));
            };
            let snapshot = serde_json::from_value::<ReplayBootstrapSnapshot>(
                snapshot_value.clone(),
            )
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid bootstrap payload at line {line_number}: {error}"),
                )
            })?;
            bootstrap = Some(snapshot);
            continue;
        }

        if parsed.event == "replay" && parsed.kind == "msg_received" {
            let seq = parsed
                .data
                .get("seq")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("missing seq for replay message at line {line_number}"),
                    )
                })?;
            let msg_value = parsed.data.get("msg").ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing msg payload at line {line_number}"),
                )
            })?;
            let msg = serde_json::from_value::<ReplayMsg>(msg_value.clone()).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid replay message payload at line {line_number}: {error}"),
                )
            })?;
            messages.push(ReplayTraceMessage { seq, msg });
            continue;
        }

        if parsed.event == "replay" && parsed.kind == "state_after_update" {
            let seq = parsed
                .data
                .get("seq")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("missing seq for replay state at line {line_number}"),
                    )
                })?;
            let state_value = parsed.data.get("state").ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing state payload at line {line_number}"),
                )
            })?;
            let state = serde_json::from_value::<ReplayStateSnapshot>(state_value.clone())
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid replay state payload at line {line_number}: {error}"),
                    )
                })?;
            states.insert(seq, state);
            continue;
        }

        if parsed.event == "frame" && parsed.kind == "rendered" {
            let Some(replay_seq) = parsed.data.get("replay_seq").and_then(Value::as_u64) else {
                continue;
            };
            if replay_seq == 0 {
                continue;
            }
            let Some(frame_hash) = parsed.data.get("frame_hash").and_then(Value::as_u64) else {
                continue;
            };
            frame_hashes
                .entry(replay_seq)
                .or_default()
                .push_back(frame_hash);
        }
    }

    let Some(bootstrap) = bootstrap else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "trace is missing replay bootstrap event, re-run with --debug-record on a build with replay enabled",
        ));
    };
    if messages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "trace is missing replay message events",
        ));
    }

    messages.sort_by_key(|message| message.seq);

    Ok(ReplayTrace {
        bootstrap,
        messages,
        states,
        frame_hashes,
    })
}
