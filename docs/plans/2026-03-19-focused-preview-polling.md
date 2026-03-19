# Focused Preview Polling Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the selected preview feel materially more responsive by tightening only foreground preview polling cadence.

**Architecture:** Keep Grove's existing earliest-deadline scheduler and `tmux capture-pane` transport. Change the selected-preview cadence rules so a focused selected workspace polls much faster while the user is watching it, then decays back toward current intervals when interactive activity cools off. Leave background status polling unchanged.

**Tech Stack:** Rust, Grove TUI runtime, tmux `capture-pane`, cargo test, make precommit

---

## Chunk 1: Poll policy and scheduler coverage

### Task 1: Add focused-preview cadence regression tests

**Files:**
- Modify: `src/application/agent_runtime/polling.rs`
- Test: `src/application/agent_runtime/polling.rs`

- [ ] **Step 1: Write the failing tests**

Add unit tests covering the intended foreground policy:

- selected + preview focused + not interactive + waiting/idle should no longer return `500 ms`
- selected + preview focused + active/thinking should poll at the new fast cadence
- non-selected workspaces must remain at `10 s`
- interactive selected cadence must remain bounded by the existing interactive path

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test poll_intervals_follow_preview_and_interactive_rules -- --nocapture
```

Expected: FAIL because the existing preview-focused path still returns `500 ms`.

- [ ] **Step 3: Write minimal implementation**

Update `poll_interval(...)` in `src/application/agent_runtime/polling.rs` so:

- preview-focused selected workspaces use a faster cadence than today
- active output and preview focus do not fight each other
- the interactive branch still wins when present

Prefer one explicit constant for the new focused cadence instead of magic numbers.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test poll_intervals_follow_preview_and_interactive_rules -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/application/agent_runtime/polling.rs
git commit -m "feat: tighten focused preview poll cadence"
```

### Task 2: Prove the scheduler prefers the tighter foreground deadline

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing test**

Add a focused scheduler test that:

- creates an app with the preview pane focused
- selects an active workspace
- ensures no interactive debounce deadline is present
- calls `schedule_next_tick()`
- asserts `next_tick_trigger == Some("poll")`
- asserts `next_tick_interval_ms` is at or below the new focused cadence

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test focused_preview_schedules_fast_poll_deadline -- --nocapture
```

Expected: FAIL because the current adaptive poll deadline is still too slow.

- [ ] **Step 3: Write minimal implementation**

Adjust only the selected-preview cadence inputs. Do not add a special-case scheduler path if the existing scheduler can already honor the faster poll interval.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test focused_preview_schedules_fast_poll_deadline -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/mod.rs src/application/agent_runtime/polling.rs
git commit -m "test: cover fast focused preview scheduling"
```

## Chunk 2: Runtime safeguards and user-visible verification

### Task 3: Preserve background polling behavior explicitly

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing test**

Add a regression test proving a non-selected active workspace still reports the existing background cadence in the performance rows.

Assert that:

- selected focused workspace shows the new fast cadence
- background active workspace still shows `10000 ms`

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test session_performance_rows_preserve_background_cadence_when_selected_preview_is_fast -- --nocapture
```

Expected: FAIL until the foreground/background distinction is updated consistently.

- [ ] **Step 3: Write minimal implementation**

Update any performance-row or cadence-label expectations so the dialog reports the new selected-preview cadence without altering background rows.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test session_performance_rows_preserve_background_cadence_when_selected_preview_is_fast -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/mod.rs src/ui/tui/performance.rs src/application/agent_runtime/polling.rs
git commit -m "test: preserve background cadence reporting"
```

### Task 4: Record the new tuning and validate locally

**Files:**
- Modify: `docs/adr/003-interactive-polling-strategy.md`
- Modify: `docs/plans/2026-03-19-focused-preview-polling.md`

- [ ] **Step 1: Document the new foreground cadence**

Update `docs/adr/003-interactive-polling-strategy.md` so the accepted decision matches the new selected-preview timing.

- [ ] **Step 2: Run targeted tests**

Run:

```bash
cargo test poll_intervals_follow_preview_and_interactive_rules -- --nocapture
cargo test focused_preview_schedules_fast_poll_deadline -- --nocapture
cargo test session_performance_rows_preserve_background_cadence_when_selected_preview_is_fast -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run required local validation**

Run:

```bash
make precommit
```

Expected: PASS.

- [ ] **Step 4: Measure the user-visible result**

Open Grove's performance dialog and verify:

- the selected workspace row now shows the faster cadence
- redraw cost stays low
- perceived preview freshness is improved while watching an active session

Capture before/after notes in the implementation PR or follow-up comment.

- [ ] **Step 5: Commit**

```bash
git add docs/adr/003-interactive-polling-strategy.md docs/plans/2026-03-19-focused-preview-polling.md
git commit -m "docs: record focused preview polling plan"
```

## Success Criteria

- Selected preview updates noticeably faster while the preview pane is focused.
- Background workspaces keep their current low-cost polling behavior.
- Existing interactive debounce behavior still wins over adaptive polling.
- Targeted tests and `make precommit` pass.

## Non-Goals

- No tmux transport rewrite.
- No streaming preview channel.
- No changes to workspace status-target generation cadence.
- No background polling acceleration.
