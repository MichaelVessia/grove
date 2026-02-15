# ADR-010: Observability Contract, Event Log and Debug Record

**Status:** Accepted  
**Date:** 2026-02-13

## Context

Grove debugging requires reconstructing timing-sensitive runtime behavior
(polling, interactive input latency, lifecycle races) that is hard to diagnose
from user reports alone.

## Decision

Use structured NDJSON event logging as a first-class runtime debug contract.

- Provide file-backed event logging for runtime events.
- Support continuous debug recording mode for full-session traces.
- Keep event schema stable enough for scripted analysis.

## Rationale

- Makes latency and race regressions reproducible.
- Enables deterministic postmortem analysis in CI/local scripts.
- Reduces reliance on ad-hoc print debugging in terminal flows.

## Consequences

- New runtime features should emit targeted structured events.
- Debug workflows depend on log schema consistency.
- Logging overhead is accepted as a tradeoff for diagnosability.

## Related Commits

- `5b10578`
- `5c5caf6`
- `bd76632`
