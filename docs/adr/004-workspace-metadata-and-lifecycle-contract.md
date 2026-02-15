# ADR-004: Workspace Metadata and Lifecycle Contract

**Status:** Accepted  
**Date:** 2026-02-13

## Context

Grove needs restart-safe workspace state without introducing a central database
or complex manifest lifecycle. Workspace state must survive process restarts and
remain discoverable from git worktree state.

## Decision

Use git worktrees as the source of workspace existence, plus per-worktree marker
files for Grove metadata.

- Discover workspaces from `git worktree list --porcelain`.
- Persist Grove metadata in each worktree root:
  - `.grove-agent`
  - `.grove-base`
- Keep lifecycle actions (create/delete/recover) driven by this contract.

## Rationale

- Keeps source of truth local to each worktree, no separate state store.
- Survives crashes and app restarts naturally.
- Mirrors proven sidecar behavior while keeping Grove minimal.

## Consequences

- Startup reconciliation can rebuild runtime state deterministically.
- Corrupt or missing marker files become explicit, detectable lifecycle errors.
- Adding workspace metadata requires adding/changing marker files.

## Related Commits

- `787b4ed`
- `fc42e00`
- `682dbbb`
