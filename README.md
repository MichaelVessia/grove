# Grove

Minimal workspace manager for AI coding agents (Rust + FrankenTUI).

## Bootstrap

```bash
# Enter pinned toolchain shell
nix develop

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
```
