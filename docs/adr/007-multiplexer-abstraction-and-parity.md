# ADR-007: Multiplexer Abstraction and Parity

**Status:** Accepted  
**Date:** 2026-02-14

## Context

Grove started on tmux, then added zellij support. Runtime behavior must stay
consistent across multiplexers for discovery, lifecycle, capture, and status.
Drift between backends would create user-visible behavioral bugs.

## Decision

Treat tmux and zellij as first-class backends behind shared runtime interfaces,
with an explicit parity requirement.

- Add config-level multiplexer selection.
- Keep shared app/runtime flow, backend-specific command plans.
- Enforce parity for lifecycle, capture, key forwarding, and status polling.

## Rationale

- Allows backend expansion without forking app behavior.
- Keeps UX expectations stable when switching multiplexers.
- Makes backend differences explicit and local to adapter/runtime boundaries.

## Consequences

- Any multiplexer-affecting runtime change must be checked on both backends.
- Tests include both tmux and zellij paths where behavior can diverge.
- Some backend-specific handling remains, but behind shared contracts.

## Related Commits

- `72cafec`
- `4e65391`
- `c60cbed`
