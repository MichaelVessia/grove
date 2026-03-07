# Base Task Remove Design

## Problem

Tasks that point at a repository's primary checkout can currently linger in the
task list with no safe removal path. Users want to remove those task entries
without deleting the underlying checkout.

## Goals

- Allow base tasks to be removed from Grove's task list.
- Keep the underlying repository checkout intact.
- Prevent destructive options that do not apply to base tasks.
- Keep non-base task deletion behavior unchanged.

## Non-Goals

- Changing how base worktrees are discovered.
- Deleting the primary checkout from disk.
- Deleting the primary checkout's branch.

## Decision

Treat base-task removal as a non-destructive "remove from list" operation.

A task is considered a base task when any task worktree points at its primary
repository checkout. In practice this means either:

- `worktree.path == worktree.repository_path`, or
- the persisted worktree status is `Main`

## UX

- Base tasks remain selectable in the task list.
- The existing delete action opens a dialog for base tasks, but the dialog copy
  changes from destructive delete wording to non-destructive remove wording.
- The dialog disables local branch cleanup for base tasks.
- The dialog disables worktree/root deletion for base tasks.
- Session cleanup may remain available so the user can stop Grove-managed tmux
  sessions while removing the task entry.

## Lifecycle Behavior

- Non-base tasks keep the current delete flow.
- Base tasks skip `git worktree remove`.
- Base tasks skip branch deletion.
- Base tasks skip removing `task.root_path` when it is the primary checkout.
- Base tasks remove only the manifest entry under `~/.grove/tasks/<slug>`.

## Implementation Notes

- Detect base-task semantics at the `Task` level, not from the flattened UI
  workspace row.
- Update command gating so task removal is allowed for base tasks.
- Add an explicit non-destructive removal path in task lifecycle code rather
  than overloading the existing destructive path implicitly.

## Testing

- Regression test: base tasks can open the remove dialog.
- Regression test: base-task remove deletes only the manifest entry.
- Regression test: base-task remove does not invoke `git worktree remove`.
- Regression test: non-base task delete remains destructive.
