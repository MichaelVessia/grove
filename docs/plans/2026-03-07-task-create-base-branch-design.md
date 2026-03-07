# Task Create Base Branch Design

## Goal

Make task creation use each selected project's default base branch implicitly, with git-based auto-detection when no project default is configured.

## Decision

Remove manual base-branch entry from the New Task dialog.

For each selected project:

- use `project.defaults.base_branch` when non-empty
- otherwise auto-detect from the repository itself

Task creation must no longer assume one shared base branch across all selected repositories. Each created worktree gets its own resolved base branch.

## Why

The current dialog exposes a single `BaseBranch` field even though tasks can include multiple repositories. That forces one branch choice across repositories and breaks when one repo uses `main` while another uses `master`.

Project defaults already exist and are the right source of truth for stable per-repo behavior. Auto-detection covers unconfigured repositories without adding UI friction.

## Resolution Rules

Per selected project, resolve base branch in this order:

1. `project.defaults.base_branch`, if non-empty
2. repository default branch from git metadata
3. current branch, if not detached
4. `main`, if it exists locally
5. `master`, if it exists locally

If none resolve, task creation should fail with a clear per-project error.

## UI Changes

- remove `BaseBranch` from the New Task dialog
- remove branch dropdown behavior tied to that field
- keep project defaults editing in the Projects dialog
- update help text so task creation points users to project defaults instead of inline branch entry

## Application Changes

- task creation request should no longer carry one shared `base_branch`
- lifecycle code should resolve base branch per repository during creation
- each worktree should persist its own `.grove/base` marker using that resolved value

## Tests

Add regression coverage for:

- mixed selected repositories with different configured defaults
- repositories with no configured default that auto-detect `main`
- repositories with no configured default that auto-detect `master`
- New Task dialog rendering and navigation without a base-branch field
