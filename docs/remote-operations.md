# Grove Remote Operations (SSH Tunnel + systemd --user)

This runbook documents the supported Phase 5 transport path:
- `groved` on a remote Linux host (Unix socket only)
- SSH tunnel from client machine to remote socket
- one Grove TUI session with mixed local + remote projects

## 0. Local run shortcuts (Makefile)

Use these from repo root:

```bash
make tui
make groved
make tui-daemon
make root
make tunnel-up REMOTE_HOST=<remote-host>
make tunnel-status REMOTE_HOST=<remote-host>
make tunnel-down REMOTE_HOST=<remote-host>
```

Meaning:
- `make tui`: primary mode, one TUI that can show both local and remote projects.
- `make groved`: runs daemon process (`groved`) on a Unix socket.
- `make tui-daemon`: optional, routes local-target lifecycle calls through a daemon socket.
- `make root`: prints root JSON command tree.
- `make tunnel-up`: opens background SSH tunnel for remote daemon socket.
- `make tunnel-status`: checks tunnel control socket.
- `make tunnel-down`: closes background SSH tunnel.

`tui-daemon` is not a separate "remote TUI". Remote projects are selected per project target/profile inside the same TUI.

Default socket path:

```bash
$(HOME)/.grove/groved.sock
```

Override socket:

```bash
make groved SOCKET=/tmp/groved.sock
make tui-daemon SOCKET=/tmp/groved.sock
```

Tunnel variables:

```bash
REMOTE_HOST=build.example.com              # required
REMOTE_USER=michael                        # defaults to local $USER
REMOTE_SOCKET=/home/michael/.grove/groved.sock
LOCAL_SOCKET=~/.grove/groved-michael-build.example.com.sock
```

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

Preferred: just use TUI `Connect` in Settings. It now auto-starts the SSH tunnel from saved `host` + `user`.

Recommended (Makefile wrapper):

```bash
make tunnel-up REMOTE_HOST=<remote-host> REMOTE_USER=<remote-user>
make tunnel-status REMOTE_HOST=<remote-host> REMOTE_USER=<remote-user>
```

This creates/uses local socket:

```bash
~/.grove/groved-<remote-user>-<remote-host>.sock
```

Stop tunnel:

```bash
make tunnel-down REMOTE_HOST=<remote-host> REMOTE_USER=<remote-user>
```

Manual SSH is still valid if you prefer:

```bash
ssh -fN \
  -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=30 \
  -o ServerAliveCountMax=3 \
  -L ~/.grove/groved-<remote-user>-<remote-host>.sock:/home/<remote-user>/.grove/groved.sock \
  <remote-user>@<remote-host>
```

## 3. Verify remote control path

CLI smoke:

```bash
grove workspace list --socket ~/.grove/groved-<remote-user>-<remote-host>.sock
```

Expected: JSON envelope response from remote daemon.

TUI verification:

```bash
make tui
```

Then in `Settings`, connect the remote profile and open projects/workspaces. Local and remote entries should coexist in one list.

Project list badges:
- `[L]` means project target is local.
- `[R:<profile>]` means project target is remote via that profile.

## 4. TUI profile fields

In `Settings` remote profile, use:
- `name`: profile name, e.g. `prod`
- `host`: SSH host
- `user`: SSH user
- `remote_socket_path`: optional local tunneled socket path. Blank auto-infers `~/.grove/groved-<remote-user>-<remote-host>.sock`
- `default_repo_path`: defaults to `/home/<user>/`, override if your repos live elsewhere

Then use `Connect` / `Disconnect` actions in Settings (connect starts tunnel, disconnect stops it).

## 4.1 Add remote projects in TUI

Remote projects are explicit, they are not auto-discovered from the remote host in this phase.

From the TUI:
1. Open `Projects` view.
2. Choose `Add`.
3. Set `RunsOn` to `remote host`.
4. Set `RemoteProfile` to your connected profile name.
5. Enter `Path` as the repo path on the remote host (for example `/home/michael/src/grove`).
6. Optional: set `Name`, if blank Grove derives one from repo directory (remote defaults to `(<profile>)` suffix).

Collision rules:
- Local and remote may use the same repo path string, this is allowed because identity is `target + path`.
- Two remote entries with same `profile + path` are rejected as duplicates.
- Project names are still unique in config, use a distinct name when local and remote repos share the same basename.

## 5. Troubleshooting

- Tunnel socket missing:
  - Check SSH command still running.
  - Confirm `~/.grove/groved-<remote-user>-<remote-host>.sock` exists locally.
- `REMOTE_UNAVAILABLE` in TUI:
  - Re-run `Test` or `Connect` in Settings.
  - Verify tunnel path matches profile `remote_socket_path`.
- Service not running on remote:
  - `systemctl --user status groved.service`
  - `journalctl --user -u groved.service -n 200 --no-pager`
