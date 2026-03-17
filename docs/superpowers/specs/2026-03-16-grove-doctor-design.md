# Grove Doctor Design

## Summary

`grove doctor` is a diagnosis-only CLI for existing Grove users.

It audits Grove-managed state across task manifests, configured projects, git
worktree paths, and Grove-managed tmux sessions, then prints:

- a compact summary
- explicit findings with evidence
- a deterministic repair plan for a human or coding agent to execute

V1 does not mutate anything. No `--apply`, no startup self-healing, no TUI
surface.

## Goals

- Give existing users one trusted entrypoint for "what is wrong with Grove
  state right now?"
- Replace scattered migration/debug knowledge with one concrete diagnosis flow.
- Produce repair guidance that an agent can execute safely.
- Keep Grove task-first. Diagnose state drift without reintroducing workspace-era
  runtime compatibility.
- Be safe to run at any time against live tmux state.

## Non-Goals

- Applying repairs.
- Automatically mutating manifests, sessions, or git state.
- Adding a TUI repair modal in v1.
- Rebuilding old runtime compatibility paths.
- Diagnosing arbitrary non-Grove git or tmux issues outside Grove-owned state.

## User Decisions

- V1 is diagnosis-only.
- Repair output is the product. Execution remains external.
- Prioritize low scope and high trust over automation.
- Optimize for existing users with live tasks and legacy drift risk.

## Current Problem

Today Grove has the pieces needed to reason about broken state, but they are
spread across multiple places:

- task discovery and manifest decoding
- refresh-time base-task materialization
- session cleanup planning
- migration docs for task manifests and legacy tmux sessions

That is workable for contributors, but poor for users. Existing users do not
need another feature surface. They need one command that answers:

1. What is broken?
2. Why does Grove think it is broken?
3. What should be done, in what order?

## Accepted Approach

Add a new CLI mode:

```text
grove doctor
grove doctor --json
```

Default mode prints human-readable text.

`--json` emits a machine-readable diagnosis envelope so another agent can turn
the findings into actions.

The command never writes files, changes tmux metadata, kills sessions, or edits
git state.

## User Experience

### Human Output

The default CLI output is ordered for fast triage:

1. overall status summary
2. grouped findings
3. ordered repair plan

Example shape:

```text
doctor: 5 findings (2 error, 3 warn)

errors
- invalid manifest: /Users/me/.grove/tasks/foo/.grove/task.toml
  reason: unsupported workspace status "working"
- missing worktree path: task=bar repo=web
  path: /Users/me/.grove/tasks/bar/web

warnings
- missing base-task manifest for configured repo
  repo: /Users/me/src/api
- orphaned grove tmux session
  session: grove-wt-old-api

repair plan
1. Rewrite or remove invalid manifest at ...
2. Recreate or delete missing worktree entry for task ...
3. Materialize a base-task manifest for configured repo ...
4. Kill orphaned tmux session ...
```

### JSON Output

`--json` emits structured data with stable top-level keys:

- `summary`
- `findings`
- `repair_plan`

Each finding includes:

- `severity`
- `kind`
- `subject`
- `evidence`
- `recommended_action`

The JSON does not need a formal versioned public API in v1, but it should be
stable enough for Grove-driven agent workflows and tests.

## Findings Model

### Severity

- `info`
- `warn`
- `error`

Use `error` when Grove state is invalid or contradictory.

Use `warn` when Grove can still function but drift or leftovers should be
addressed.

### Initial Finding Kinds

V1 should cover the highest-value, lowest-ambiguity classes:

- `invalid_task_manifest`
- `duplicate_task_slug`
- `missing_worktree_path`
- `missing_base_marker`
- `configured_repo_missing_base_task_manifest`
- `orphaned_grove_session`
- `stale_auxiliary_session`
- `legacy_grove_session_missing_metadata`
- `manifest_repository_mismatch`

This list is intentionally constrained. V1 should prefer a strong core taxonomy
 over a wide but fuzzy audit.

### Subject Shape

A finding subject identifies the broken thing directly. Depending on kind, it
may include:

- `task_slug`
- `manifest_path`
- `repository_path`
- `worktree_path`
- `session_name`

## Repair Plan Model

The repair plan is derived from findings, not discovered independently.

Each step contains:

- `priority`
- `action`
- `reason`
- `targets`

Actions are recommendation labels, not executable Grove subcommands. Initial
actions:

- `inspect_or_rewrite_manifest`
- `remove_duplicate_manifest_owner`
- `restore_or_remove_missing_worktree`
- `write_base_marker`
- `materialize_base_task_manifest`
- `kill_or_adopt_session`

Rules:

- deterministic ordering
- no duplicate steps for the same root cause
- action wording must stay concrete enough for an agent to act on directly

## Architecture

### New Application Service

Add a dedicated diagnosis service in `src/application/doctor.rs`.

Responsibilities:

- collect Grove-relevant state from existing sources
- produce findings
- derive repair plan steps
- expose plain Rust data structures for CLI rendering and tests

The service should be pure where possible. External state collection should be
isolated behind small helpers so the reasoning layer can be tested with fixtures
instead of shelling out end-to-end for every case.

### Reused Existing Sources

Doctor should reuse current Grove logic rather than inventing parallel parsing
rules:

- task manifest decode from `src/infrastructure/task_manifest.rs`
- project config loading from `src/infrastructure/config.rs`
- task discovery assumptions from `src/application/task_discovery.rs`
- base-task materialization rules from task lifecycle/bootstrap code
- session cleanup classification from `src/application/session_cleanup.rs`

The audit should share Grove's task-first truth model:

- manifests are canonical
- configured repos may imply missing base-task manifests
- Grove-managed tmux sessions are identified by canonical task/worktree naming

### CLI Integration

Extend `src/cli/mod.rs` with a new command mode:

- `doctor`
- optional `--json`

The CLI layer should:

- parse args
- invoke the diagnosis service
- render either human text or JSON
- return non-zero on `error` findings

Recommended exit behavior:

- `0` when no findings or only `info`
- `1` when any `warn` or `error` findings exist
- `2` only for command/runtime failure, not diagnosis results

This keeps doctor useful in automation without claiming the system is healthy
when actionable drift exists.

## Data Flow

1. Resolve configured projects and task root paths.
2. Enumerate task manifests under the task root.
3. Decode valid manifests and capture parse/validation failures as findings.
4. Cross-check manifest worktrees against filesystem expectations.
5. Cross-check configured repositories against manifest-backed base tasks.
6. Enumerate Grove-managed tmux sessions.
7. Classify orphaned/stale/legacy session findings.
8. Reduce findings into ordered repair steps.
9. Render summary plus findings plus repair plan.

## Error Handling

- If config loading fails, return command failure, not a finding.
- If tmux is unavailable, doctor should still run and emit a warning that session
  checks were skipped.
- If a manifest cannot be parsed, keep auditing the rest of the tree.
- If one repository path no longer exists, emit a finding for that repo and keep
  scanning the others.

Doctor should degrade gracefully. Partial diagnosis is better than a hard stop.

## Testing Strategy

Cover the service in layers.

### Unit-Style Diagnosis Tests

Fixture-driven tests for:

- invalid manifest parse
- duplicate slug detection
- missing worktree path
- missing `.grove/base`
- configured repo missing a base-task manifest
- orphaned Grove session classification
- legacy session without metadata classification
- repair-plan deduplication and ordering

### CLI Tests

Focused tests for:

- `doctor` arg parsing
- human output summary shape
- `--json` output shape
- exit status for healthy vs unhealthy diagnosis

### Regression Principle

Tests should assert behavior, not internal helper arrangement. The contract is:

- diagnosis remains non-mutating
- findings remain concrete
- repair plan remains deterministic

## Files Likely To Change

- `src/application/mod.rs`
- `src/application/doctor.rs`
- `src/application/session_cleanup.rs`
- `src/cli/mod.rs`
- `src/main.rs`
- `README.md`

Tests likely in:

- `src/application/doctor.rs`
- `src/cli/mod.rs`

## Open Questions

None blocking for v1.

Deferred explicitly:

- whether doctor should later gain `--apply`
- whether doctor should gain a TUI surface
- whether JSON output should become a versioned public schema
