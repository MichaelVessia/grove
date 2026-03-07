# Task Scale Benchmarks

Issue: [#38](https://github.com/MichaelVessia/grove/issues/38)

## Command

```bash
# Human-readable report
cargo run -- benchmark-scale

# Emit JSON report
cargo run -- benchmark-scale --json

# Write a baseline file
cargo run -- benchmark-scale --write-baseline docs/workspace-scale-baseline.json

# Compare against baseline (informational warnings only)
cargo run -- benchmark-scale --baseline docs/workspace-scale-baseline.json
```

Notes:
- Benchmark checks are informational today, they do not fail the command.
- Severe regressions are flagged when p95 exceeds baseline by default 35%.
- Override threshold with `--warn-regression-pct <N>`.

## Flows Measured

- `discovery`: parse synthetic `git worktree --porcelain` + branch activity, build task-shaped fixtures, read marker metadata, sort.
- `status-target-generation`: build tmux status polling targets for task worktrees (with live-preview exclusion).
- `sort-update-pipeline`: reconcile sessions, compute status targets, apply status detection updates, re-sort task worktrees.

## Baseline (2026-03-02 01:49:43 UTC)

Environment:
- OS: `Linux 6.12.74 x86_64 GNU/Linux`
- Rust: `rustc 1.93.1 (01f6ddf75 2026-02-11)`
- Package version: `0.1.0`
- Warmup runs: `2`
- Measured runs: `15`
- Task counts: `10, 100, 500`

| N | Flow | p50 (ms) | p95 (ms) |
|---:|---|---:|---:|
| 10 | discovery | 0.058 | 0.080 |
| 10 | status-target-generation | 0.007 | 0.025 |
| 10 | sort-update-pipeline | 17.694 | 18.901 |
| 100 | discovery | 0.641 | 0.658 |
| 100 | status-target-generation | 0.061 | 0.062 |
| 100 | sort-update-pipeline | 352.631 | 407.856 |
| 500 | discovery | 3.317 | 3.449 |
| 500 | status-target-generation | 0.301 | 0.305 |
| 500 | sort-update-pipeline | 1946.945 | 2073.642 |

Baseline JSON:
- [docs/workspace-scale-baseline.json](docs/workspace-scale-baseline.json)
