# Investigation: Full-Screen Diff Every Frame

## Status

Identified, not yet fixed. Budget workaround committed in `3c928ae`.

## Problem

Grove renders take 185-365ms per frame. The overhead is in frame
infrastructure (buffer allocation, diff computation, terminal write), not
in widget content. Stripping content via the degradation system does not
reduce draw time.

The app feels sluggish as a result.

## Root Cause

The diff strategy config in `src/ui/tui/runner.rs:60-66` forces a
full-screen redraw every frame:

```rust
DiffStrategyConfig {
    c_scan: 1_000_000.0,
    uncertainty_guard_variance: 1_000_000.0,
    hysteresis_ratio: 0.0,
    ..DiffStrategyConfig::default()
}
```

- `c_scan: 1_000_000.0` makes the scanner always favor a full scan over
  targeted region detection. The cost model never selects partial updates.
- `uncertainty_guard_variance: 1_000_000.0` inflates the confidence
  interval so the variance tracker can never converge, keeping the
  strategy in permanent "uncertain" mode.
- `hysteresis_ratio: 0.0` disables smoothing between strategy switches,
  so the system flips aggressively rather than settling.

Together these values effectively disable the adaptive diff optimizer and
force full-screen terminal writes on every frame.

## Impact

- 185-365ms per frame (vs. ~16ms target for 60fps)
- Before the budget workaround, the budget controller escalated
  degradation through `Full > SimpleBorders > NoStyling > EssentialOnly >
  Skeleton > SkipFrame`, producing blank frames that never recovered.
  See debug record at `.grove/debug-record-1771684367289-81523.jsonl`.
- With the workaround (`strict(1s)` budget), degradation never triggers
  but the app is visibly sluggish.

## Workaround (committed)

Raised the frame budget from `strict(250ms)` to `strict(1s)` so the
degradation controller never activates. This prevents blank screens but
does not address the underlying render cost.

## Suggested Fix

Tune the `DiffStrategyConfig` to allow the adaptive diff optimizer to
work properly. Options:

1. **Use defaults**: Remove the custom `DiffStrategyConfig` entirely and
   let FrankenTUI's defaults handle it. The defaults are tuned for
   typical TUI workloads.

2. **Tune incrementally**: Lower `c_scan` to a reasonable value (the
   default is likely much smaller), set `uncertainty_guard_variance` to
   something that allows convergence, and add a nonzero
   `hysteresis_ratio` to prevent strategy flapping.

3. **Profile first**: Run with `--debug-record`, capture the
   `frame_rendered` events, and look at `draw_ms` vs `view_ms` to
   understand where time is spent. The debug record already captures
   these fields.

## Key Files

| File | Relevance |
|---|---|
| `src/ui/tui/runner.rs:57-75` | `program_config()` with diff strategy and budget |
| `.reference/frankentui/crates/ftui-render/src/budget.rs` | `FrameBudgetConfig`, `DegradationLevel`, `BudgetController` |
| `.reference/frankentui/crates/ftui-render/src/diff.rs` | Diff strategy, `DiffStrategyConfig`, cost model |
| `.reference/frankentui/crates/ftui-runtime/src/program.rs` | `Program::run()`, render loop, budget integration |
| `.reference/frankentui/crates/ftui-runtime/src/cost_model.rs` | Cost model that uses `c_scan` |

## Secondary Issue: Workspace Refresh Auto-Walk

The debug log also showed the workspace selection cursor rapidly cycling
through all 13 workspaces during refresh completion. Each selection
change triggers stale preview captures that get dropped. This creates
wasted work during already-stressed rendering. See
`refresh_workspaces_completed` events in the debug record.

Relevant code: `src/ui/tui/update_lifecycle_workspace_refresh.rs:117-147`
(`apply_refresh_workspaces_completion`).

