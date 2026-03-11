# Native Terminal Parser Design

**Goal:** Replace Grove's custom preview ANSI parsing with ftui's native terminal parser backend, while keeping preview rendering and text selection aligned on one parsed source of truth.

## Decision

Use `ftui_pty::virtual_terminal::VirtualTerminal` at capture time in `src/application/preview.rs`.

Do not keep Grove's theme-aware ANSI palette remap. Use ftui's standard ANSI colors.

Do not store `FtLine<'static>` in `PreviewState`. Store an app-local parsed snapshot and convert to ftui text types only at the view edge.

## Why

- The current custom ANSI logic is isolated and replaceable.
- Parsing once in application state is cleaner than reparsing visible slices in view code.
- A parsed snapshot gives one source of truth for both styled preview rendering and plain-text selection.
- ftui's terminal model covers more ANSI/VT behavior than Grove's handwritten SGR parser.

## Architecture

`PreviewState::apply_capture` remains the entry point for new captured output.

It continues to use `evaluate_capture_change(...)` for change detection and mouse-noise cleanup. When the raw render output changes, `apply_capture` feeds `change.render_output` into a `VirtualTerminal`, then derives:

- `lines: Vec<String>` for plain-text consumers
- `parsed_lines: Vec<PreviewParsedLine>` for styled preview rendering

The view layer no longer parses ANSI. It only converts `PreviewParsedLine` values into `FtLine<'static>` when rendering.

## Data Model

Replace `render_lines: Vec<String>` in `PreviewState` with an app-local parsed representation:

- `PreviewParsedLine`
- `PreviewParsedSpan`
- `PreviewParsedStyle`

The parsed representation should be span-based, not cell-based. During capture, adjacent cells with identical style are coalesced into spans. This keeps state smaller and makes view conversion straightforward.

`lines` stays as plain text because several existing consumers already depend on it.

## Behavior

The new parser path intentionally adopts ftui terminal semantics:

- SGR styling uses ftui's standard palette
- Style can carry across lines until reset
- Carriage return, erase, and cursor movement can affect visible text
- OSC and other non-printing control sequences do not leak into plain text
- DEC charset handling comes from the VT parser, not Grove-specific stripping
- 256-color and truecolor are preserved

This is not a viewport-local parser. Parsing happens against the full capture content, then the view slices parsed lines.

## Testing

Use TDD for the implementation.

Primary regression coverage:

- `PreviewState::apply_capture` builds plain text and parsed spans from ANSI output
- style carry-over across lines remains correct
- carriage return and erase sequences affect final visible rows correctly
- OSC and charset sequences do not leak into plain text
- preview rendering still shows styled output from parsed preview state
- selection mapping still reads the plain-text representation
- old theme-remap assertions are removed

## Migration Notes

- Rename the preview render field instead of preserving the old `render_lines` name
- Delete `src/ui/tui/ansi/colors.rs`
- Delete Grove's custom ANSI parser implementation in `src/ui/tui/ansi/parser.rs`
- Replace the custom ANSI plain-text stripper with the parsed preview snapshot path

## Open Constraints

- `VirtualTerminal` itself is not an ideal long-lived state type here because it would leak parser internals into app state and complicate trait derives used in tests. Build an app-local immutable snapshot from it instead.
- `evaluate_capture_change(...)` still owns diffing and mouse-fragment cleanup. This change is about rendering/parser replacement, not capture diff policy.
