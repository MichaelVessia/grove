# Grove Doctor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a diagnosis-only `grove doctor` CLI that audits Grove state and prints findings plus a deterministic repair plan without mutating anything.

**Architecture:** Introduce a dedicated doctor service that gathers manifests, configured repos, filesystem facts, and Grove-managed tmux sessions into a single diagnosis model. Keep diagnosis pure where possible, render through the CLI only, and reuse existing task-first lifecycle and cleanup rules rather than inventing new mutation paths.

**Tech Stack:** Rust, existing Grove config/task/session helpers, CLI tests, unit tests.

---

## Chunk 1: Diagnosis Core

### Task 1: Add diagnosis data model and pure repair-plan reducer

**Files:**
- Create: `src/application/doctor.rs`
- Modify: `src/application/mod.rs`
- Test: `src/application/doctor.rs`

- [ ] **Step 1: Write failing unit tests for summary counting and repair-plan ordering**

Add tests that construct findings in memory and assert:
- severity counts are correct
- duplicate root-cause findings do not create duplicate repair steps
- repair steps are ordered deterministically by priority and target

- [ ] **Step 2: Run targeted tests and verify they fail**

Run: `cargo test doctor::tests:: -- --nocapture`
Expected: FAIL because `src/application/doctor.rs` does not exist yet.

- [ ] **Step 3: Implement the diagnosis model and reducer**

Add:
- `DoctorSeverity`
- `DoctorFindingKind`
- `DoctorFinding`
- `DoctorSubject`
- `DoctorSummary`
- `DoctorRepairAction`
- `DoctorRepairStep`
- `DoctorReport`

Add a pure helper that turns `Vec<DoctorFinding>` into:
- grouped summary counts
- deterministic repair-plan steps

Keep the model serializable for later CLI JSON output.

- [ ] **Step 4: Run targeted tests and verify they pass**

Run: `cargo test doctor::tests:: -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/application/mod.rs src/application/doctor.rs
git commit -m "feat: add grove doctor report model"
```

## Chunk 2: State Collection

### Task 2: Diagnose manifests, repos, and worktree filesystem drift

**Files:**
- Modify: `src/application/doctor.rs`
- Modify: `src/infrastructure/task_manifest.rs`
- Test: `src/application/doctor.rs`

- [ ] **Step 1: Write failing diagnosis tests for manifest and repo drift**

Add fixture-style tests covering:
- invalid manifest file yields `invalid_task_manifest`
- two manifests with same slug yield `duplicate_task_slug`
- manifest worktree path missing on disk yields `missing_worktree_path`
- manifest worktree missing `.grove/base` yields `missing_base_marker`
- configured repo not represented by a base task yields `configured_repo_missing_base_task_manifest`

- [ ] **Step 2: Run targeted tests and verify they fail**

Run: `cargo test doctor::tests::manifest -- --nocapture`
Expected: FAIL because diagnosis does not inspect these cases yet.

- [ ] **Step 3: Implement filesystem and config-backed diagnosis**

In `src/application/doctor.rs`:
- enumerate task manifest paths under the resolved tasks root
- decode manifests, capturing decode failures as findings instead of aborting
- track slug ownership to detect duplicates
- check each decoded worktree path exists
- check each decoded worktree has a non-empty `.grove/base` marker where Grove expects one
- compare configured repos against manifest-backed base tasks using existing path-equivalence helpers

Only add helpers to `src/infrastructure/task_manifest.rs` if a small parse wrapper is needed to preserve decode errors with manifest path context.

- [ ] **Step 4: Run targeted tests and verify they pass**

Run: `cargo test doctor::tests::manifest -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/application/doctor.rs src/infrastructure/task_manifest.rs
git commit -m "feat: diagnose grove manifest and repo drift"
```

## Chunk 3: Session Diagnosis

### Task 3: Diagnose Grove-managed and legacy tmux session drift

**Files:**
- Modify: `src/application/doctor.rs`
- Modify: `src/application/session_cleanup.rs`
- Test: `src/application/doctor.rs`

- [ ] **Step 1: Write failing tests for session findings**

Add tests for:
- orphaned Grove task/worktree session yields `orphaned_grove_session`
- stale auxiliary session yields `stale_auxiliary_session`
- legacy Grove session missing metadata yields `legacy_grove_session_missing_metadata`
- tmux unavailable degrades to a warning instead of command failure inside the diagnosis layer

- [ ] **Step 2: Run targeted tests and verify they fail**

Run: `cargo test doctor::tests::sessions -- --nocapture`
Expected: FAIL because doctor does not inspect sessions yet.

- [ ] **Step 3: Implement session collection and classification**

Reuse existing session cleanup logic where possible:
- extract a small reusable session inventory helper from `src/application/session_cleanup.rs` if needed
- classify canonical Grove task/worktree sessions against discovered tasks
- detect legacy `grove-ws-*` sessions that lack current metadata semantics

Do not kill or mutate sessions. Return findings only.

- [ ] **Step 4: Run targeted tests and verify they pass**

Run: `cargo test doctor::tests::sessions -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/application/doctor.rs src/application/session_cleanup.rs
git commit -m "feat: diagnose grove session drift"
```

## Chunk 4: CLI Surface

### Task 4: Add `grove doctor` CLI and machine-readable output

**Files:**
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`
- Test: `src/cli/mod.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests for:
- `doctor` enables doctor mode
- `doctor --json` enables JSON rendering
- `doctor` cannot be combined with replay, benchmark, or cleanup modes
- doctor exits successfully on healthy reports
- doctor exits non-zero when warnings/errors exist

- [ ] **Step 2: Run targeted tests and verify they fail**

Run: `cargo test cli::tests::doctor -- --nocapture`
Expected: FAIL because doctor CLI mode is not implemented.

- [ ] **Step 3: Implement CLI parsing, rendering, and exit behavior**

In `src/cli/mod.rs`:
- add doctor args to `CliArgs`
- parse `doctor` and `--json`
- call the diagnosis service
- render:
  - human summary + findings + repair plan
  - JSON envelope for `--json`
- return non-zero on actionable findings

Keep human output compact and stable enough for test assertions.

- [ ] **Step 4: Run targeted tests and verify they pass**

Run: `cargo test cli::tests::doctor -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cli/mod.rs src/main.rs
git commit -m "feat: add grove doctor cli"
```

## Chunk 5: Docs And Validation

### Task 5: Document the new workflow and run required validation

**Files:**
- Modify: `README.md`
- Modify: `docs/PRD.md`
- Test: `src/application/doctor.rs`
- Test: `src/cli/mod.rs`

- [ ] **Step 1: Update docs**

Document:
- `grove doctor`
- `grove doctor --json`
- diagnosis-only scope
- expected workflow: run doctor, inspect repair plan, have an agent perform fixes

Add the command to the CLI section in `README.md`.

- [ ] **Step 2: Run modified targeted tests**

Run:
- `cargo test doctor::tests:: -- --nocapture`
- `cargo test cli::tests::doctor -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Run required repo validation**

Run: `make precommit`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/PRD.md src/application/doctor.rs src/cli/mod.rs src/main.rs src/application/session_cleanup.rs src/infrastructure/task_manifest.rs src/application/mod.rs
git commit -m "feat: add grove doctor diagnosis command"
```

## Notes For The Implementer

- Keep v1 diagnosis-only. Reject any temptation to add `--apply`.
- Prefer reusing existing helpers over duplicating task/session classification logic.
- Keep the finding taxonomy intentionally small and defensible.
- Do not compromise task-first architecture by adding fallback runtime behavior.
- Tests should prove the command is non-mutating by observing filesystem/session state before and after where practical.
