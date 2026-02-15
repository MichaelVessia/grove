# ADR-008: Zellij Capture via Log Replay and VT Emulation

**Status:** Accepted  
**Date:** 2026-02-14

## Context

tmux provides direct pane capture primitives that Grove can poll. zellij capture
behavior differs and does not provide identical semantics for Grove preview
requirements (ANSI fidelity, resizing, incremental polling behavior).

## Decision

For zellij, capture session output via log file stream and replay it through an
in-process VT emulator to build preview output.

- Maintain per-session capture logs.
- Parse and sanitize script headers/trailers.
- Replay incremental bytes in a terminal engine for rendered preview text.

## Rationale

- Produces consistent ANSI rendering behavior for zellij sessions.
- Supports incremental capture with deterministic replay state.
- Avoids relying on weaker or mismatched direct capture behavior.

## Consequences

- Grove owns emulator state management per zellij session.
- Resize behavior must keep emulator dimensions aligned with pane size.
- zellij preview bugs are debugged in replay/emulator logic, not only command
  execution.

## Related Commits

- `6858014`
- `135c6fc`
