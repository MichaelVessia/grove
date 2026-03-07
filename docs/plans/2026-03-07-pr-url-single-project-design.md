# PR URL Single-Project Create Design

**Date:** 2026-03-07

## Summary

`From PR URL` should create a task for exactly one project. A GitHub PR is repo-scoped, so the create flow should reflect that directly instead of exposing multi-project task selection in this mode.

## Problem

The current create dialog mixes two different concepts:

- `Manual` mode creates a task from one or more selected projects
- `From PR URL` mode still renders `Included` projects, even though the PR URL only identifies one repository

That makes the PR flow ambiguous. The UI implies the PR can seed a multi-project task, but the lifecycle code validates the PR against a single selected project.

## Decision

Treat `From PR URL` as a single-project-only create mode.

- Keep `Manual` as the general multi-project task creation flow
- Keep `From PR URL` for "resume or import an existing PR branch into Grove"
- Require exactly one selected project in PR mode
- Remove multi-project affordances from PR mode

## Chosen UX

### Manual

- Task name field
- Project picker
- `Included` projects summary
- Create / cancel actions

### From GitHub PR

- Project field
- PR URL field
- Read-only derived name, `pr-<number>`
- No `Included` row
- No multi-select semantics

## Behavioral Rules

- Parse owner, repo, and PR number from the GitHub URL
- Validate that the selected project's `origin` matches the PR repo
- Use the PR number to derive the task name
- Create exactly one worktree for the selected project
- Check out the appropriate PR branch or ref for that repository

## Alternatives Considered

### 1. Keep PR mode multi-project

Rejected. Flexible, but semantically muddy. The PR URL only defines one repository, so extra included repos would be arbitrary hitchhikers.

### 2. Remove PR mode, link PR after manual creation

Rejected. Cleaner model, worse workflow. The user wants to bootstrap directly from an existing PR.

## Consequences

### Good

- Matches domain reality, PRs are repo-scoped
- Removes misleading UI from the PR tab
- Keeps manual mode as the only place where multi-project selection exists

### Trade-off

- Users cannot create a multi-project task from a PR in one step
- If that becomes necessary later, it should be modeled as a follow-up "attach repo to task" action, not overloaded into PR bootstrap

## Testing

- Dialog tests should verify PR mode does not show or rely on multi-project selection
- Create flow tests should verify PR mode creates one worktree only
- Validation tests should verify repo mismatch errors remain intact
