---
name: cli-design
description: >
  Design Grove's agent-first CLI contract (Rust) with JSON-only envelopes,
  HATEOAS-style `next_actions`, deterministic error codes, and context-safe
  output. Use when adding CLI commands, refining response schemas, or reviewing
  agent automation ergonomics for lifecycle operations.
  Trigger phrases: "cli design", "agent-first cli", "HATEOAS", "next_actions",
  "json envelope", "root command tree".
args: "[command or contract question]"
allowed-tools: Read, Grep, Glob
---

# Grove Agent-First CLI Design (Rust)

Use this when defining or reviewing CLI behavior in Grove.

## Query

$ARGUMENTS

## Scope And Sources

Source priority:
1. `docs/PRD.md`
2. `docs/agent-first-lifecycle-plan.md`
3. Existing Rust code (`src/`)

Scope guard:
- Grove is Rust-only.
- No Effect/Bun/TypeScript assumptions.
- No streaming protocol design in lifecycle v1 unless explicitly requested.

## Contract Baseline (must hold)

Every command returns JSON only, one envelope:

Success envelope:
- `ok: true`
- `command: string`
- `result: object`
- `warnings?: string[]` (present only when non-empty)
- `next_actions: [{ command: string, description: string }]`

Error envelope:
- `ok: false`
- `command: string`
- `error: { code: string, message: string }`
- `fix: string`
- `warnings?: string[]` (present only when non-empty)
- `next_actions: [{ command: string, description: string }]`

Output rules:
- No plain text mode.
- No ANSI formatting.
- Truncate large output safely, include a pointer to full output when truncated.

## HATEOAS In Grove

`next_actions` is required on success and error.

Design rules:
1. Keep each action directly runnable by an agent.
2. Keep commands contextual to current state (different on success vs failure).
3. Prefer concrete command strings over abstract prose.
4. Keep shape minimal in v1 (`command`, `description` only).

Do not add a richer `params` schema unless a new decision explicitly approves it.

## Command Discoverability

Root `grove` (no args) should return:
1. Command tree metadata.
2. Capability snapshot.
3. Minimal usage templates per command.

Recommendation:
- Include usage templates, skip long examples in v1 to keep payload small.
- If examples are needed later, add compact single-line examples only.

## Decision Guidance For Current Open Questions

1. PRD single-repo vs daemon multi-repo:
- Treat multi-repo daemon namespacing as post-v1 expansion, not a v1 behavior change.
- Keep local lifecycle v1 single-repo semantics intact.
- Document this explicitly in phase headers to avoid silent scope creep.

2. Root `grove` JSON shape:
- Yes, this skill supports (2).
- Recommended v1: command tree + capability snapshot + per-command usage template.
- Defer verbose examples/extended docs to a later phase.

3. Dry-run `plan.steps` identity:
- Add stable `step_id` in addition to `index`.
- `index` is presentation order, `step_id` is machine-stable identity.
- This reduces brittle automation when step ordering changes.

## Error Model Requirements

Use stable machine codes (for example):
- `INVALID_ARGUMENT`
- `WORKSPACE_INVALID_NAME`
- `REPO_NOT_FOUND`
- `WORKSPACE_NOT_FOUND`
- `WORKSPACE_ALREADY_EXISTS`
- `CONFLICT`
- `TMUX_COMMAND_FAILED`
- `GIT_COMMAND_FAILED`
- `IO_ERROR`
- `INTERNAL`

Rules:
1. Keep mapping deterministic.
2. Keep `fix` actionable.
3. Keep exit code policy stable (`0`, `1`, `2`).

## Rust Implementation Guidance

Use typed structs/enums with `serde` for envelopes and error codes.

Preferred patterns:
1. Shared DTO module for request/response contracts.
2. Single builder path for `next_actions`.
3. Central error classifier function with ordered precedence.
4. Integration tests over CLI binary output plus unit tests for mapping/builders.

Avoid:
1. Ad-hoc JSON object assembly spread across handlers.
2. Stringly-typed error code branching in many files.
3. Divergent output schemas between commands.

## Review Checklist

- [ ] Output is always valid JSON envelope.
- [ ] `next_actions` present and contextual.
- [ ] Error responses include deterministic `error.code` and concrete `fix`.
- [ ] Root command remains self-discoverable.
- [ ] Potentially large outputs are truncated safely with full-output pointer.
- [ ] Tests cover schema shape, error mapping, and selector/validation behavior.
