# Workspace List Actions Design

**Goal:** Make task structure editable from the workspace list by adding worktrees to existing tasks and deleting either a single worktree or the whole task.

## Decisions

- `a`, `d`, and `D` are workspace-list-only actions.
- Preview keeps runtime/session actions. `a` in preview still launches an agent.
- `a` in the workspace list opens the existing create dialog in "add worktree to selected task" mode.
- Added worktrees always attach to the selected task. This flow never creates a base task and never creates a new task.
- `d` deletes only the selected worktree.
- If `d` targets the last remaining worktree, the confirmation dialog explicitly warns that the task will also be deleted.
- `D` deletes the selected task and all of its worktrees.
- Keybind help and command palette must describe these pane-scoped meanings.

## Architecture

- Extend the create dialog with a target mode:
  - create new task
  - add worktree to selected task
- Add task lifecycle support for appending one worktree to an existing task and rewriting the task manifest.
- Split delete dialog state between:
  - delete selected worktree
  - delete whole task
- Reuse existing delete execution paths:
  - worktree deletion via workspace lifecycle delete
  - full task deletion via task lifecycle delete

## UX Notes

- Delete dialog copy must distinguish:
  - "Delete worktree"
  - "Delete final worktree and task"
  - "Delete task"
- Last-worktree deletion should keep the current guardrails for base/main worktrees where applicable, but the dialog must still be truthful about deleting the task.

## Tests

- `a` in workspace list opens add-worktree flow for the selected task.
- `a` in preview still opens launch dialog.
- `d` in workspace list opens single-worktree delete.
- `d` on the last worktree warns that the task will be deleted.
- `D` only works in workspace list.
- Add-worktree completion refreshes and reselects the new worktree inside the existing task.
- Help and command palette reflect pane-scoped actions.
