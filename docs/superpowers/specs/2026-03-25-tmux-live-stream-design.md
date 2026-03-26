# Tmux Live Stream Design

## Goal

Improve Grove terminal fidelity while keeping tmux as the runtime owner.

Replace snapshot-driven selected-session preview and interactive rendering with a
tmux control-mode live stream backed by shared terminal state. Keep
`capture-pane` as bootstrap and recovery, not as the primary steady-state render
source.

## Context

Grove is explicitly tmux-backed. Today selected-session preview already uses a
tmux control-mode subscription, but only as a trigger. On `%output` events it
still falls back to `capture-pane -e`, rebuilds a snapshot, and redraws from
that snapshot.

That gives Grove persistence and detachability, but preview fidelity still
differs from native Codex because:

- output is polled or snapshotted rather than streamed as terminal state
- non-SGR control sequences are stripped before render
- preview and interactive state are not owned by one shared terminal model
- Grove theme defaults repaint unstyled cells during redraw

## Decision

Use a tmux-first live stream architecture.

- tmux remains the process owner and session source of truth
- Grove opens long-lived `tmux -C` control-mode clients for selected sessions
- Grove applies incremental tmux output to a shared in-memory terminal core
- preview and inline interactive mode render from that same terminal core
- `capture-pane` remains for initial bootstrap, reconnect recovery, and
  desync/backfill fallback

Do not replace tmux with child PTYs.

## Architecture

### Transport

Add a dedicated tmux live transport layer centered on a `TmuxControlClient`.

Responsibilities:

- resolve pane/session targets
- own a long-lived `tmux -C attach-session ...` process
- parse control-mode lines into typed Grove events
- stream output incrementally
- surface connection lifecycle, transport errors, and session exit
- reconnect when the stream drops

The existing `preview_stream.rs` path is the starting point, but it must be
promoted from "stream as trigger" to "stream as source".

### Terminal Core

Introduce session-local terminal state shared by preview and interactive mode.

Responsibilities:

- incremental ANSI/terminal state application
- cell grid ownership
- cursor metadata
- pane dimensions
- transport health, `stream` vs `fallback`
- bootstrap / recovery generation tracking

This terminal core becomes the render source for the selected session. It
replaces repeated snapshot reconstruction as the steady-state model.

### UI Integration

Preview and inline interactive mode should stop owning separate output models.

- preview renders terminal-core cells/spans
- interactive mode uses the same rendered state plus cursor placement
- input still routes through tmux
- resize still routes through tmux
- selected-session polling is suppressed while stream health is good

Background status polling for non-selected sessions can remain snapshot-based.

## Data Flow

1. User selects a running session or enters interactive mode.
2. Grove retargets `TmuxControlClient` to that session.
3. Grove resolves the active pane id.
4. Grove performs one bootstrap snapshot:
   - `capture-pane -e`
   - cursor metadata query
5. Terminal core is seeded from that bootstrap snapshot.
6. Grove starts the control-mode client.
7. `%output` and `%extended-output` messages are decoded to terminal bytes and
   applied incrementally to the terminal core.
8. Preview and interactive render directly from terminal-core state.
9. Keyboard, paste, and resize actions go to tmux immediately.
10. Resulting terminal updates arrive back through the same live stream.

## Failure Model

Default posture: degrade, do not break.

- If the control stream drops, Grove falls back to the current `capture-pane`
  selected-session path.
- If reconnect succeeds, Grove performs a fresh bootstrap snapshot and resumes
  live streaming.
- If the parser detects malformed or unusable stream output, treat it as desync,
  log it, reset from snapshot, and continue.
- If tmux reports the session or pane is gone, retain current missing-session
  behavior as the source of truth.
- Show warning toasts only for repeated reconnect failures or explicit operator
  action failures, not for every transient stream drop.

## Scope

In scope:

- selected-session live stream for preview
- selected-session live stream for inline interactive mode
- shared selected-session terminal core
- bootstrap and reconnect fallback using `capture-pane`
- rendering changes needed to prefer terminal-core cell styling
- resize and cursor parity for selected live sessions

Out of scope:

- replacing tmux with direct child PTYs
- full session streaming for every background session
- removing `capture-pane` entirely
- tmux attach UX changes

## Implementation Slices

### Slice 1, Transport

Extend `src/ui/tui/terminal/preview_stream.rs`:

- parse control-mode output into typed incremental events
- stop mapping `%output` to full `capture-pane` snapshots on every event
- add reconnect / fallback policy

Likely supporting files:

- `src/ui/tui/terminal/preview_stream.rs`
- possibly a new parser/helper module if the file boundary gets too wide

### Slice 2, Terminal State

Refactor `src/application/preview.rs`:

- separate one-shot snapshot parsing from persistent terminal ownership
- add shared selected-session terminal state
- allow incremental byte application

Likely supporting files:

- `src/application/preview.rs`
- `src/ui/tui/model.rs`
- `src/ui/tui/msg.rs`

### Slice 3, Interactive Integration

Update interactive handling so render state comes from the shared terminal core
rather than a separate preview approximation.

Likely supporting files:

- `src/application/interactive.rs`
- `src/ui/tui/update/update_input_key_events.rs`
- `src/ui/tui/update/update_navigation_preview.rs`
- `src/ui/tui/view/view_preview.rs`

### Slice 4, Polling / Recovery

Keep existing polling paths as degraded-mode recovery and for non-selected
sessions.

Likely supporting files:

- `src/ui/tui/update/update_polling_capture_dispatch.rs`
- `src/ui/tui/update/update_polling_capture_live.rs`
- `src/ui/tui/update/update_polling_capture_workspace.rs`

### Slice 5, Rendering Fidelity

Render from terminal-core cells where possible and reduce Grove theme repainting
for cells that already carry explicit terminal fg/bg state.

Likely supporting files:

- `src/ui/tui/view/view_preview.rs`
- `src/ui/tui/view/view_preview_content.rs`

## Risks

- tmux control-mode parsing is more complex than snapshot polling
- reconnect and generation ownership bugs could create stale render state
- resize synchronization may still lag under rapid layout changes
- terminal-core refactor may touch preview, interactive, replay, and status
  assumptions together

## Testing

Use TDD for each slice.

Critical regressions:

- control-mode `%output` applies incremental terminal updates without invoking
  `capture-pane`
- preview and interactive mode share the same selected-session terminal state
- disconnect falls back to snapshot mode, reconnect reseeds and resumes stream
- resize updates target pane size and subsequent output reflows correctly
- session exit still updates workspace/task runtime state correctly

Keep existing regressions green:

- theme-based preview rendering
- session lifecycle / missing-session behavior
- preview stream generation and disconnect behavior
- interactive input forwarding behavior

## Recommendation

Implement this in small slices behind the current fallback path. The first
successful milestone is not "delete polling", it is "selected preview stays
correct from live control-mode bytes, and failure degrades safely to today's
behavior".
