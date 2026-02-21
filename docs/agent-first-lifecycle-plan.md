# Grove Agent-First Lifecycle Plan

## Goal

Make every Grove lifecycle capability agent-callable (CLI first), then evolve to daemon + socket transport, then remote.

Primary success criterion:
- An agent can run basic workflows end-to-end, for example `create workspace`, `start agent`, `update/merge`, `stop/delete`, and changes are reflected in Grove TUI.

## Kickoff Snapshot (2026-02-21)

- Current execution state: `in progress` (`Phase 0a` and `Phase 0` completed).
- Immediate focus: Phase 1 (`migrate existing lifecycle call sites to command service internals`).
- Tracking rule: a phase is complete only when deliverables, tests, and exit criteria all pass.
- Scope guard for kickoff: satisfied, no daemon/remote code started before Phase 0a-3 completion.

## Resolved Decisions (2026-02-21)

- PRD single-repo remains v1 behavior. Multi-repo daemon namespacing is explicitly post-v1 expansion only.
- Root `grove` JSON includes command tree metadata, capability snapshot, and compact per-command usage templates (no verbose examples in v1).
- Dry-run `plan.steps` includes stable `step_id` and ordered `index`.

## Decisions Captured

- Scope v1: lifecycle only (no raw interactive keystroke streaming API).
- Expose all lifecycle actions (not partial subset).
- CLI response model: JSON-only, HATEOAS style (`next_actions`), per `cli-design` skill.
- Idempotency: repeated `agent start/stop` on same state is no-op success.
- Auto-refresh in TUI: every 2 seconds, always on, no toast.
- Local first: single-process/in-process command API first, no daemon yet.
- Repo scope for future daemon: one daemon serving many repos (namespaced by repo path).
- Repo identity: filesystem path (not abstract ID) is acceptable.
- PRD alignment: local lifecycle v1 remains single-repo; multi-repo behavior starts only in post-v1 daemon phases.
- Remote target (later): SSH tunnel path first.
- Backward compatibility: not required.
- Testing: heavy validation each phase.
- Mixed mode: local and remote projects must coexist in same app session.
- Remote profile management should live in Grove UI settings (after initial manual phase).
- Remote outages must not block local project workflows.
- Root command cutover: `grove` returns JSON command tree, `grove tui` launches UI.
- Structure strategy: no big-bang prework restructure, do incremental boundary extraction inside this plan.
- Packaging strategy: keep one crate for v1, add module boundaries now, evaluate separate crates only after service boundary stabilizes.

## Defaults Assumed (from recommendations accepted)

- `--repo` optional, defaults to `cwd`.
- Mutating commands support both `--workspace <name>` and `--workspace-path <path>`.
- For mutating commands that target an existing workspace, selector is required (no implicit `cwd` selection).
- If both selectors are provided and resolve to same workspace, allow.
- If both selectors are provided and resolve to different workspaces, return `INVALID_ARGUMENT` and exit `2`.
- `workspace create` does not auto-start by default (separate `agent start`), optional `--start`.
- `workspace delete` default keeps branch, optional branch deletion flag.
- `workspace merge` default no cleanup, optional cleanup flags.
- `workspace edit` v1 supports `agent` + `base_branch`.
- Error envelope includes stable `error.code` now.
- `next_actions` required on every success and error, machine-usable shape:
  - `{ command: string, description: string }`
- Exit codes: `0` success, `1` runtime/domain failure, `2` usage/validation failure.

## Important Atomicity Constraint

Strict ACID atomicity is not possible across git + tmux + filesystem side effects.

Adopt transaction-like behavior:
- Validate up front.
- Execute ordered steps.
- On failure, run best-effort compensating actions.
- Return structured `warnings` when compensation is partial.

Command-level policy:
- `workspace create --start`:
  - If create succeeds and agent start fails, keep workspace.
  - Return error (`ok: false`) with deterministic `error.code`.
  - Include `warnings` + `next_actions` for recovery (`agent start`, optional `workspace delete`).
- `workspace delete --force-stop`:
  - If stop fails, delete is not attempted.
  - Return error (best available atomic behavior for this operation).
- `workspace merge --cleanup-*`:
  - Merge is primary/irreversible action.
  - If merge succeeds and cleanup fails, return success (`ok: true`) with `warnings`.

## CLI Contract (Phase 0 Baseline)

All commands return a single JSON envelope:

Success:
- `ok: true`
- `command: string`
- `result: object`
- `warnings: string[]` (optional, present when non-empty)
- `next_actions: [{ command: string, description: string }]`

Error:
- `ok: false`
- `command: string`
- `error: { code: string, message: string }`
- `fix: string`
- `warnings: string[]` (optional, present when non-empty)
- `next_actions: [{ command: string, description: string }]`

Output discipline:
- No plain text.
- No ANSI formatting.
- Context-safe truncation for large lists/outputs, include pointer to full output when truncated.

Dry-run contract:
- `--dry-run` performs full validation, executes zero side effects.
- Result includes:
  - `dry_run: true`
  - `plan.steps: [{ step_id: string, index: number, summary: string }]`
  - `predicted_effects: string[]`
  - `warnings: string[]`
- Dry-run exit behavior:
  - validation failure -> exit `2`, `ok: false`
  - executable plan (even if runtime risk predicted) -> exit `0`, `ok: true`

## Proposed Command Surface (v1 local automation)

- `grove` (root) -> JSON command tree + capability snapshot + compact usage templates per command.
- `grove tui [--repo <path>]` -> launch TUI for repo.
- `grove workspace list [--repo <path>]`
- `grove workspace create --name <name> [--base <branch> | --existing-branch <branch>] [--agent <claude|codex>] [--start] [--dry-run] [--repo <path>]`
- `grove workspace edit [--workspace <name> | --workspace-path <path>] [--agent <...>] [--base <branch>] [--repo <path>]`
- `grove workspace delete [--workspace <name> | --workspace-path <path>] [--delete-branch] [--force-stop] [--dry-run] [--repo <path>]`
- `grove workspace merge [--workspace <name> | --workspace-path <path>] [--cleanup-workspace] [--cleanup-branch] [--dry-run] [--repo <path>]`
- `grove workspace update [--workspace <name> | --workspace-path <path>] [--dry-run] [--repo <path>]`
- `grove agent start [--workspace <name> | --workspace-path <path>] [--prompt <text>] [--pre-launch <cmd>] [--skip-permissions] [--dry-run] [--repo <path>]`
- `grove agent stop [--workspace <name> | --workspace-path <path>] [--dry-run] [--repo <path>]`

Selector resolution rules:
- Applies to: `workspace edit|delete|merge|update`, `agent start|stop`.
- Exactly one selector is recommended.
- If both selectors passed:
  - resolve each in selected repo scope
  - if same workspace identity, continue
  - if different, return `INVALID_ARGUMENT` (`exit 2`)
- If neither selector passed, return `INVALID_ARGUMENT` (`exit 2`).

## Mixed Local/Remote Project Model

Project target model:
- `local`
- `remote:<profile_name>`

UI requirements:
- Project list shows clear target indicator:
  - Local badge: `L`
  - Remote badge: `R:<profile>`
- Show remote connection state per profile:
  - `connected`
  - `degraded`
  - `offline`
- Local and remote projects appear in one list and can be filtered:
  - `all`
  - `local`
  - `remote`

Routing rules:
- Actions against local projects always use local in-process backend/socket.
- Actions against remote projects use selected remote profile transport.
- Remote connection failures never block local actions.

Failure behavior:
- Remote project mutations on disconnected profile return structured error:
  - `error.code = REMOTE_UNAVAILABLE`
  - `fix` explains reconnect path
  - `next_actions` include reconnect/switch suggestions
- Remote projects remain visible while offline.

Identity rules:
- Project identity key includes target + path to avoid collisions.
- Same repo path can exist as both local and remote entries without conflict.

## Architecture Strategy

Do not jump to client/server first. Separate concerns in this order:

1. Define command contract.
2. Extract application command service (in-process).
3. Bind CLI to service.
4. Rewire TUI to same service.
5. Add daemon transport without changing command semantics.

This avoids dual implementation and keeps behavior parity.

## Phased Plan

## Feedback Loop Contract (required)

For every PR-sized slice:
1. Make the smallest possible change set for one checklist item.
2. Run targeted tests only for touched modules first.
3. Fix failures before moving to next slice.
4. Record pass/fail evidence in PR notes (commands + result summary).

Per-phase completion gate:
1. All phase deliverables implemented.
2. All listed phase tests passing.
3. CI parity checks passing:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test`
4. Exit criteria marked complete.

Refactor safety rule:
- Do not batch multiple unchecked slices.
- Land or validate each slice before starting the next one.

## Phase 0a, Module Boundary Guardrails

Deliverables:
- Add `interface::cli` module boundary for CLI argument parsing + JSON output wiring.
- Add `interface::tui` module boundary for TUI startup/wiring.
- Introduce `application::commands` service surface (trait + DTO stubs only, no behavior migration yet).
- Remove new cross-layer references that violate target direction:
  - `interface -> application -> domain`
  - `infrastructure` must not depend on `interface`.

Implementation notes:
- Keep behavior unchanged in this phase, this is structural extraction only.
- Keep existing files where practical, move minimally and re-export to avoid churn.
- Do not split into multiple crates in v1.

Tests:
- Compile + existing targeted regression tests for startup/CLI smoke still pass.
- Add module visibility/import tests as needed to enforce boundary compile checks.

Exit criteria:
- Phase 1 can add command service logic without reworking entrypoint structure.
- New CLI implementation can be developed under `interface::cli` without touching TUI runtime code.

Phase 0a implementation slices (PR-sized):
- [x] P0a.1 Add `interface` module with `cli` and `tui` submodules, move current entry wiring.
- [x] P0a.2 Add `application::commands` trait + request/response stubs.
- [x] P0a.3 Remove/avoid new cross-layer dependencies and lock with compile-time checks/tests.

## Phase 0, Contract + Types

Deliverables:
- Add CLI envelope types and error code taxonomy.
- Add shared command request/response DTOs for lifecycle operations.
- Add root command-tree response schema.
- Include compact usage templates in root command-tree payload.
- Document command templates for `next_actions` (`command` + `description` shape).
- Document dry-run response schema (`plan.steps.step_id`, `plan.steps.index`, `predicted_effects`, `warnings`).

Implementation notes:
- New module for envelope + `next_actions` builders.
- Centralize error-code mapping from existing lifecycle errors.

Tests:
- Snapshot/shape tests for success and error envelopes.
- Error code stability tests (mapping table).
- Root command tree completeness tests (all commands listed).
- Root command tree usage template tests (all commands include compact usage).
- Envelope tests asserting `next_actions` object shape.
- Dry-run envelope tests (success + validation failure).
- Dry-run step identity tests (`step_id` stable, `index` ordered).
- Error classifier precedence tests using real failure strings from workspace/git/tmux paths.

Exit criteria:
- JSON contract frozen for Phase 1 and Phase 2.

Phase 0 implementation slices (PR-sized):
- [x] P0.1 Add envelope types + serializer tests.
- [x] P0.2 Add error code enum + classifier mapping table tests.
- [x] P0.3 Add `next_actions` builder helpers + shape tests.
- [x] P0.4 Add root command-tree schema (with compact usage templates) + completeness tests.
- [x] P0.5 Add dry-run response schema (`step_id` + `index`) + validation failure tests.

## Phase 1, In-Process Command Service

Deliverables:
- Introduce `application::commands` service that wraps existing lifecycle/runtime functions.
- Service methods for each lifecycle command.
- Transaction-like execution with compensating actions and warning capture.

Implementation notes:
- Reuse existing logic from:
  - `src/application/workspace_lifecycle.rs`
  - `src/application/agent_runtime.rs`
  - `src/infrastructure/adapters.rs`
- Keep tmux/git execution in existing adapters/runners, no new behavior fork.
- Keep domain validation first, then execution.

Tests:
- Unit tests per command path (success, no-op, validation failure, compensation path).
- Regression tests for existing lifecycle behavior parity.
- Dry-run tests (no side effects asserted).

Exit criteria:
- CLI and TUI can call same command service API.

## Phase 2, CLI Implementation (Local, no daemon)

Deliverables:
- Implement CLI command tree over command service.
- JSON-only output everywhere.
- `next_actions` generated contextually for success/failure.
- `--repo` + workspace selector resolution rules.

Implementation notes:
- Root command becomes self-documenting JSON.
- TUI launched explicitly via `grove tui`.
- Mutating commands return result payload including warnings array and selected workspace identity.
- No compatibility shim for old root behavior.

Tests:
- Integration tests invoking CLI binaries for each command.
- Golden tests for envelope structure and `next_actions`.
- Failure tests with deterministic error codes and fixes.
- Repo path resolution tests (`cwd` default + explicit `--repo`).
- Selector resolution tests:
  - both selectors same workspace -> success
  - both selectors mismatch -> `INVALID_ARGUMENT`, exit `2`
  - no selector for mutating target ops -> `INVALID_ARGUMENT`, exit `2`
- `workspace create --start` partial failure test:
  - workspace preserved on start failure
  - error envelope includes recovery `next_actions`

Exit criteria:
- Agent can fully drive lifecycle workflow via CLI alone.

## Phase 3, TUI Integration + External Change Reflection

Deliverables:
- TUI lifecycle actions call command service (remove direct orchestration duplication).
- Add periodic workspace refresh every 2s (always-on) so external CLI mutations appear.
- Maintain current UX behavior for toasts/status where relevant.

Implementation notes:
- Refactor TUI lifecycle call sites to service boundary.
- Keep preview/status polling behavior, add inventory refresh cadence.

Tests:
- TUI runtime flow tests for parity.
- Integration test: run CLI mutation, assert TUI state reflects on refresh cycle.
- Performance guard: refresh cadence does not regress interactive responsiveness.

Exit criteria:
- One behavior source for lifecycle logic, UI mirrors external automation reliably.

## Phase 4, Daemon + Unix Socket Transport

Deliverables:
- Introduce `groved` daemon process with Unix socket (for example `~/.grove/groved.sock`).
- Move command service execution into daemon.
- Add thin client mode in CLI/TUI transport layer (same request/response schema).

Implementation notes:
- Keep command semantics unchanged from Phase 2.
- Namespace all operations by repo path.
- Enforce repo allowlist in daemon config.
- Local auth via filesystem permissions first.

Tests:
- Daemon lifecycle tests (startup/shutdown, stale socket handling).
- IPC protocol tests (request/response parity with in-process mode).
- Multi-client tests (concurrent read + serialized writes behavior).

Exit criteria:
- CLI and TUI both operate against daemon with no semantic drift.

## Phase 4.5, Remote UX in TUI (Single-User SSH Profile)

Deliverables:
- Add `Remotes` settings surface in TUI.
- Remote profile fields:
  - `name`
  - `host`
  - `user`
  - `remote_socket_path`
  - optional `default_repo_path`
- Add profile actions:
  - `connect`
  - `disconnect`
  - `test`
- Connection status visible in chrome/status line.

Connection lifecycle:
- Initial rollout: manual tunnel is supported first.
- Next rollout: TUI manages tunnel process for profile.
- Reconnect behavior:
  - auto-reconnect with bounded backoff
  - in-flight server work continues across disconnects
  - client surfaces degraded state until restored
- Auth mode (v1):
  - SSH key-based auth via user SSH config (`~/.ssh/config`) only.
  - No password or interactive credential prompts inside TUI.

Tests:
- Remote profile CRUD tests.
- Connect/disconnect state transition tests.
- Disconnect/reconnect tests while local projects remain fully usable.
- Routing tests, verify local ops still succeed when remote offline.

Exit criteria:
- User can initiate and manage remote connection from TUI settings.
- Local and remote projects are simultaneously usable and clearly distinguished.

## Phase 5, Remote Access (SSH Tunnel First)

Deliverables:
- Document supported remote workflow via SSH tunnel to Unix socket.
- Add connection mode options for CLI/TUI (local socket vs tunneled socket).
- Add operational docs for headless Grove deployment.

Implementation notes:
- No raw TCP listener initially.
- Keep auth model simple, rely on SSH + socket permissions.

Tests:
- End-to-end remote smoke (tunneled command executes, response valid).
- Disconnect/reconnect resilience tests.

Exit criteria:
- Practical remote control of Grove from another machine with same command protocol.

## Remote Connection UX (Target End State)

User flow:
1. Server one-time setup:
   - `groved` installed/running (ideally `systemd --user`)
   - daemon socket path known
2. Laptop:
   - create/select remote profile in TUI settings
   - press `connect` (or manual tunnel in early phase)
   - open remote project from project list
3. Run normal lifecycle operations from same TUI.

Operational guarantees:
- If laptop disconnects, server-side tmux + agent processes continue.
- Reconnecting restores control plane visibility.
- Local projects remain fully available regardless of remote state.

## Error Code Mapping Table (Phase 0 Contract)

Mapping precedence (first match wins):
1. Usage/argument validation (`INVALID_ARGUMENT`, `WORKSPACE_INVALID_NAME`)
2. Target resolution (`REPO_NOT_FOUND`, `WORKSPACE_NOT_FOUND`, `WORKSPACE_ALREADY_EXISTS`)
3. Runtime/domain conflicts (`CONFLICT`)
4. Adapter/process failures (`TMUX_COMMAND_FAILED`, `GIT_COMMAND_FAILED`, `IO_ERROR`)
5. Fallback (`INTERNAL`)

Local lifecycle mapping:

| `error.code` | Trigger (deterministic) | Current source signal examples | `fix` guidance template |
|---|---|---|---|
| `INVALID_ARGUMENT` | CLI/schema validation fails before execution | selector mismatch, missing selector, `workspace name is required`, `workspace branch is required`, `base branch is required`, `existing branch is required`, `workspace branch matches base branch` | Correct flags/selector and retry, run `grove` for command schema |
| `WORKSPACE_INVALID_NAME` | Workspace slug fails name validator | `workspace name must be [A-Za-z0-9_-]` | Use `[A-Za-z0-9_-]` only |
| `REPO_NOT_FOUND` | Repo target cannot be resolved/used | invalid `--repo`, `workspace project root unavailable`, git root resolution failure (`rev-parse --show-toplevel`) | Pass valid repo root or run from repo directory |
| `WORKSPACE_NOT_FOUND` | Target workspace cannot be resolved on disk/inventory | selector lookup miss, `workspace path does not exist on disk` | Refresh list, pick existing workspace, or recreate |
| `WORKSPACE_ALREADY_EXISTS` | Create target conflicts with existing worktree/path/branch identity | git worktree add errors indicating already exists/checked out | Choose different name or attach existing branch |
| `CONFLICT` | Operation blocked by repository state conflicts | merge output contains `CONFLICT (content):`, `Automatic merge failed...`, `base worktree has uncommitted changes`, `workspace worktree has uncommitted changes` | Resolve git conflicts/dirty state, commit/stash, retry |
| `TMUX_COMMAND_FAILED` | tmux execution step fails | process error starts with `command failed: tmux ...`, missing session/server errors | Ensure tmux/server/session health, retry stop/start |
| `GIT_COMMAND_FAILED` | git execution step fails (non-conflict) | errors prefixed by `git ...`, `git command failed: ...`, `git worktree remove failed: ...`, `git pull failed: ...` | Inspect git error, fix repo state/refs, retry |
| `IO_ERROR` | Filesystem/home-dir/script I/O fails | `io error: ...`, `home directory unavailable`, launcher script write failures | Fix filesystem permissions/path/home availability |
| `INTERNAL` | Unclassified unexpected failure | any non-mapped error | Capture debug logs, report bug, retry if safe |

Idempotency notes:
- `agent start` on already-running workspace returns `ok: true` no-op.
- `agent stop` on not-running workspace returns `ok: true` no-op.
- No dedicated `AGENT_*` error codes in v1 taxonomy under idempotent semantics.

Remote mapping (Phase 4.5+):

| `error.code` | Trigger (deterministic) | `fix` guidance template |
|---|---|---|
| `REMOTE_PROFILE_NOT_FOUND` | Referenced remote profile name does not exist | Pick/create valid profile in Remotes settings |
| `REMOTE_UNAVAILABLE` | Profile exists but backend is disconnected/offline/timeouts | Reconnect profile or switch target |
| `REMOTE_CONNECT_FAILED` | Connect/test action fails to establish tunnel/session | Verify SSH host/user/socket/auth, then retry connect/test |
| `PROTOCOL_VERSION_MISMATCH` | Client/daemon protocol versions incompatible | Upgrade/downgrade client or server to compatible versions |

## Concurrency Model by Phase

- Phase 1-3: local single-writer expectation (no heavy lock manager).
- Phase 4+: daemon enforces write serialization per repo, multi-client safe.
- Mixed target mode: write serialization enforced per backend target, local and remote paths independent.

## Risks + Mitigations

- Risk: behavior drift between TUI and CLI.
  - Mitigation: single command service boundary, shared tests.
- Risk: verbose/unbounded CLI payloads.
  - Mitigation: context-safe truncation + explicit full output pointers.
- Risk: partial failures in git/tmux workflows.
  - Mitigation: compensation steps + warning channel + deterministic error codes.
- Risk: transport refactor churn during daemon phase.
  - Mitigation: freeze request/response schema in Phase 0, treat transport as adapter only.
- Risk: remote drop degrades all work.
  - Mitigation: mixed local/remote routing, strict backend isolation, local always available.
- Risk: unclear local vs remote context causes mistakes.
  - Mitigation: explicit target badges + connection-state indicators in project list and status bar.

## Implementation Order (concrete)

- [x] 1. Add Phase 0a module boundaries (`interface::cli`, `interface::tui`, `application::commands` stubs).
- [x] 2. Add envelope + error code mapping module.
- [x] 3. Add command service trait + in-process implementation.
- [ ] 4. Migrate existing lifecycle code paths to service internally.
- [ ] 5. Build CLI root + lifecycle subcommands.
- [ ] 6. Add CLI integration and golden JSON tests.
- [ ] 7. Rewire TUI lifecycle actions to service, add 2s inventory refresh.
- [ ] 8. Add daemon binary + socket adapter.
- [ ] 9. Add mixed local/remote project target model + indicator UI.
- [ ] 10. Add remote profiles + connect/disconnect/test flows in settings.
- [ ] 11. Add reconnect handling and backend isolation tests.
- [ ] 12. Add remote operational docs (SSH and systemd user service).
