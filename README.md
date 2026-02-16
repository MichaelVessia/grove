# Grove

Minimal workspace manager for AI coding agents (Rust + FrankenTUI).

## Prerequisites

Grove depends on external tools on your `PATH`:

- `git`
- `tmux` (required)
- `lazygit` (required for Git tab)

Quick check:

```bash
command -v git tmux lazygit
```

## Bootstrap

```bash
# Enter pinned toolchain shell
nix develop

# Install git hooks (once per clone)
lefthook install

# Validate clean clone setup
make ci
```

## Commands

```bash
make fmt
make clippy
make test
make ci

# local codex flicker harness (no manual interaction)
scripts/check-codex-flicker.sh

# run harness against real codex instead of fake emitter
GROVE_FLICKER_CODEX_CMD='codex' scripts/check-codex-flicker.sh

# continuous frame+event debug record (writes .grove/debug-record-<app-start>-<pid>.jsonl)
cargo run -- --debug-record
tail -f .grove/debug-record-*.jsonl

# filter input-lag telemetry (seq-linked input -> tmux send -> preview update)
rg '"event":"input"|"event":"preview_update"|"event":"preview_poll"' .grove/debug-record-*.jsonl

# filter git-preview launch latency (first-open lazygit startup vs UI update stall)
rg '"event":"lazygit_launch"|"event":"update_timing"' .grove/debug-record-*.jsonl
```
