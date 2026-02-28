---
name: replay-debug
description: >
  Replay and analyze Grove debug-record traces. Use when debugging runtime
  regressions, polling races, interactive input bugs, state transitions,
  frame/render mismatches, or when user asks to analyze `.grove/debug-record`.
  Not for instrumentation design, use `logging-best-practices` when adding or
  changing log events/fields.
  Trigger phrases: "replay", "debug record", "analyze trace", "runtime race",
  "timing bug", "state mismatch".
allowed-tools: Read, Grep, Glob, Task
---

# Grove Replay Debug Skill

Use this skill when a bug report involves runtime behavior and timing.

If the task is adding/modifying logging events, switch to
`.agents/skills/logging-best-practices/SKILL.md`.

## Objective

Convert reported runtime issues into deterministic replay failures, fix against
the failing sequence, and confirm replay passes on the same trace.

## Inputs

- Preferred: explicit trace path (for example `.grove/debug-record-*.jsonl`)
- If missing: ask user to record one with `cargo run -- --debug-record`

## Workflow

1. Locate trace:
   - `TRACE="$(ls -t .grove/debug-record-*.jsonl | head -n 1)"`
2. Run strict replay first:
   - `cargo run -- replay "$TRACE"`
3. If strict replay fails, note failing replay sequence from output.
4. Inspect nearby events:
   - `rg -n '"event":"replay"|"replay_seq"|\"seq\":<N>' "$TRACE"`
5. If strict replay fails on frame hash and behavior may still be correct:
   - `cargo run -- replay "$TRACE" --invariant-only`
6. For deeper diff context:
   - `cargo run -- replay "$TRACE" --snapshot .grove/replay-snapshot.json`
7. Implement fix and rerun strict replay on same trace.
8. For regression inputs:
   - `cargo run -- replay "$TRACE" --emit-test <fixture-name>`
   - adds `tests/fixtures/replay/<fixture-name>.jsonl`

## Reporting

When handing off, include:

- Trace path used
- Replay command(s) run
- Failing sequence before fix (if any)
- Replay result after fix
- Any fixture emitted (`tests/fixtures/replay/...`)

## Guardrails

- Do not debug runtime issues from partial logs alone when replay trace is available.
- Keep the original trace unchanged, copy via `--emit-test` for fixtures.
- Prefer strict replay as source of truth, use `--invariant-only` only to isolate rendering-only drift.
