# ADR-011: Git Preview Delegation to Lazygit

**Status:** Accepted  
**Date:** 2026-02-15

## Context

Grove needs a useful git-focused preview mode without growing a large custom git
UI surface inside the main TUI runtime.

## Decision

Use a dedicated Git preview tab that delegates git interaction to embedded
`lazygit`, instead of implementing a bespoke in-app git interface.

## Rationale

- Reuses a mature git TUI for rich git workflows.
- Keeps Grove focused on workspace and agent orchestration.
- Limits maintenance burden of custom git UX/state handling.

## Consequences

- `lazygit` becomes a runtime dependency for full Git-tab functionality.
- Key handling is scoped by preview tab to avoid conflicts with agent preview.
- Advanced git UX changes are mostly inherited from upstream lazygit behavior.

## Related Commits

- `9a0bd1e`
- `5684f65`
