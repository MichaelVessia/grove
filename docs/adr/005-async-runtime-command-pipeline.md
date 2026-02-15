# ADR-005: Async Runtime Command Pipeline

**Status:** Accepted  
**Date:** 2026-02-13

## Context

Grove runs in a single TUI event loop. Blocking git or multiplexer commands in
update/key paths can stall rendering, increase input latency, and create output
ordering issues.

## Decision

Run runtime subprocess work asynchronously and apply results through explicit
completion messages.

- Use background tasks for preview polling and lifecycle operations.
- Use generation IDs for poll requests and drop stale results.
- Keep update/key paths non-blocking.

## Rationale

- Preserves responsiveness under polling and interactive typing.
- Makes stale poll races explicit and testable.
- Aligns with ftui command/subscription model and one-writer discipline.

## Consequences

- Runtime state includes in-flight and generation tracking.
- Failures are surfaced through completion handlers and logs, not direct command
  return paths.
- Tests must cover async ordering and stale-result dropping behavior.

## Related Commits

- `6f4394f`
- `a862316`
- `6b859bb`
- `9844902`
- `f34f2f8`
