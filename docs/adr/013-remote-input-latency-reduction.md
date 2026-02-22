# ADR-013: Remote Input Latency Reduction

**Status:** Proposed
**Date:** 2026-02-21

## Context

ADR-012 introduced persistent daemon connections with pipelined send-keys,
eliminating per-keystroke socket connections and response waits. This removed
the worst latency (~50-100ms connect + round-trip per key), but interactive
typing on remote workspaces still feels laggy. Profiling the remaining pipeline
reveals three bottlenecks.

### Bottleneck 1: Per-keystroke tmux subprocess spawn

Every keystroke dispatches through the daemon to
`infrastructure::tmux::execute_command`, which calls
`Command::new("tmux").args(["send-keys", ...]).output()`. Each call is a
fork+exec+wait of a tmux subprocess (~5-20ms). Typing "hello" spawns 5
separate tmux processes, executed serially in the daemon's connection handler
loop. This is now the dominant source of input lag.

**Path:** client `pipeline_send_keys` -> daemon `handle_connection` loop ->
`dispatch_request` -> `handle_session_send_keys_request` ->
`tmux::execute_command` -> `process::execute_command` (fork+exec+wait)

### Bottleneck 2: Unread response backpressure

The daemon writes a JSON response for every pipelined send-keys request
(`daemon.rs:744-751`), even though the client never reads responses from the
persistent stream (fire-and-forget). Responses accumulate in the OS socket
receive buffer on the client side. Once that buffer fills (~128-256KB on macOS,
roughly 3000-8000 unread responses), the daemon's `writer.flush()` blocks,
stalling all subsequent request processing on that connection. This manifests
as sudden input freezes after sustained typing.

### Bottleneck 3: Output capture poll round-trip

After each keystroke, a debounced poll (20ms) triggers a `SessionCapture`
request that traverses the full round-trip: client -> SSH tunnel -> daemon ->
`tmux capture-pane` subprocess -> response -> SSH tunnel -> client. This is
inherent to the polling architecture and is not addressed in this ADR (see
ADR-012 Option B for the eventual solution via direct SSH PTY).

### Non-solution: Mosh

Mosh provides local echo prediction and UDP transport for interactive shell
sessions. It does not help here because the SSH tunnel carries structured
JSON-over-Unix-socket traffic, not a terminal session. The latency is on the
daemon side (subprocess spawning, serial processing), not in network transport.

## Decision

### Fix A: Daemon-side keystroke coalescing

Before executing a `SessionSendKeys` request, the daemon attempts to drain
additional queued requests from the buffered reader using non-blocking reads.
Consecutive literal send-keys requests targeting the same session are coalesced
into a single `tmux send-keys -l -t <session> <combined_text>`. Named keys
(Enter, Tab, Escape, etc.) flush any pending literal batch, execute, then
continue draining.

This reduces N subprocess spawns to 1 for bursts of literal characters (the
common case when typing). A fast typist producing 10 characters between daemon
processing cycles gets one tmux call instead of 10.

**Implementation:**

1. After reading a `SessionSendKeys` request in the connection loop, set the
   stream to non-blocking temporarily.
2. Read ahead all available lines, parsing each as a `DaemonRequest`.
3. Partition into a coalesced batch: accumulate consecutive
   `SessionSendKeys` requests whose command is `["tmux", "send-keys", "-l",
   "-t", session, text]` (literal sends to the same session). When a non-literal
   or different-session request is encountered, stop accumulating.
4. Execute the coalesced literal batch as one `tmux send-keys -l -t <session>
   <combined>` call.
5. Execute remaining non-coalesced requests normally.
6. Restore blocking mode on the stream before resuming the read loop.

### Fix B: Suppress responses for fire-and-forget sends

Add a `fire_and_forget: bool` field to `DaemonSessionSendKeysPayload`
(defaulting to `false` for backward compatibility). When `true`, the daemon
skips serializing and writing a response for that request. The persistent
pipeline sets this to `true`; the queued fallback path leaves it `false`.

This eliminates response accumulation entirely, preventing the backpressure
stall from Bottleneck 2.

**Alternative considered:** spawning a client-side drain thread to consume
responses. Rejected because it adds thread management complexity for no
benefit over simply not sending the responses.

## Consequences

- Daemon request processing is no longer strictly one-request-one-response for
  fire-and-forget sends. The protocol becomes partially unidirectional.
- Coalescing changes the observable behavior: instead of N `tmux send-keys`
  invocations for N characters, the remote tmux session receives them as a
  single literal string. This matches what `tmux send-keys -l` does with
  multi-character arguments, so behavior is identical.
- Error reporting for coalesced batches attributes the failure to the entire
  batch rather than a specific character. This is acceptable because
  `send-keys` failures are rare and already only logged daemon-side.
- The non-blocking read-ahead introduces a brief mode switch on the stream.
  If no additional data is available, it returns immediately with no penalty.
- Bottleneck 3 (capture poll latency) remains. It is the inherent cost of
  polling over SSH and is addressed separately by ADR-012 Option B (direct
  SSH PTY).
