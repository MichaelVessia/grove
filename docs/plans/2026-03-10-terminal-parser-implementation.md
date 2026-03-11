# Native Terminal Parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Grove's custom preview ANSI parsing with an app-local parsed snapshot built from `ftui_pty::virtual_terminal::VirtualTerminal`.

**Architecture:** Parse capture output once in `PreviewState`, store plain text plus parsed spans in app-local types, and convert parsed spans to ftui render types only in the TUI view layer. Keep capture diffing and mouse-noise cleanup where they already live.

**Tech Stack:** Rust, ftui, ftui-pty, Grove preview state, TDD

---

### Task 1: Add preview snapshot types

**Files:**
- Modify: `src/application/preview.rs`
- Test: `src/application/preview.rs`

**Step 1: Write the failing test**

Add a unit test in `src/application/preview.rs` asserting `apply_capture("a\u{1b}[31mb\u{1b}[0mc")` produces:
- `lines == vec!["abc".to_string()]`
- one parsed line
- three parsed spans
- middle span text `"b"`
- middle span contains a foreground color

**Step 2: Run test to verify it fails**

Run: `cargo test apply_capture_builds_plain_and_styled_preview_from_ansi`
Expected: FAIL because parsed preview snapshot types do not exist yet.

**Step 3: Write minimal implementation**

In `src/application/preview.rs`:
- add `PreviewParsedLine`
- add `PreviewParsedSpan`
- add `PreviewParsedStyle`
- add a placeholder `parsed_lines: Vec<PreviewParsedLine>` field to `PreviewState`
- implement the minimum `apply_capture` behavior needed for the new test shape

**Step 4: Run test to verify it passes**

Run: `cargo test apply_capture_builds_plain_and_styled_preview_from_ansi`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/preview.rs
git commit -m "test: add preview parsed snapshot types"
```

### Task 2: Add ftui-pty-backed capture parsing

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/application/preview.rs`
- Test: `src/application/preview.rs`

**Step 1: Write the failing test**

Add a unit test asserting ANSI style carries across lines:
- capture `"a\u{1b}[31mb\nc\n\u{1b}[0md"`
- plain lines become `["ab", "c", "d"]`
- second line first span still has a foreground color
- third line has reset styling

**Step 2: Run test to verify it fails**

Run: `cargo test apply_capture_carries_style_across_lines_until_reset`
Expected: FAIL because parsing is not yet VT-backed.

**Step 3: Write minimal implementation**

- add `ftui-pty` dependency pinned to the same FrankenTUI tag as `ftui`
- build a `VirtualTerminal` from `change.render_output`
- size the terminal from parsed input content with a minimum of `1x1`
- convert VT rows into span-based `PreviewParsedLine`
- derive `lines` from the same VT rows

**Step 4: Run test to verify it passes**

Run: `cargo test apply_capture_carries_style_across_lines_until_reset`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml src/application/preview.rs
git commit -m "feat: parse preview output with ftui terminal backend"
```

### Task 3: Cover VT-only behavior Grove did not model well

**Files:**
- Modify: `src/application/preview.rs`
- Test: `src/application/preview.rs`

**Step 1: Write the failing test**

Add regression tests for:
- carriage return overwrite, for example `"hello\rxy"`
- erase in line, for example `"abcdef\u{1b}[1;4H\u{1b}[K"`
- OSC/title stripping from plain text

**Step 2: Run tests to verify they fail**

Run: `cargo test preview_`
Expected: At least the new VT behavior cases FAIL before the implementation covers them.

**Step 3: Write minimal implementation**

Adjust VT sizing, row extraction, and span coalescing logic only as needed to satisfy the new behavior.

**Step 4: Run tests to verify they pass**

Run: `cargo test preview_`
Expected: PASS for the new regression coverage.

**Step 5: Commit**

```bash
git add src/application/preview.rs
git commit -m "test: cover terminal control behavior in preview parsing"
```

### Task 4: Move preview rendering to parsed snapshot data

**Files:**
- Modify: `src/ui/tui/view/view_preview_content.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add or update a preview rendering test in `src/ui/tui/mod.rs` that sets preview parsed state and asserts rendered output still contains colored spans for ANSI-derived preview content.

**Step 2: Run test to verify it fails**

Run: `cargo test preview_pane_renders_ansi_colors`
Expected: FAIL because view code still expects string-based `render_lines`.

**Step 3: Write minimal implementation**

- replace string-based preview render consumption with parsed preview lines
- convert parsed spans to `FtLine<'static>` only inside the view layer
- keep cursor overlay behavior working on the rendered preview slice

**Step 4: Run test to verify it passes**

Run: `cargo test preview_pane_renders_ansi_colors`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_preview_content.rs src/ui/tui/model.rs src/ui/tui/mod.rs
git commit -m "feat: render preview from parsed terminal snapshot"
```

### Task 5: Move plain-text selection to parsed preview state

**Files:**
- Modify: `src/ui/tui/view/view_selection_mapping.rs`
- Modify: `src/ui/tui/view/view_selection_logging.rs`
- Modify: `src/ui/tui/view/view_selection_interaction.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a regression test proving selection plain text comes from the parsed preview state, not raw ANSI stripping. The selected text should exclude control sequences while matching the rendered visible content.

**Step 2: Run test to verify it fails**

Run: `cargo test preview_selection`
Expected: FAIL because selection code still reads `render_lines` and the custom ANSI plain-text path.

**Step 3: Write minimal implementation**

- route selection/plain-text helpers through `preview.lines`
- remove dependencies on the custom ANSI plain-text stripper
- update any logging paths that still refer to string-based `render_lines`

**Step 4: Run test to verify it passes**

Run: `cargo test preview_selection`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_selection_mapping.rs src/ui/tui/view/view_selection_logging.rs src/ui/tui/view/view_selection_interaction.rs src/ui/tui/mod.rs
git commit -m "refactor: use parsed preview state for selection"
```

### Task 6: Delete custom ANSI parser code and old tests

**Files:**
- Modify: `src/ui/tui/ansi.rs`
- Delete: `src/ui/tui/ansi/colors.rs`
- Delete: `src/ui/tui/ansi/parser.rs`
- Delete or modify: `src/ui/tui/text/ansi_plain.rs`
- Modify: `src/ui/tui/text.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add or update a compile-level test step by removing old imports and references first.

**Step 2: Run targeted tests to verify failure**

Run: `cargo test ansi_`
Expected: FAIL or compile errors due to remaining references to deleted custom parser paths.

**Step 3: Write minimal implementation**

- remove old ANSI module exports
- remove obsolete tests for theme remapping
- update remaining call sites to the new preview snapshot path

**Step 4: Run tests to verify it passes**

Run: `cargo test preview_`
Expected: PASS with no remaining custom ANSI parser references.

**Step 5: Commit**

```bash
git add src/ui/tui/ansi.rs src/ui/tui/text.rs src/ui/tui/mod.rs src/ui/tui/ansi/colors.rs src/ui/tui/ansi/parser.rs src/ui/tui/text/ansi_plain.rs
git commit -m "refactor: remove custom ansi preview parser"
```

### Task 7: Validate the touched preview path

**Files:**
- Modify: `src/application/preview.rs`
- Modify: `src/ui/tui/mod.rs`
- Modify: any touched preview/view files from earlier tasks

**Step 1: Run focused tests**

Run:
- `cargo test apply_capture_`
- `cargo test preview_pane_renders_ansi_colors`
- `cargo test codex_live_preview_capture_keeps_tmux_escape_output`
- `cargo test claude_live_preview_capture_keeps_tmux_escape_output`

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`
Expected: PASS

**Step 3: Commit final cleanup**

```bash
git add Cargo.toml src/application/preview.rs src/ui/tui docs/plans/2026-03-10-terminal-parser-design.md docs/plans/2026-03-10-terminal-parser-implementation.md
git commit -m "refactor: replace preview ansi parser with ftui terminal backend"
```
