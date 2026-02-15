# ADR-009: Multi Project Workspace Model

**Status:** Accepted  
**Date:** 2026-02-15

## Context

Original Grove scope centered on one repo root at a time. Users need one Grove
instance to manage workspaces across multiple repositories without running
multiple TUI processes.

## Decision

Adopt a first-class multi-project model in runtime config and bootstrap flow.

- Persist configured projects in `~/.config/grove/config.toml`.
- Bootstrap and reconcile workspace sets per project.
- Include project context in workspace/session identity paths.

## Rationale

- Matches real usage where active agent work spans repos.
- Keeps one control plane while preserving per-project git/worktree semantics.
- Reuses existing workspace model with minimal additive context.

## Consequences

- Session naming and discovery include project scope.
- Startup and refresh logic aggregate partial failures across projects.
- UI and commands must always preserve selected project context.

## Related Commits

- `de9cf82`
- `7c13c35`
