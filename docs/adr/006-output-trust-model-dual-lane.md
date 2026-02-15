# ADR-006: Output Trust Model, Dual Lane Processing

**Status:** Accepted  
**Date:** 2026-02-13

## Context

Grove must render rich ANSI output from agent sessions while also deriving safe
status signals (active/waiting/done/error). Using one shared output stream for
both rendering and logic risks either fidelity loss or control-path coupling to
escape sequences.

## Decision

Use dual lane output processing.

- Render lane: preserve ANSI-safe raw output for preview fidelity.
- Logic lane: use cleaned output for status/change detection and control logic.

## Rationale

- Keeps terminal fidelity (colors/cursor behavior) in preview.
- Prevents control sequence noise from affecting status detection logic.
- Makes trust boundaries explicit in runtime behavior and tests.

## Consequences

- Poll processing tracks both render output and cleaned output digests.
- Status inference must only read cleaned lane data.
- Regressions are tested at both runtime and app layers.

## Related Commits

- `eac1a07`
- `3ad781f`
- `d1eb48d`
