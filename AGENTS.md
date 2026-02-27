# Grove

A minimal workspace manager for AI coding agents. Rust + FrankenTUI.

## Reference Codebases

The `.reference/` directory contains two codebases you should consult
heavily:

- `.reference/frankentui/` -- the TUI framework Grove is built on (Elm/MVU
  architecture, widgets, layout, subscriptions, hit testing, rendering)
- `.reference/sidecar/` -- the Go application Grove is a simplified port of
  (tmux integration, interactive mode, polling, status detection, worktree
  management, mouse handling, modal dialogs, pane resize)

**Use these before inventing anything.** Grove is largely a subset of
sidecar rewritten in Rust on FrankenTUI. Most architectural decisions
(especially around terminal management, tmux session lifecycle, adaptive
polling, cursor overlay, key forwarding, output capture) should match
sidecar's proven patterns. When you're unsure how to implement something,
read the corresponding sidecar code first. When you're unsure how to use
an ftui API, read the FrankenTUI source and examples.

Specific mapping:

| Grove concern | Sidecar reference |
|---|---|
| Tmux sessions, capture, send-keys | `internal/tty/` |
| Interactive mode, key forwarding | `internal/tty/model.go`, `keymap.go` |
| Polling, output buffer, change detection | `internal/tty/output_buffer.go` |
| Mouse hit testing, drag-to-resize | `internal/mouse/` |
| Modal dialogs | `internal/modal/` |
| Workspace list, status icons | `internal/plugins/workspace/` |
| Agent status detection | `internal/plugins/workspace/agent.go` |
| Worktree operations | `internal/app/git.go` |

| Grove concern | FrankenTUI reference |
|---|---|
| Model/Update/View pattern | `ftui-runtime/src/program.rs` |
| Subscriptions (polling ticks) | `ftui-runtime/src/subscription.rs` |
| Layout (Flex, Constraint) | `ftui-layout/src/` |
| Hit regions (mouse) | `ftui-render/src/frame.rs` (HitGrid) |
| Widgets (TextInput, Block) | `ftui-widgets/src/` |
| Styling (colors, attrs) | `ftui-style/src/` |
| Buffer/Cell rendering | `ftui-render/src/buffer.rs`, `cell.rs` |

## Project Structure

```text
docs/PRD.md               -- full product requirements + technical implementation
docs/
  adr/
.reference/
  frankentui/             -- TUI framework (Rust, Elm architecture)
  sidecar/                -- reference app (Go, Bubble Tea)
```

## Workflow

```bash
# Review product requirements
cat docs/PRD.md
```

## Local Checks

```bash
# Fast local checks (pre-commit hook)
make precommit

# Full checks (CI parity)
make ci
```

- For local code changes, run `make precommit` before handoff.
- Treat `make precommit` as required minimum validation for local edits.
- Use `make ci` only when explicitly requested or when validating full CI parity.

## Runtime Parity

- Any change that touches session lifecycle, capture, polling, key forwarding,
  status detection, or workspace runtime behavior must preserve tmux behavior
  and include matching tests.

## Keybind + Command Discoverability

- Whenever adding or changing a keybind or command, update both the Keybind
  Help modal content and the Command Palette actions so UI discoverability
  stays in sync.
