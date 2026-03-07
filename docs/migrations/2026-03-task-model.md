# Task Model Migration

Grove now treats a `task` as the top-level unit.

- `workspace` -> `task`
- `project` -> `repository`
- `worktree` now means one repository checkout inside a task
- each task lives under `~/.grove/tasks/<task-slug>/`
- task metadata lives in `~/.grove/tasks/<task-slug>/.grove/task.toml`
- parent agent sessions use `grove-task-<task-slug>`
- repository worktree sessions use `grove-wt-<task-slug>-<repository>`

Behavior changes:

- creating from the TUI creates one task, with one or more repository worktrees
- refresh reloads task manifests when the task root exists
- Home-tab launch, stop, and preview operate on the task-root parent agent
- session cleanup now targets both task-root and worktree sessions

Compatibility:

- legacy single-workspace repos are treated as single-worktree tasks during fallback discovery
- tmux session naming remains stable for legacy workspace-scoped sessions only where fallback discovery is still in use
