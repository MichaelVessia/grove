# Grove

A minimal workspace manager for AI coding agents. Rust + FrankenTUI.

## Reference Codebases

The `.reference/` directory contains reference codebases:

- `.reference/frankentui/` -- the TUI framework Grove is built on (Elm/MVU
  architecture, widgets, layout, subscriptions, hit testing, rendering)
- `.reference/sidecar/` -- a Go TUI app useful for tmux interaction patterns
  (session lifecycle, key forwarding, escape handling, mouse fragment
  suppression). Consult selectively for low-level terminal behavior only.

## FrankenTUI-First UI Development (MANDATORY)

**NEVER build custom UI components without first searching the ftui crates for
an existing implementation.** FrankenTUI is a large, feature-rich framework.
Always use native widgets over custom implementations.

Before writing ANY new UI code (widgets, layout helpers, text utilities, input
handling, overlays, animations, styling), you MUST:

1. Search `.reference/frankentui/crates/` for existing widgets, traits, or
   utilities that solve the problem.
2. Read the relevant ftui source to understand its API surface.
3. Only build custom if you can confirm no native ftui solution exists, and
   document why in a code comment.

ftui provides far more than basic widgets. Areas commonly overlooked:

- **Overlays/Modals**: `Modal`, `ModalStack`, `Dialog`, `Toast`,
  `NotificationQueue`, `FocusTrap`
- **Command palette**: `CommandPalette` with fuzzy scoring, match highlighting
- **Help system**: `HelpRegistry`, `Help`, `HelpIndex`, `HintRanker`
- **Focus**: `FocusManager`, `FocusGroup`, `FocusTrap`, `FocusIndicator`,
  spatial navigation
- **Text**: `TextArea` (selection, line numbers, scrolling), `Editor`, `Wrap`,
  `FitMetrics`, `Measurable`
- **Theming**: `Theme`, `ThemeBuilder`, `AdaptiveColor`, `StyleSheet`
- **Lists**: `VirtualizedList`, `Tree`, `Table` with persist/undo support
- **Animation**: `Animation` (spring, timeline, stagger), `Spinner`, `Progress`
- **Layout**: `Responsive`, `Grid`, `Columns`, `Workspace` pane layout
- **Interaction**: `Drag` protocol, `Scrollbar`, `StatusLine`, `Badge`, `Rule`
- **Terminal**: `TerminalParser` (ANSI/VT), `Sanitize`
- **Rendering**: `CachedWidget`, `Budgeted`, `DegradationLevel`

| Grove concern                 | FrankenTUI reference                   |
| ----------------------------- | -------------------------------------- |
| Model/Update/View pattern     | `ftui-runtime/src/program.rs`          |
| Subscriptions (polling ticks) | `ftui-runtime/src/subscription.rs`     |
| Layout (Flex, Constraint)     | `ftui-layout/src/`                     |
| Hit regions (mouse)           | `ftui-render/src/frame.rs` (HitGrid)   |
| Widgets (TextInput, Block)    | `ftui-widgets/src/`                    |
| Styling (colors, attrs)       | `ftui-style/src/`                      |
| Buffer/Cell rendering         | `ftui-render/src/buffer.rs`, `cell.rs` |

## Project Structure

```text
docs/PRD.md               -- full product requirements + technical implementation
docs/debug-replay.md      -- human + agent replay debugging workflow
docs/
  adr/
.reference/
  frankentui/             -- TUI framework (Rust, Elm architecture)
  sidecar/                -- reference app (Go, Bubble Tea)
.agents/skills/
  replay-debug/           -- project skill for debug-record replay workflows
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
- Use `make ci` only when explicitly requested or when validating full CI
  parity.

## Runtime Parity

- Any change that touches session lifecycle, capture, polling, key forwarding,
  status detection, or workspace runtime behavior must preserve tmux behavior
  and include matching tests.

## Keybind + Command Discoverability

- Whenever adding or changing a keybind or command, update both the Keybind Help
  modal content and the Command Palette actions so UI discoverability stays in
  sync.

## Commit Messages (Conventional Commits)

All commits MUST use [Conventional Commits](https://www.conventionalcommits.org/)
format. This is required for accurate changelog generation via release-please.

Format: `type(optional-scope): lowercase description`

Allowed types and when to use each:

| Type       | Use when                                                      |
| ---------- | ------------------------------------------------------------- |
| `feat`     | Adding new user-facing functionality                          |
| `fix`      | Fixing a bug or correcting broken behavior                    |
| `refactor` | Restructuring code without changing behavior                  |
| `test`     | Adding or updating tests only                                 |
| `docs`     | Documentation-only changes                                    |
| `chore`    | Maintenance (deps, ignores, config) with no runtime effect    |
| `ci`       | CI/CD pipeline changes                                       |
| `perf`     | Performance improvements                                      |
| `style`    | Formatting, whitespace, linting (no logic change)             |
| `build`    | Build system or dependency changes affecting compilation      |
| `revert`   | Reverting a previous commit                                   |

Rules:

- Description must be lowercase, imperative mood ("add X", not "Added X").
- Scope is optional but encouraged when the change is clearly scoped to a
  subsystem (e.g., `fix(preview):`, `feat(tui):`).
- Breaking changes: add `!` after type/scope (e.g., `feat!:` or
  `refactor(runtime)!:`) and explain in the commit body.
- Never use bare descriptions without a type prefix.
- Pick the type that matches the **primary intent**, not a secondary effect.
  A bug fix that also adds a test is `fix`, not `test`.

## Replay Debugging

- For runtime race, polling, interactive input, or rendering regressions,
  capture a debug record first, then replay it.
- Human runbook:
  - `docs/debug-replay.md`
- Project skill:
  - `.agents/skills/replay-debug/SKILL.md`
- Default replay commands:
  - `cargo run -- --debug-record`
  - `cargo run -- replay <trace-path>`
  - `cargo run -- replay <trace-path> --snapshot <path>`
  - `cargo run -- replay <trace-path> --emit-test <name>`
