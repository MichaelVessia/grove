# Native Activity Label Animation Design

**Issue:** `#54`

**Goal:** Replace Grove's custom activity-label animation timing with native ftui animation support while preserving the existing animated text treatment where practical.

## Current State

Grove already renders activity labels with native ftui text effects:

- `StyledText`
- `TextEffect::AnimatedGradient`
- `ColorGradient`

The custom part is the timing source in `src/ui/tui/view/view_chrome_shared.rs`, where `activity_effect_time()` derives seconds from `polling.fast_animation_frame` and `FAST_ANIMATION_INTERVAL_MS`.

## Constraints

- Prefer native ftui primitives over Grove-owned animation math.
- Keep the existing render call sites in the sidebar and preview unless simplification is clearly better.
- Do not introduce custom compatibility layers.
- Keep animation deterministic enough for targeted tests.
- Avoid broad UI changes, this issue is about native animation plumbing, not redesign.

## Investigated Native ftui Options

### `ftui_extras::text_effects::AnimationClock`

This is the best fit. It is purpose-built for time-based text effects and integrates directly with `StyledText::time(...)`.

It gives Grove:

- native time accumulation
- clear ownership of animation timing
- deterministic `tick_delta(...)` support for tests

### `ftui_core::animation::{Animation, Spring, Timeline, AnimationGroup}`

These primitives exist, but they are lower-level than Grove needs here. They are useful when Grove owns animation state machines directly. For this issue, they add integration work without improving the text-effect path.

### `ftui_widgets::Spinner`

A spinner is a valid native fallback, but it changes the UI semantics from animated text to icon-plus-label. Since ftui already provides native animated text, spinner-only would be an unnecessary visual regression.

## Chosen Approach

Keep the current animated text label rendering, but replace Grove's custom frame-based time calculation with a native `AnimationClock`.

That means:

- add a clock field to app state
- initialize it during bootstrap
- advance it from the existing fast visual animation tick path
- pass `clock.time()` into `StyledText::time(...)`
- delete `activity_effect_time()` and the now-unused frame counter if nothing else needs it

## Why This Approach

- It satisfies the issue goal without changing the visible treatment much.
- It uses native ftui for both the effect and the timing source.
- It keeps the current sidebar and preview label rendering structure intact.
- It avoids overengineering with lower-level animation primitives.

## Implementation Notes

- `src/ui/tui/model.rs`
  - replace `fast_animation_frame` with a native animation clock in `PollingState` or another small app-owned state struct
- `src/ui/tui/bootstrap/bootstrap_app.rs`
  - initialize the native clock
- `src/ui/tui/update/update_polling_state.rs`
  - tick the native clock from `advance_visual_animation()`
- `src/ui/tui/view/view_chrome_shared.rs`
  - remove custom time calculation and read time from the native clock
- `src/ui/tui/mod.rs`
  - add targeted regression tests for native clock advancement and unchanged render behavior

## Testing

Add or update focused tests to prove:

- visual animation advancement now changes native animation time
- activity labels still render through the existing sidebar and preview paths
- the fast visual tick scheduling behavior does not regress

Use focused `cargo test` commands plus required `make precommit` before handoff.

## Non-Goals

- redesigning activity labels
- migrating the label to spinner-only treatment
- adopting `Spring`, `Timeline`, or `AnimationGroup` for this specific effect
- changing poll cadence or status semantics
