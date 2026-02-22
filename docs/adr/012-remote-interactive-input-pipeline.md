# ADR-012: Remote Interactive Input Pipeline

**Status:** Accepted
**Date:** 2026-02-21

## Context

Remote interactive mode sends each keystroke as a separate daemon request over
an SSH-tunneled Unix socket. Each request creates a new socket connection, does
a full JSON request-response round-trip through SSH (~50-100ms), and blocks
subsequent keystrokes until the response arrives. This serial
connect-per-keystroke model causes severe input lag on remote workspaces.

## Decision

Implement a persistent daemon connection with pipelining for remote interactive
mode (Option A).

- When entering interactive mode for a remote workspace, open a single
  persistent `UnixStream` to the daemon socket and keep it alive for the
  duration of interactive mode.
- Pipeline keystroke requests over the persistent connection without waiting for
  responses. Batch consecutive literal sends into single `tmux send-keys -l`
  commands.
- Update the daemon to loop on a connection, handling multiple requests per
  stream instead of one-request-per-connection.
- Per-keystroke cost drops from full SSH connection round-trip to writing bytes
  to an already-open socket.

## Future: Option B (Direct SSH PTY)

If Option A proves insufficient, the next step is bypassing the daemon entirely
for interactive mode:

- Open `ssh -t remote 'tmux attach -t session'` when entering interactive mode.
- Forward keystrokes directly to PTY stdin.
- Stream output continuously from PTY stdout (eliminates polling).
- Latency reduces to raw network latency, same as a normal SSH session.
- Daemon remains for workspace management (create, delete, list) but exits the
  interactive hot path.

The preview pane already handles ANSI escape sequences, so rendering a PTY
output stream is feasible without a full terminal parser.

## Consequences

- Daemon protocol changes from stateless one-shot connections to supporting
  long-lived connections with multiple requests.
- Interactive send path no longer blocks on in-flight flag for remote sessions.
- Error visibility for individual send-keys commands is reduced (fire-and-forget
  writes), but errors are rare for this operation and logged on the daemon side.
- Connection lifecycle must handle daemon restarts and SSH tunnel drops
  gracefully.
