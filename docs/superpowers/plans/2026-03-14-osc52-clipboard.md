# OSC52 Clipboard Support Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make copy operations work in local and headless Grove sessions by emitting OSC52 clipboard escapes alongside the existing system clipboard path.

**Architecture:** Keep the current synchronous clipboard read path unchanged. Refactor clipboard writes so platform command, arboard fallback, and OSC52 are attempted independently, with overall success when any write path succeeds. Add pure OSC52 formatting helpers so the terminal escape handling is testable without relying on stdout side effects.

**Tech Stack:** Rust, arboard, base64, targeted `cargo test`, `make precommit`

---

## Chunk 1: Clipboard Write Path

### Task 1: Add failing OSC52 formatting regressions

**Files:**
- Modify: `src/ui/tui/terminal/clipboard.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn osc52_sequence_formats_plain_payload() {
    let rendered = SystemClipboardAccess::osc52_sequence("hello", false);

    assert_eq!(rendered, b"\x1b]52;c;aGVsbG8=\x07");
}

#[test]
fn osc52_sequence_wraps_tmux_passthrough() {
    let rendered = SystemClipboardAccess::osc52_sequence("hello", true);

    assert_eq!(rendered, b"\x1bPtmux;\x1b\x1b]52;c;aGVsbG8=\x07\x1b\\");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test osc52_sequence_ -- --nocapture`
Expected: FAIL because the helper does not exist yet.

- [ ] **Step 3: Write minimal implementation**

```rust
fn osc52_sequence(text: &str, inside_tmux: bool) -> Vec<u8> {
    let payload = BASE64_STANDARD.encode(text.as_bytes());
    if inside_tmux {
        format!("\x1bPtmux;\x1b\x1b]52;c;{payload}\x07\x1b\\").into_bytes()
    } else {
        format!("\x1b]52;c;{payload}\x07").into_bytes()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test osc52_sequence_ -- --nocapture`
Expected: PASS

### Task 2: Add write-path aggregation regressions

**Files:**
- Modify: `src/ui/tui/terminal/clipboard.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn clipboard_write_succeeds_when_only_osc52_succeeds() {
    let result = combine_write_results(
        Err("platform failed".to_string()),
        Err("arboard failed".to_string()),
        Ok(()),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn clipboard_write_returns_all_errors_when_every_path_fails() {
    let result = combine_write_results(
        Err("platform failed".to_string()),
        Err("arboard failed".to_string()),
        Err("osc52 failed".to_string()),
    );

    assert_eq!(
        result,
        Err("platform failed; arboard: arboard failed; osc52: osc52 failed".to_string())
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test clipboard_write_ -- --nocapture`
Expected: FAIL because the aggregation helper does not exist yet.

- [ ] **Step 3: Write minimal implementation**

```rust
fn combine_write_results(
    platform_result: Result<(), String>,
    arboard_result: Result<(), String>,
    osc52_result: Result<(), String>,
) -> Result<(), String> {
    if platform_result.is_ok() || arboard_result.is_ok() || osc52_result.is_ok() {
        return Ok(());
    }

    let mut errors = Vec::new();
    if let Err(error) = platform_result {
        errors.push(error);
    }
    if let Err(error) = arboard_result {
        errors.push(format!("arboard: {error}"));
    }
    if let Err(error) = osc52_result {
        errors.push(format!("osc52: {error}"));
    }
    Err(errors.join("; "))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test clipboard_write_ -- --nocapture`
Expected: PASS

### Task 3: Wire OSC52 into real clipboard writes

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/ui/tui/terminal/clipboard.rs`

- [ ] **Step 1: Update dependencies and write path**

```rust
fn write_text(&mut self, text: &str) -> Result<(), String> {
    let platform_result = Self::write_text_with_platform_command(text);
    let arboard_result = self
        .clipboard()
        .and_then(|clipboard| clipboard.set_text(text.to_string()).map_err(|error| error.to_string()));
    let osc52_result = Self::write_osc52(text);
    combine_write_results(platform_result, arboard_result, osc52_result)
}
```

- [ ] **Step 2: Add unicode and size-limit tests**

Run: `cargo test clipboard -- --nocapture`
Expected: PASS with coverage for unicode payloads and oversized OSC52 payload rejection if a limit is implemented.

- [ ] **Step 3: Run focused verification**

Run: `cargo test clipboard -- --nocapture`
Expected: PASS

- [ ] **Step 4: Run required repo verification**

Run: `make precommit`
Expected: PASS
