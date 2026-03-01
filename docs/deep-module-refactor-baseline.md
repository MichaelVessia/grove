# Deep Module Refactor Baseline

Date: 2026-03-01

## Target Architecture (Phase 0 Baseline)

```text
main/cli
  -> ui::tui::app (presentation orchestration only)
      -> application::services::{workspace_service, runtime_service, discovery_service}
          -> domain::*
          -> infrastructure::{git, tmux, config, process, event_log}
```

## Dependency Constraints

1. UI does not call low-level runtime/lifecycle helpers directly.
2. Application services expose coarse operations and own orchestration.
3. Infrastructure modules avoid mixing command execution, parsing, and domain construction in one file.
4. Facade pass-through modules should be removed or replaced with true orchestration modules.

## Hot-Path Snapshot

Measured with `wc -l`:

| File | LOC |
|---|---:|
| `src/application/agent_runtime/mod.rs` | 1828 |
| `src/application/workspace_lifecycle.rs` | 811 |
| `src/infrastructure/adapters.rs` | 518 |
| `src/ui/tui/mod.rs` | 270 |
| `src/ui/tui/model.rs` | 183 |
| `src/ui/tui/update/update_navigation_preview.rs` | 519 |

## Baseline Validation Runs

All phase-0 baseline commands passed on 2026-03-01:

1. `cargo test workspace_lifecycle` (29 passed)
2. `cargo test agent_runtime` (106 passed)
3. `cargo test ui::tui::tests::runtime_flow::flow_a` (96 passed)
4. `cargo test --test startup_reconciliation` (3 passed)
5. `make precommit` (`fmt`, `check`, `clippy -D warnings` all passed)
