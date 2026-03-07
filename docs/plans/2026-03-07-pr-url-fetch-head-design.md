# PR URL Fetch Head Design

**Date:** 2026-03-07

## Summary

`From GitHub PR` should not only name the task from the PR number, it should also create the task worktree from the PR head commit. Grove should fetch the GitHub PR ref directly with git, then create the worktree branch from `FETCH_HEAD`.

## Problem

The current PR create flow is only partially PR-aware:

- it parses and validates the GitHub PR URL
- it derives the task name as `pr-<number>`
- it still creates the task worktree from the repository base branch

That means the resulting task does not reflect the actual remote PR contents.

## Decision

Use direct git PR-ref fetching for PR mode.

- Keep PR mode single-project-only
- When PR mode is selected, fetch `origin pull/<number>/head`
- Create the local task worktree branch from `FETCH_HEAD`
- Continue to use the task name `pr-<number>`

## Why Direct Git Fetch

This fits Grove better than a `gh`-driven approach:

- Grove already has the same pattern in the older workspace lifecycle
- no new dependency on `gh`
- no auth flow beyond what git remote access already requires
- we do not need the source branch name, only the PR head commit

## Chosen Behavior

For `From GitHub PR`:

1. Parse owner, repo, and PR number from the URL
2. Validate the selected project's `origin` matches the PR repository
3. Fetch `origin pull/<number>/head`
4. Create local worktree branch `pr-<number>` from `FETCH_HEAD`
5. Persist the task manifest normally

Manual mode remains unchanged.

## Architecture

Task lifecycle needs a PR-aware branch creation path, similar to the existing workspace lifecycle:

- extend `CreateTaskRequest` with optional branch source metadata for PR mode
- in task creation, switch between:
  - manual mode: `git worktree add -b <task> <path> <base-branch>`
  - PR mode: `git fetch origin pull/<n>/head`, then `git worktree add -b <task> <path> FETCH_HEAD`

The TUI create dialog already parses the PR URL and validates repo match, so it can pass the PR number into the lifecycle request instead of encoding PR behavior indirectly in the task name.

## Alternatives Considered

### 1. Resolve via `gh`

Rejected. It adds a dependency and auth requirement without giving Grove something it actually needs for this flow.

### 2. Keep current base-branch behavior

Rejected. It makes PR mode misleading because the task does not check out PR content.

### 3. Hybrid git-first, `gh` fallback

Rejected for now. More moving parts than needed. Direct git fetch is sufficient.

## Error Handling

- If `git fetch origin pull/<n>/head` fails, task creation should fail with a clear git error
- Repo mismatch validation remains before fetch
- Manual mode should not be affected by PR-mode failures

## Testing

- task lifecycle tests should verify PR mode runs `git fetch origin pull/<n>/head`
- task lifecycle tests should verify PR mode creates the worktree from `FETCH_HEAD`
- TUI integration should verify PR mode still creates a single-repo task named `pr-<n>`
- manual multi-repo task creation should continue to pass unchanged
