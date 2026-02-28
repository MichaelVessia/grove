# Debug Replay Workflow

Use replay to turn a timing-sensitive bug report into a deterministic,
executable trace.

## What Replay Does

- Records full runtime event stream in `.grove/debug-record-*.jsonl`
- Replays recorded `Msg` flow headlessly
- Compares state snapshots and frame hashes (unless `--invariant-only`)
- Stops on first divergence with the failing replay sequence

## Human Quickstart

1. Capture a trace:

```bash
cargo run -- --debug-record
```

2. Reproduce the issue in the TUI, quit, then replay:

```bash
TRACE="$(ls -t .grove/debug-record-*.jsonl | head -n 1)"
cargo run -- replay "$TRACE"
```

3. Optional modes:

```bash
# Skip frame-hash strictness, keep invariants + state checks
cargo run -- replay "$TRACE" --invariant-only

# Write replay snapshot for review/diff
cargo run -- replay "$TRACE" --snapshot .grove/replay-snapshot.json

# Copy trace as fixture input
cargo run -- replay "$TRACE" --emit-test flow-name
```

## Failure Triage

1. Start with strict replay:
   - `cargo run -- replay "$TRACE"`
2. If it fails, capture the replay sequence from error output.
3. Inspect nearby replay records in trace:
   - `rg -n '"event":"replay"|"replay_seq"|\"seq\":<N>' "$TRACE"`
4. Re-run with `--snapshot` if you need state-by-state diffs.
5. Fix code, replay same trace until green.

## Agent Workflow

1. Request or generate a debug-record trace first, do not debug from summary logs alone.
2. Run strict replay before changing code.
3. If strict fails on frame hash but state is plausible, run `--invariant-only` to separate render drift from behavioral drift.
4. Use failing replay sequence as the primary debugging anchor.
5. After fix:
   - replay same trace (strict)
   - rerun modified tests
6. For regression coverage:
   - promote trace with `--emit-test`
   - add/adjust tests around the failing behavior

## Notes

- Replay expects traces containing `replay/bootstrap`, `replay/msg_received`,
  and `replay/state_after_update` events.
- Older debug-record files without replay events are not replayable.
