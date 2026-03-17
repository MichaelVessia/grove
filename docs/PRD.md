# Grove PRD

Grove is a minimal task and worktree manager for AI coding agents. It is a
Rust TUI built on FrankenTUI and backed by tmux.

## Product Summary

Grove manages parallel coding tasks. A task is the top-level unit. Each task
has a task root, one or more repository worktrees, and zero or more running
agent sessions. The TUI is the control plane for creating tasks, launching
agents, monitoring output, interacting with sessions, and cleaning up when the
task is done.

This document is the product and architecture source of truth. Historical
workspace-era designs are obsolete.

## Goals

1. Make `task` the only first-class runtime model.
2. Create and manage multi-repository tasks from a single TUI flow.
3. Launch, monitor, and control Claude Code, Codex, and OpenCode via tmux.
4. Support inline interactive control, live preview, mouse input, and replay.
5. Keep the implementation small, explicit, and maintainable.

## Non-Goals

- Backwards compatibility with legacy workspace-era Grove state.
- Runtime discovery from `~/.grove/workspaces`.
- Marker-file based workspace discovery.
- Plugin systems, kanban views, issue tracker integration.
- Shell-first workspace management outside the task model.
- PR creation, review automation, or merge orchestration beyond local
  merge/update helpers.

## Core Model

### Task

A task is the top-level unit in Grove.

- Each task lives at `~/.grove/tasks/<task-slug>/`.
- Task metadata lives at `~/.grove/tasks/<task-slug>/.grove/task.toml`.
- A task has:
  - `name`
  - `slug`
  - `root_path`
  - `branch`
  - `worktrees`

### Worktree

A worktree is one repository checkout inside a task.

- Each worktree belongs to exactly one task.
- Each worktree records:
  - `repository_name`
  - `repository_path`
  - `path`
  - `branch`
  - `base_branch`
  - `agent`
  - `status`
  - `is_orphaned`
  - `supported_agent`
  - pull request metadata

### UI Workspace

The TUI still renders `Workspace` rows, but that is a view model derived from
`Task + Worktree`, not a separate persistence model.

- `AppState` stores canonical `tasks`.
- `workspaces` is a flattened projection of all task worktrees.
- Selection is tracked by:
  - flat `selected_index`
  - `selected_task_index`
  - `selected_worktree_index`
- These selection fields must stay synchronized.

## Filesystem Layout

Canonical task layout:

```text
~/.grove/tasks/
  <task-slug>/
    .grove/
      task.toml
    <repository-a>/
    <repository-b>/
```

There is no runtime dependency on `~/.grove/workspaces`.

## Source Of Truth

The task manifest is the source of truth for Grove-managed state.

- Startup loads tasks from `~/.grove/tasks/*/.grove/task.toml`.
- Manual refresh reloads task manifests from the task root.
- Replay bootstrap reconstructs state from task-shaped data.
- Session cleanup plans from discovered tasks, not legacy workspace discovery.

If a task manifest is missing or invalid, that is a task discovery problem, not
a signal to fall back to a different model.

## Session Model

Grove uses task-native tmux session families.

- Task-root parent agent session:
  - `grove-task-<task-slug>`
- Repository worktree agent session:
  - `grove-wt-<task-slug>-<repository>`
- Auxiliary sessions derive from the worktree session name when needed:
  - `...-git`
  - `...-shell`

Implications:

- Home-tab launch, stop, and preview target the task-root session.
- Repository preview and interactive mode target the selected worktree session.
- Session cleanup operates on Grove-managed task/worktree sessions.

## Startup And Refresh

On startup Grove:

1. Resolves the tasks root.
2. Loads task manifests.
3. Reconciles discovered tasks with running tmux sessions.
4. Builds `AppState` from tasks only.
5. Restores persisted task order, sidebar state, and attention acks.

Manual refresh repeats task discovery and rebuilds task-derived UI state.

There is no workspace-era bootstrap path in the runtime model.

## Task Creation

The primary user flow is creating a task.

Inputs:

- Task name or GitHub pull request URL
- One or more configured repositories
- Agent type
- Branch source

Behavior:

1. Create the task root under `~/.grove/tasks/<task-slug>/`.
2. Materialize one worktree per selected repository.
3. Use the task branch or PR-derived branch as the worktree branch.
4. Apply repository defaults from config.
5. Run setup hooks for newly created worktrees.
6. Persist the task manifest.
7. Refresh the TUI and focus the new task.

## Agent Lifecycle

Grove supports three agents:

- Claude
- Codex
- OpenCode

Per worktree, Grove can:

- start an agent
- stop an agent
- restart an orphaned or finished agent
- preview captured output
- enter interactive mode

Status is derived from runtime state and manifest/session reconciliation. The
main checkout of a repository is represented as `WorkspaceStatus::Main` in the
UI projection.

## Preview And Interactive Mode

The TUI supports:

- live output preview with ANSI rendering
- inline interactive mode against tmux panes
- mouse hit testing
- resizable sidebar
- task/worktree-aware selection and preview routing

Interactive control is inside the TUI. Direct tmux attach is not part of the
core product model.

## Merge, Update, Delete

Grove supports local task/worktree lifecycle operations needed to finish work:

- merge a selected worktree back to base
- update a selected worktree from base
- delete a selected worktree/task state after confirmation

These flows are task-aware and must carry `task_slug` through runtime
operations so session cleanup and lifecycle commands target the correct
resources.

## Discovery And Cleanup

Task discovery is task-manifest based.

- Discover tasks by enumerating `~/.grove/tasks`.
- Parse `task.toml`.
- Reconcile running sessions against expected task/worktree session names.
- Mark orphaned or stopped worktrees accordingly.

Cleanup is also task-based.

- Use discovered tasks to determine which Grove-managed sessions are expected.
- Kill orphaned or stale Grove-managed sessions.
- Skip attached sessions unless explicitly told otherwise.

Diagnosis is task-based as well.

- `grove doctor` audits task manifests, configured repositories, filesystem
  paths, and Grove-managed tmux sessions.
- Doctor prints findings and a repair plan, but does not mutate state in v1.
- Doctor is the primary product surface for understanding runtime drift before a
  human or coding agent performs repairs.

## Config

Grove uses two config files:

- global settings, typically `~/.config/grove/config.toml`
- project state, typically `~/.config/grove/projects.toml`

Current config responsibilities:

- sidebar width
- theme
- launch permission behavior
- configured repositories (`projects` in the current config schema)
- per-repository defaults:
  - `base_branch`
  - `workspace_init_command`
  - `agent_env`
- persisted task ordering
- attention acknowledgements

The config schema may still use some `project` naming, but the runtime model is
task-first.

## Replay And Debugging

Replay is a first-class debugging tool for Grove runtime issues.

- Debug recordings capture task-native state transitions.
- Replays reconstruct app state from task bootstrap data.
- Replay-generated tests should assert task/worktree behavior, not legacy
  workspace behavior.

## Design Principles

- No backwards compatibility in runtime architecture.
- One canonical model, `task -> worktrees`.
- Derived UI projections are allowed, duplicate persistence models are not.
- Prefer explicit state transitions over compatibility shims.
- Persist durable state in task manifests, not scattered marker files.
- Keep tmux naming predictable and deterministic.

## Acceptance Criteria

Grove matches this PRD when all of the following are true:

1. Startup and refresh build runtime state from task manifests only.
2. The TUI operates on task-derived worktree rows, with synchronized selection
   invariants.
3. Task-root and worktree tmux session families are canonical.
4. Create, launch, preview, merge, update, delete, replay, and cleanup all
   operate on the task model.
5. No product behavior depends on legacy workspace-era discovery or storage.
