# Grove Remote Operations (SSH Tunnel + systemd --user)

This runbook documents the supported Phase 5 transport path:
- `groved` on a remote Linux host (Unix socket only)
- SSH tunnel from client machine to remote socket
- Grove CLI/TUI targeting tunneled socket

## 1. Remote host setup

Prerequisites:
- `groved` binary installed on remote host, in `PATH` or absolute path known.
- Remote user has access to repo paths and tmux environment needed by Grove.

Create user service at `~/.config/systemd/user/groved.service`:

```ini
[Unit]
Description=Grove daemon
After=default.target

[Service]
Type=simple
ExecStart=%h/.local/bin/groved --socket %h/.grove/groved.sock
Restart=on-failure
RestartSec=1

[Install]
WantedBy=default.target
```

Enable/start:

```bash
systemctl --user daemon-reload
systemctl --user enable --now groved.service
systemctl --user status groved.service
```

Optional for headless operation without active SSH login:

```bash
loginctl enable-linger "$USER"
```

## 2. Client tunnel setup

Create local socket dir:

```bash
mkdir -p ~/.grove
```

Start tunnel (foreground):

```bash
ssh -N \
  -L ~/.grove/groved-prod.sock:/home/<remote-user>/.grove/groved.sock \
  <remote-user>@<remote-host>
```

Background tunnel with keepalives:

```bash
ssh -fN \
  -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=30 \
  -o ServerAliveCountMax=3 \
  -L ~/.grove/groved-prod.sock:/home/<remote-user>/.grove/groved.sock \
  <remote-user>@<remote-host>
```

## 3. Verify remote control path

CLI smoke:

```bash
grove workspace list --socket ~/.grove/groved-prod.sock
```

Expected: JSON envelope response from remote daemon.

## 4. TUI profile fields

In `Settings` remote profile, use:
- `name`: profile name, e.g. `prod`
- `host`: SSH host
- `user`: SSH user
- `remote_socket_path`: local tunneled socket path, e.g. `~/.grove/groved-prod.sock`
- `default_repo_path` (optional): default remote repo root

Then use `Test`, `Connect`, `Disconnect` actions in Settings.

## 5. Troubleshooting

- Tunnel socket missing:
  - Check SSH command still running.
  - Confirm `~/.grove/groved-prod.sock` exists locally.
- `REMOTE_UNAVAILABLE` in TUI:
  - Re-run `Test` or `Connect` in Settings.
  - Verify tunnel path matches profile `remote_socket_path`.
- Service not running on remote:
  - `systemctl --user status groved.service`
  - `journalctl --user -u groved.service -n 200 --no-pager`
