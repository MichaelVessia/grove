# ADR-003: Interactive Polling Strategy

**Status:** Accepted  
**Date:** 2026-02-13

## Context

Interactive mode must keep key-to-preview latency low while avoiding runaway
polling cost. Grove forwards input into tmux and renders captured output from
the same session, so transport and polling strategy directly control perceived
typing lag.

## Decision

Use a Sidecar-aligned hybrid polling strategy:

1. Debounced interactive key polls at `20ms`.
2. Adaptive background polling with decay (`50ms`, `200ms`, `500ms`, and
   status-based slower intervals outside active interaction).
3. Stream-first selected preview transport for the focused foreground preview,
   backed by tmux control mode for one session at a time.
4. Selected preview polling is allowed exactly once as a bootstrap capture to
   seed the pane, then disabled in favor of the stream-only path.
5. Earliest-deadline scheduling for ticks, where an already-earlier pending
   tick is retained and never postponed by newer key events.
6. Interactive debounce deadlines are cleared once consumed or on interactive
   exit.

## Details

- Interactive key/paste forwarding schedules a debounced interactive poll
  deadline (`now + 20ms`).
- When the selected preview is focused, Grove opens a dedicated tmux control
  mode stream for that session and applies incremental `%output` updates
  through the existing live preview path.
- The selected preview may perform one bootstrap capture before the stream is
  healthy, then foreground polling for that session is suppressed.
- If the stream disconnects or errors after bootstrap, the preview remains
  visible but stops receiving new selected-preview updates until the stream is
  re-established or Grove restarts.
- Global tick scheduling computes the next adaptive deadline and the interactive
  debounce deadline, then selects the earliest.
- If a tick is already pending earlier than the new target, no replacement
  timer is scheduled.
- Tick processing clears consumed interactive debounce deadlines and then polls
  preview output/cursor as normal.

## Consequences

- Continuous typing cannot starve polling by repeatedly pushing deadlines out.
- Active selected previews no longer wait on the adaptive poll timer while the
  stream is healthy.
- Fast bursts are still coalesced, but bounded by a short debounce window.
- Watching a selected preview in the preview pane reduces foreground staleness
  materially and removes steady-state selected-preview polling cost.
- Adaptive polling behavior remains unchanged for non-interactive paths.
- A broken selected preview stream now fails visibly as `connecting` or
  `disconnected` rather than silently reverting to polling.
- Event logs can distinguish adaptive scheduling, debounce scheduling, retained
  timers, processed ticks, and preview stream connect or disconnect events for
  deterministic latency debugging.
