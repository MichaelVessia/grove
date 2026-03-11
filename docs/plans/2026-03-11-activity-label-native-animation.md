# Native Activity Label Animation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Grove's custom activity-label animation timing with native ftui animation timing while keeping the existing animated text label behavior.

**Architecture:** Grove already uses native ftui text effects for activity labels, so this change should stay narrow. Replace the app-owned frame-to-seconds calculation with a native `AnimationClock`, tick that clock from the existing visual animation path, and keep the sidebar and preview render call sites intact.

**Tech Stack:** Rust, Grove TUI, FrankenTUI `ftui_extras::text_effects::AnimationClock`, targeted `cargo test`, `make precommit`

---

### Task 1: Lock the animation timing contract with tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add focused tests that assert:
- the native activity animation time starts at `0.0`
- `advance_visual_animation()` increases native animation time
- repeated visual ticks continue increasing time monotonically

Use deterministic stepping, not wall-clock sleeps.

**Step 2: Run test to verify it fails**

Run: `cargo test activity_animation`
Expected: FAIL because the old implementation does not expose native clock-backed timing.

**Step 3: Write minimal implementation**

Do not implement yet, only after confirming the failure.

**Step 4: Run test to verify it passes**

Run: `cargo test activity_animation`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/mod.rs
git commit -m "test: lock native activity animation timing"
```

### Task 2: Replace frame-count timing with `AnimationClock`

**Files:**
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/bootstrap/bootstrap_app.rs`
- Modify: `src/ui/tui/update/update_polling_state.rs`

**Step 1: Write the failing test**

Use the new timing tests from Task 1 as the regression gate.

**Step 2: Run test to verify it fails**

Run: `cargo test activity_animation`
Expected: FAIL until the native clock is wired in.

**Step 3: Write minimal implementation**

- add `AnimationClock` to app state
- initialize it in bootstrap
- update `advance_visual_animation()` to tick the clock with the fast animation interval converted to seconds
- remove `fast_animation_frame` if nothing else uses it

**Step 4: Run test to verify it passes**

Run: `cargo test activity_animation`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/model.rs src/ui/tui/bootstrap/bootstrap_app.rs src/ui/tui/update/update_polling_state.rs src/ui/tui/mod.rs
git commit -m "refactor: use ftui animation clock for activity labels"
```

### Task 3: Switch label rendering to native clock time

**Files:**
- Modify: `src/ui/tui/view/view_chrome_shared.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add or tighten focused render assertions so the sidebar and preview still render activity labels after the timing migration. Reuse existing active-workspace render fixtures where possible.

**Step 2: Run test to verify it fails**

Run: `cargo test active_workspace_`
Expected: FAIL if render plumbing depends on the removed frame counter path.

**Step 3: Write minimal implementation**

- remove `activity_effect_time()`
- feed `self.polling.activity_animation.time()` into `StyledText::time(...)`
- keep `TextEffect::AnimatedGradient` and the existing gradient helper unless cleanup shows one can be deleted safely

**Step 4: Run test to verify it passes**

Run:
- `cargo test activity_animation`
- `cargo test active_workspace_`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_chrome_shared.rs src/ui/tui/mod.rs
git commit -m "refactor: render activity labels with native animation time"
```

### Task 4: Remove dead custom timing code and imports

**Files:**
- Modify: `src/ui/tui/shared.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/view/view_chrome_shared.rs`
- Modify: any compiler-reported imports affected by the cleanup

**Step 1: Write the failing test**

Use compilation and the focused tests as the regression gate.

**Step 2: Run test to verify it fails**

Run: `cargo test activity_animation`
Expected: compile failure or test failure until dead code is removed cleanly.

**Step 3: Write minimal implementation**

Delete any now-unused helpers, fields, and imports left behind by the migration. Keep `FAST_ANIMATION_INTERVAL_MS` only if it remains the canonical tick interval for visual work.

**Step 4: Run test to verify it passes**

Run:
- `cargo test activity_animation`
- `cargo test active_workspace_`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/shared.rs src/ui/tui/model.rs src/ui/tui/view/view_chrome_shared.rs src/ui/tui/update/update_polling_state.rs src/ui/tui/bootstrap/bootstrap_app.rs src/ui/tui/mod.rs
git commit -m "refactor: remove custom activity animation timing"
```

### Task 5: Final verification

**Files:**
- Review touched files only

**Step 1: Run focused tests**

Run:
- `cargo test activity_animation`
- `cargo test active_workspace_`
- `cargo test schedule_next_tick`

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`
Expected: PASS

**Step 3: Update issue**

Comment on `#54` with the native animation-clock migration summary if needed.

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: replace custom activity animation timing"
```
