# Grove PRD

A minimal workspace manager for AI coding agents. Rust + FrankenTUI.

## Problem

Working with AI coding agents across multiple tasks requires context
isolation. Each task needs its own git worktree and agent session, but
managing these manually (creating worktrees, launching tmux sessions,
tracking what's running where) is tedious.

Sidecar solves this comprehensively but carries significant complexity
(plugin architecture, 8 agent types, kanban views, merge wizards, task
linking). Grove strips this to the core workflow: create a workspace,
assign an agent, interact with it inline, tear it down when done.

## Goals

1. Manage git worktree-backed workspaces from a TUI
2. Launch and monitor Claude Code or Codex in each workspace via tmux
3. Interactive mode: type directly into agent sessions from within the TUI
4. Real-time output preview with full ANSI rendering and cursor overlay
5. Full mouse support (click, drag-select, pane resize)
6. Keep the codebase small and maintainable

## Non-Goals

- Plugin architecture
- Kanban/board views
- Task/issue tracker integration
- PR creation or merge workflows
- Agent types beyond Claude Code and Codex
- Shell sessions (non-agent workspaces)
- Prompt templates or reusable prompts
- Git stats display (+/- lines, commits ahead/behind) (defer to v2)
- Diff view (defer to v2)
- Multi-repo support

## Delegation Contract

- This document is the source of truth for delegated work.
- References to sidecar are rationale only, not an external dependency.
- If this document and sidecar behavior differ, this document wins.
- Rust snippets are illustrative unless explicitly labeled as required shell
  commands or acceptance criteria.

## Decisions

Decisions made during spec review, with rationale.

- **Single repo scope**: Grove runs from a repo root and manages that
  repo's worktrees. No multi-repo support.
- **Alt-screen mode**: full-screen TUI (matches sidecar, standard for TUI
  apps).
- **Branch naming**: no prefix for new branches. New workspace creation
  uses workspace name as branch name.
- **Name validation**: workspace name (UI label + directory slug) allows
  alphanumeric characters, hyphens, and underscores. No spaces.
- **Existing branch attach naming**: attaching to an existing branch uses
  that branch verbatim (may include `/`, `.`). In this mode, workspace
  name and branch may differ.
- **Directory naming**: global, repo-prefixed. Workspace `auth-flow` in repo
  `myapp` creates `~/.grove/workspaces/myapp-<repo_hash>/myapp-auth-flow/`.
  The hashed repo bucket disambiguates repos with the same basename.
- **Main worktree visible**: the main worktree (repo itself) is shown in
  the workspace list but is not deletable. Useful to run an agent against
  the main branch.
- **Enter on idle workspace**: tries interactive mode, falls back to
  loading preview content if no agent is running. User must press `s` to
  start an agent first (matches sidecar).
- **Orphaned worktree recovery**: if Enter is pressed on an orphaned
  worktree (had an agent, tmux session gone), auto-restart the agent with
  the original agent type (matches sidecar).
- **Quit behavior**: confirm dialog ("Quit Grove?"), then exit. Running
  agents are left alive in their tmux sessions. Reattach on next launch
  (matches sidecar).
- **Delete behavior**: two-stage removal. First attempt without `--force`,
  if that fails (unmerged commits), automatically retry with `--force`.
  User has already confirmed via the delete dialog (matches sidecar).
- **Delete dialog**: modal showing workspace name, branch, path, warning
  about uncommitted changes, and an optional "Delete local branch"
  checkbox (off by default). Matches sidecar's delete dialog.
- **Gitignore management**: auto-add Grove marker files to `.gitignore`
  on worktree creation. Entries: `.grove/agent`, `.grove/base`,
  `.grove/start.sh`, `.grove/setup.sh` (matches sidecar's approach with
  `.sidecar-*` files).
- **Status bar**: context-dependent keybinding hints in list/preview mode,
  `-- INSERT --` in interactive mode. Hints change based on selected
  workspace status (e.g., show `[y]approve` when agent is waiting).
  Flash messages replace hints temporarily for errors/confirmations
  (auto-dismiss after 3 seconds).
- **No tmux attach**: no direct tmux attach feature (`t` key or `Ctrl+]`).
  Interactive mode is the only way to interact with agents. Simplifies
  the mental model.
- **Existing branch support**: creating a workspace from a branch that
  already exists attaches to that branch instead of creating a new one.
  Useful for resuming work on an existing feature branch.
- **Start on running agent**: pressing `s` when an agent is already
  running shows a flash message "Agent already running". User must Stop
  first, then Start.
- **Workspace sort order**: most recently active first (based on last
  output timestamp). Main worktree pinned at top.
- **No notifications/toasts**: errors and confirmations shown as flash
  messages in the status bar. No overlay toast system.
- **Setup script**: `.grove/setup.sh` in the repo root runs on workspace
  creation (before agent launch). Runs once, not on restart or orphan
  recovery. Also auto-copies `.env` files from main worktree (matches
  sidecar).
- **Full ANSI rendering**: preserve all ANSI escape sequences (colors,
  bold, underline, cursor movement) in both preview and interactive mode.
  Capture with `-e` flag.
- **Auto-scroll**: preview pane auto-scrolls to follow new output when
  user is at the bottom. Scrolling up pauses auto-scroll. Scrolling back
  to bottom (or pressing `G`) resumes it (matches sidecar).
- **Pane resize**: draggable divider between list and preview panes.
  Mouse drag to resize. Persist ratio across sessions.
- **Input guards**: when a modal dialog is open, all non-dialog keys are
  blocked. During interactive mode, only exit keys and copy/paste work.
- **Waiting state UX**: workspaces in Waiting status get a highlighted
  row (amber/yellow) in the list and a distinct status icon to draw
  attention.
- **Multi-key sequences**: implement `gg` (go to top) with 500ms timeout
  only if FrankenTUI supports key buffering without significant effort.
  Otherwise, use single-key alternatives.

## Architecture

### Tech Stack

- **Language**: Rust
- **TUI**: FrankenTUI (ftui crate, Elm architecture), alt-screen mode
- **Process management**: tmux (sessions per workspace, survive restarts)
- **VCS**: git worktrees for isolation

### Data Model

```
Workspace {
    name: String,           // workspace label + directory slug (safe chars only)
    path: PathBuf,          // absolute path to worktree directory
    branch: String,         // git branch name (may differ from `name` when attaching existing branch)
    base_branch: String,    // branch workspace was created from
    agent_type: AgentType,  // Claude, Codex
    status: Status,         // derived from agent state
    created_at: Instant,
    last_output_at: Option<Instant>,  // for sort order (most recently active first)
    is_main: bool,          // true for the primary repo worktree (not deletable)
    is_orphaned: bool,      // agent file exists but tmux session gone
}

AgentType: Claude | Codex

Status: Idle | Active | Thinking | Waiting | Done | Error

AgentSession {
    tmux_session: String,   // tmux session name: "grove-ws-{sanitized_workspace_name}"
    tmux_pane: String,      // pane ID (e.g., "%12"), globally unique and stable
    output_buffer: CircularBuffer<String>, // last 500 rendered lines (capture still reads 600)
    last_output_at: Option<Instant>,
    status: AgentStatus,
    waiting_for: Option<String>, // prompt text if waiting for approval
}

InteractiveState {
    active: bool,
    target_pane: String,       // tmux pane ID
    target_session: String,    // tmux session name
    last_key_time: Instant,    // for polling decay
    cursor_row: u16,           // 0-indexed, cached from poll
    cursor_col: u16,           // 0-indexed, cached from poll
    cursor_visible: bool,
    pane_height: u16,          // actual tmux pane dimensions
    pane_width: u16,
    bracketed_paste: bool,     // detected from agent output
    last_scroll_time: Instant, // scroll burst guard + snap-back guard
    scroll_burst_count: u32,
}
```

### Persistence

Following sidecar's approach: per-worktree marker files, project-local.

#### Per-Worktree Marker Files

Each worktree directory gets marker files at its root:

- `.grove/agent` -- agent type string (`"claude"`, `"codex"`)
- `.grove/base` -- base branch the worktree was created from

These are plain text, single-line files. Cheap to read, survive across
restarts, and don't require a central manifest for worktree metadata.

#### Setup Script

A `.grove/setup.sh` file in the main repo root is executed on workspace
creation, before the agent launches. This is for project-specific
environment setup (e.g., `direnv allow`, `nvm use`, custom tool init).

**Execution:**

1. Check if `.grove/setup.sh` exists in the main repo root
2. Run it in the new worktree directory: `bash /path/to/main/.grove/setup.sh`
3. Environment variables provided:
   - `MAIN_WORKTREE`: path to main repo
   - `WORKTREE_BRANCH`: branch name
   - `WORKTREE_PATH`: full path to the new worktree
4. Errors are logged as warnings but do not fail worktree creation

**Runs only on initial workspace creation.** Not on restart, orphan
recovery, or agent re-launch.

#### Env File Copying

On workspace creation, automatically copy `.env` files from the main
worktree to the new worktree:

- `.env`
- `.env.local`
- `.env.development`
- `.env.development.local`

Only copy files that exist. Skip silently if missing.

#### Environment Overrides

Apply default worktree-safe environment overrides for agent launch and
setup script execution:

- `GOWORK=off`
- clear `GOFLAGS`
- clear `NODE_OPTIONS`, `NODE_PATH`
- clear `PYTHONPATH`, `VIRTUAL_ENV`

Optionally load per-repo overrides from `.grove-env` (KEY=VALUE lines).
User overrides take precedence over defaults.

#### Gitignore Management

On worktree creation, ensure the repo's `.gitignore` contains entries
for Grove marker files. Check for missing entries and append them:

```
.grove/agent
.grove/base
.grove/start.sh
.grove/setup.sh
```

Only append entries that are not already present. Do not rewrite or
reorder existing `.gitignore` content.

#### Worktree Discovery

No central workspace list. On startup, discover workspaces by:

1. Run `git worktree list --porcelain` to get all worktrees
2. The main worktree is always included (marked `is_main = true`)
3. For each non-main worktree, check if `.grove/agent` exists
4. If it does, this is a Grove-managed workspace. Read agent type and base
   branch from marker files.
   - If agent marker is not `claude` or `codex`, mark workspace as unsupported
     and disable start/restart actions.
5. Cross-reference with live tmux sessions (prefix `grove-ws-`) to
   determine running/stopped status.

This means the source of truth is the filesystem (git worktrees + marker
files + tmux sessions), not a JSON manifest.

#### Worktree Directory Naming

Worktree directories are created under the global Grove workspaces root,
prefixed with the repo name, and grouped by a repo hash bucket:

```
~/.grove/workspaces/
  myapp-a1b2c3d4e5f6a7b8/ # repo bucket (name + stable hash of repo path)
    myapp-auth-flow/      # workspace "auth-flow"
    myapp-fix-tests/      # workspace "fix-tests"
```

#### Reconciliation on Startup

1. List all git worktrees via `git worktree list --porcelain`
2. Identify main worktree, add to list with `is_main = true`
3. Filter remaining to those with `.grove/agent` marker
4. List all tmux sessions matching `grove-ws-*`
5. Match sessions to worktrees by name
6. **Orphaned worktree** (`.grove/agent` exists, no tmux session): mark
   `is_orphaned = true`. Enter on this workspace restarts the agent.
7. **Orphaned session** (tmux exists, worktree directory gone): mark for
   cleanup
8. **Missing worktree** (git tracks it, directory gone): auto-prune via
   `git worktree prune`
9. **Current workdir deleted** (Grove running inside a removed worktree):
   detect missing cwd and recover by switching to the repo's main worktree
10. **Stopped workspace** (worktree exists, no tmux session, not orphaned):
   show as Idle

### Tmux Integration

Using sidecar-inspired patterns, fully specified here.

#### Session Lifecycle

Session names use a sanitized workspace name (`.`, `:`, `/` replaced by `-`)
to satisfy tmux session naming constraints.

**Create session:**
```bash
tmux new-session -d -s grove-ws-{name} -c {worktree_path}
tmux set-option -t grove-ws-{name} history-limit 10000
```

**Capture pane ID** (globally unique, stable across session):
```bash
tmux list-panes -t grove-ws-{name} -F "#{pane_id}"
```

**Output capture** (polling, used in both preview and interactive):
```bash
tmux capture-pane -p -e -J -S -600 -t grove-ws-{name}
```
- `-p`: print to stdout
- `-e`: preserve escape sequences (full ANSI)
- `-J`: join wrapped lines in normal preview capture
- `-S -600`: capture last 600 lines
- In interactive mode: omit `-J` flag to preserve tmux's native line
  wrapping (required for cursor alignment)
- For selected pane in "interactive input" preview path: also omit `-J`
  (matches sidecar direct-capture path so preview wrap == pane wrap)

**Batch capture** (when multiple sessions active):
```bash
for session in {sessions}; do
    echo "===GROVE_SESSION:$session==="
    tmux capture-pane -p -e -J -S -600 -t "$session" 2>/dev/null
done
```

Batch capture follows sidecar's coordinator model:
- Maintain an active-session registry (sessions polled in last 30s).
- On cache miss, capture only active sessions in one batch (singleflight).
- Cache results for 300ms, then fall back to direct single-session capture.
- When interactive-input preview path is active, batch capture omits `-J`.

**Cursor query** (interactive mode, captured atomically with output):
```bash
tmux display-message -t {pane_id} -p \
  "#{cursor_x},#{cursor_y},#{cursor_flag},#{pane_height},#{pane_width}"
```

**Send keys** (interactive mode):
```bash
# Named keys (Enter, Tab, C-c, etc.)
tmux send-keys -t grove-ws-{name} {key_name}

# Literal text (letters, digits, symbols)
tmux send-keys -l -t grove-ws-{name} '{text}'
```

**Resize pane** (on entering interactive, on terminal resize):
```bash
tmux set-option -t grove-ws-{name} window-size manual
tmux resize-pane -t {pane_id} -x {width} -y {height}
```

**Stop agent** (graceful then force):
```bash
tmux send-keys -t grove-ws-{name} C-c
# wait 2 seconds, if still running:
tmux kill-session -t grove-ws-{name}
```

**Kill session** (on workspace delete):
```bash
tmux kill-session -t grove-ws-{name}
```

**Reconnect on startup:**
```bash
tmux list-sessions -F "#{session_name}"
```
Filter for `grove-ws-*` prefix, match back to worktrees.

#### Polling Intervals

Adaptive intervals from sidecar. In interactive mode, polling decays
based on time since last keystroke.

**Preview mode:**

| State                       | Interval |
|-----------------------------|----------|
| Initial (just launched)     | 200ms    |
| Active (output changing)    | 200ms    |
| Thinking (agent reasoning)  | 200ms    |
| Idle (no recent changes)    | 2s       |
| Waiting (approval prompt)   | 2s       |
| Done (agent exited)         | 20s      |
| Runaway output throttled    | 20s      |
| Background (not selected)   | 10s      |
| Visible but unfocused       | 500ms    |

**Interactive mode:**

| State                              | Interval |
|------------------------------------|----------|
| Active typing (< 2s since key)    | 50ms     |
| Recent typing (2-10s since key)   | 200ms    |
| Idle (> 10s since key)            | 500ms    |

#### Output Change Detection

Hash-based, following sidecar.

- First hash raw capture + length. If unchanged, skip all processing.
- If changed, strip full and partial mouse escape fragments
  (`\x1b[<35;192;47M`, `\x1b[?1000h`, `[<64;10;5M`, etc.), then hash cleaned
  content and parse only when needed.
- Trim trailing newline before splitting into lines.

### Agent Launch

Following sidecar's launcher script approach for prompt support.

#### Without Prompt

```bash
tmux send-keys -t grove-ws-{name} '{agent_cmd}' Enter
```

Where `agent_cmd` is `claude` or `codex`.

#### With Prompt

Write a launcher script to `{worktree_path}/.grove/start.sh`:

```bash
#!/bin/bash
# Ensure tools are on PATH (nvm, homebrew, etc.)
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
[ -s "$NVM_DIR/nvm.sh" ] && source "$NVM_DIR/nvm.sh" 2>/dev/null
if ! command -v node &>/dev/null; then
  [ -f "$HOME/.zshrc" ] && source "$HOME/.zshrc" 2>/dev/null
  [ -f "$HOME/.bashrc" ] && source "$HOME/.bashrc" 2>/dev/null
fi

{agent_cmd} "$(cat <<'GROVE_PROMPT_EOF'
{prompt}
GROVE_PROMPT_EOF
)"
rm -f {launcher_path}
```

Then:
```bash
tmux send-keys -t grove-ws-{name} 'bash {launcher_path}' Enter
```

The script self-deletes after running.

#### Skip Permissions Flags

| Agent  | Flag                                          |
|--------|-----------------------------------------------|
| Claude | `--dangerously-skip-permissions`              |
| Codex  | `--dangerously-bypass-approvals-and-sandbox`  |

Optional, toggled in the new workspace dialog.
Guardrails for v1:
- Default off.
- Requires explicit per-workspace opt-in in the create dialog.
- Show an explicit "unsafe mode enabled" warning in the dialog before create.
- Never persist as a global default.

### Status Detection

Dual-signal approach from sidecar.

**Signal 1: Tmux output pattern matching** (check tail bytes):

- **Waiting patterns**: `[y/n]`, `(y/n)`, `allow edit`, `allow bash`,
  `approve`, `confirm`
- **Thinking patterns**: unclosed `<thinking>` tags, `thinking...`
- **Done patterns**: `task completed`, `finished`, `exited with code 0`
- **Error patterns**: `error:`, `failed`, `panic:`, `traceback`

Only check last 5 output lines for waiting patterns to avoid false
positives from scrollback.

**Signal 2: Agent session files** (checked every poll):

- **Claude**: `~/.claude/projects/{path-with-nonalnum-replaced-by-dashes}/agent-*.jsonl`
  - Fast path: if main session file mtime is within 30s, active
  - Also check subagent files under
    `{session-uuid}/subagents/*.jsonl` for recent activity
  - Iterate recent session candidates by mtime (skip abandoned files)
  - Fallback: parse JSONL tail to determine last user vs assistant message
- **Codex**: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
  - Match session by `cwd` in `session_meta` JSONL records
  - Cache matched session path and extracted `cwd` metadata
  - Fast path: if matched file mtime is within 30s, active
  - Fallback: parse JSONL tail for last speaker role

Important nuance: session-file checks run even when tmux output hash is
unchanged, so Active/Waiting can still update when pane text does not.

Resolution rule:
- Tmux output is authoritative for Thinking/Done/Error
- Session files are authoritative for Active/Waiting

### Status Icons

| Status  | Icon | Color        |
|---------|------|--------------|
| Active  | `●`  | Green        |
| Thinking | `◐`  | Blue/Cyan    |
| Waiting | `⧗`  | Yellow/Amber |
| Done    | `✓`  | Cyan         |
| Error   | `✗`  | Red          |
| Idle    | `○`  | Gray/Dim     |
| Main    | `◉`  | Default      |

Workspaces in Waiting status are highlighted with an amber/yellow
background on the entire row to draw attention.

## UI Layout

### Workspace List Item (two lines)

Each workspace in the sidebar list displays two lines:

```
Line 1: {icon} {name}                        {relative_time}
Line 2:    {agent_type}  {warnings}
```

**Line 1**: Status icon, workspace name (truncated with `...` if needed),
relative timestamp ("2m ago", "1h ago", "now").

**Line 2**: Agent type label ("Claude", "Codex"), orphan warning
(`session ended`), missing folder warning (`folder missing`).

Selected workspace uses bright background when list pane is focused,
dimmed background when preview pane is focused.

### Auto-Scroll Behavior

The preview pane follows new output by default:

- **Auto-scroll enabled**: when the user is at the bottom of output,
  new content automatically scrolls into view
- **Paused**: when the user scrolls up manually (j/k, scroll wheel,
  Ctrl+u), auto-scroll pauses and the viewport stays put
- **Resumed**: when the user scrolls back to the bottom (`G`, or
  `previewOffset` reaches 0), auto-scroll resumes
- **Reset on selection change**: selecting a different workspace resets
  scroll position and re-enables auto-scroll
- **Scroll burst protection**: debounce rapid wheel/trackpad bursts
  (short cooldown + burst mode) so offsets do not oscillate under
  high-frequency input
- **Snap-back guards**: when scrolled up, only snap back on real typing
  keys. Never snap back for Escape, multi-rune fragments, or inputs that
  look like leaked mouse CSI fragments (`[<...`).

```rust
fn scroll_preview(&mut self, delta: i32) -> Cmd<Msg> {
    let now = Instant::now();
    let since_last = now.saturating_duration_since(self.last_scroll_time);
    if since_last < Duration::from_millis(40) {
        self.scroll_burst_count += 1;
        let burst_debounce = if self.scroll_burst_count > 4 { 120 } else { 40 };
        if since_last < Duration::from_millis(burst_debounce) {
            return Cmd::None;
        }
    } else {
        self.scroll_burst_count = 1;
    }
    self.last_scroll_time = now;

    if delta < 0 {
        self.auto_scroll = false;
        self.preview_offset = self.preview_offset.saturating_add(delta.unsigned_abs() as usize);
    } else {
        self.preview_offset = self.preview_offset.saturating_sub(delta as usize);
        if self.preview_offset == 0 {
            self.auto_scroll = true;
        }
    }
    Cmd::None
}
```

## UI Modes

The TUI has three modes, following sidecar's model.

### Mode 1: List View (default, sidebar focused)

Two-pane layout. Left = workspace list, right = output preview.
j/k navigates the workspace list. Preview shows selected workspace's
agent output.

```
+---------------------------+------------------------------------------+
| Workspaces                |  Preview: auth-flow                      |
|                           |                                          |
| ◉ main              now  |  I'll start by looking at the existing   |
| ● auth-flow        2m ago|  auth module...                          |
|   Claude                  |                                          |
| ● db-migration     5m ago|  Reading src/auth/mod.rs                 |
|   Codex                   |  ...                                     |
| ○ fix-tests        1h ago|                                          |
|   Claude                  |                                          |
+---------------------------+------------------------------------------+
 [n]ew [D]elete [s]tart [S]top [Enter] interactive       [q]uit
```

### Mode 2: Preview Focus (preview pane focused)

Same two-pane layout, but j/k now scrolls the output preview instead
of navigating the workspace list. Entered by pressing `l`/`Right`/`Tab`
from list view. Press `h`/`Left`/`Esc` to return to list focus.

### Mode 3: Interactive (-- INSERT --)

Full agent interaction within the TUI. The preview pane becomes a live
terminal view of the agent's tmux session. Keystrokes are forwarded
directly to the tmux pane. A cursor overlay shows the agent's cursor
position.

```
+---------------------------+------------------------------------------+
| Workspaces                |                                          |
|                           |  I'll implement the OAuth flow. Let me   |
| ◉ main              now  |  read the current auth module first.     |
| ● auth-flow        2m ago|                                          |
|   Claude                  |  src/auth/mod.rs                         |
| ● db-migration     5m ago|  ...                                     |
|   Codex                   |                                          |
|                           |  Do you want me to proceed? [Y/n] █      |
|                           |                                          |
+---------------------------+------------------------------------------+
                                                       -- INSERT --
```

## Mouse Support

Full mouse support throughout the TUI.

Modal guard rule (match sidecar):
- When any modal is open, absorb background clicks, scroll, and drag.
- Do not allow pane focus changes, divider drag, or interactive entry
  from background regions until modal closes.

### Workspace List

- **Click workspace**: select it, reset scroll, load preview content
- **Click when preview focused**: switches focus back to list pane

### Preview Pane

- **Click preview pane**: focus the preview pane. If an agent is
  running, enter interactive mode and forward the click.
- **Scroll wheel**: scroll output up/down (pauses auto-scroll on
  scroll up)

### Pane Divider

- **Drag divider**: resize the split ratio between list and preview
  panes. Ratio persisted across sessions.

### Interactive Mode

- **Click-drag**: character-level text selection (see Copy/Paste)
- **Scroll wheel**: forwarded to tmux pane
- All other clicks forwarded to tmux only when mouse reporting is enabled by
  the target app (`ESC[?1000h` / `ESC[?1006h` seen in output)
- Filter leaked partial mouse fragments (`[<...`) from key stream before
  forwarding keys
- If user clicks outside preview while in interactive mode, exit
  interactive first, then process target-pane click

## Interactive Mode

The core feature. Modeled after sidecar's implementation.

### Entering Interactive Mode

1. Press `Enter` on a workspace with a running agent (or click the
   preview pane when an agent is running)
2. If workspace is orphaned (had agent, session gone): auto-restart agent
   with original agent type, then enter interactive
3. If no agent running: fall back to loading preview content (no-op for
   interactive, user must `s` first)
4. Resize tmux pane to match preview dimensions (critical for cursor
   alignment)
5. Verify resize succeeded (retry once if mismatch)
6. Initialize `InteractiveState`
7. Switch to interactive view mode
8. Trigger immediate poll for fresh output at new dimensions
9. Show `-- INSERT --` indicator in status line

### Keystroke Forwarding

All keystrokes (except exit keys) are forwarded to the tmux pane via
`tmux send-keys`.

**Key mapping** (FrankenTUI KeyEvent to tmux key name):

| FrankenTUI Key  | tmux send-keys       |
|-----------------|----------------------|
| `Enter`         | `Enter`              |
| `Tab`           | `Tab`                |
| `Backspace`     | `BSpace`             |
| `Delete`        | `DC`                 |
| `Up/Down/L/R`   | `Up`/`Down`/`Left`/`Right` |
| `Home`/`End`    | `Home`/`End`         |
| `PgUp`/`PgDn`   | `PPage`/`NPage`      |
| `Ctrl+A-Z`      | `C-a` through `C-z`  |
| `Escape`        | `Escape`             |
| `F1-F12`        | `F1` through `F12`   |
| Letters/digits  | `-l` flag (literal)  |

Keys are sent asynchronously in order to preserve sequencing. If
`tmux send-keys` returns an error containing "can't find pane" or
"no such session", exit interactive mode and show a flash message in
the status bar.

Sidecar-aligned guardrails:
- Track `last_scroll_time` and reject suspicious key fragments during and
  shortly after scroll bursts.
- Drop rune fragments matching mouse CSI shape (`[<`, `;`, trailing `M/m`).
- Suppress bare `[` when it arrives in escape-proximity or mouse-proximity
  windows (split SGR mouse sequence leak case).
- While preview is scrolled up, only snap back to bottom for real typing/
  editing keys, never for Escape or mouse-like fragments.

### Exiting Interactive Mode

**Ctrl+\\**:

Immediate exit back to list view. Agent keeps running in tmux.

**Double-Escape** (150ms window):

Two Escape presses within 150ms exits interactive mode. A single Escape
is forwarded to the agent as normal. The 150ms window distinguishes
intentional exit from Escape key usage within the agent.

### Cursor Overlay

The agent's cursor position is rendered as a block character overlay on
the captured output. Following sidecar's approach:

1. **Capture**: cursor position queried atomically with output in the
   poll task via `tmux display-message`. Never queried during
   view rendering (no blocking in `view()`).

2. **Adjust**: cursor row adjusted for display height vs pane height
   mismatch:
   ```
   relative_row = cursor_row - (pane_height - display_height)  // if pane taller
   relative_row = cursor_row + (display_height - pane_height)  // if pane shorter
   ```
   Clamped to visible area.

3. **Render**: ANSI-aware string slicing at cursor position. Replace
   character under cursor with reverse-video styled block. If cursor is
   past end of line, pad with spaces and append block.

### Pane Resize Synchronization

The tmux pane must match the preview area dimensions for cursor alignment
and line wrapping to be correct.

- **On enter**: resize pane to preview dimensions before first poll
- **On terminal resize**: immediate reflow of TUI layout, then resize
  tmux pane to match new preview dimensions. Immediate poll for fresh
  content at new width.
- **During poll**: verify pane matches expected dimensions, resize if
  drifted
- **Capture flag**: omit `-J` in interactive mode to preserve tmux's
  native wrapping at pane width
- **Direct preview capture path**: when interactive-input preview mode is
  enabled for the selected pane, also omit `-J` outside interactive mode
  so preview wrapping matches live pane width

### Paste Handling

Detect paste events (multi-line input or > 10 characters in single
event). If the agent has bracketed paste mode enabled (detected from
output escape sequences), wrap pasted text with `ESC[200~` ... `ESC[201~`
before sending via tmux.

### Copy/Paste

Following sidecar's interactive copy/paste model.

#### Text Selection (mouse drag)

Click-and-drag selects text at character granularity:

1. Mouse click stores an anchor position (line index, visual column)
2. Drag motion activates selection, extending from anchor to current
   position
3. Selection is normalized so start is always before end regardless of
   drag direction
4. Visual columns account for tab expansion and multi-width characters
   (emoji snap to character boundaries)

Selected text is highlighted with a background color, injected per-line
during rendering via ANSI 24-bit background codes. Selection points are
absolute buffer indices (not viewport-relative), so they persist across
scrolling.

#### Copy (Alt+C)

1. If text is selected: extract selected lines from output buffer using
   visual column ranges. Strip ANSI escape codes.
2. If no selection: copy all currently visible lines in the preview area.
3. Write to system clipboard (via platform clipboard API).
4. Show flash message in status bar ("Copied N line(s)").
5. Clear selection.

#### Paste (Alt+V)

1. Read text from system clipboard.
2. Check if agent has bracketed paste mode enabled (detected from output
   escape sequences: `ESC[?2004h` = enabled, `ESC[?2004l` = disabled).
3. If bracketed paste enabled: wrap text in `ESC[200~` ... `ESC[201~`
   before sending.
4. If not: load into tmux buffer via `tmux load-buffer -`, then
   `tmux paste-buffer`.
5. If scrolled up, snap back to live output so user sees the paste.

### Poll Generation Tracking

Prevent duplicate polls from running in parallel.

- Track generation per workspace/session.
- Increment generation when entering interactive mode.
- Increment generation when scheduling debounced interactive polls.
- Increment generation when a workspace/session is stopped or deleted.
- Increment generation when session identity changes (restart, orphan recovery).
- Include generation in capture requests/results.
- Drop stale poll messages on receipt when generation mismatches.

## Key Bindings

Modeled after sidecar's workspace plugin bindings. Context-aware: bindings
change based on current mode. When a modal dialog is open, all non-dialog
keys are blocked.

### Global

| Key          | Action                   |
|--------------|--------------------------|
| `q`          | Quit Grove (confirm dialog) |
| `Esc`        | Back / cancel            |

### Workspace List (left pane focused)

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / `Down` / `Ctrl+n` | Select next workspace |
| `k` / `Up` / `Ctrl+p`   | Select previous workspace |
| `g g`        | Select first workspace (multi-key, 500ms timeout) |
| `G`          | Select last workspace    |
| `Enter`      | Enter interactive mode (or load preview if no agent) |
| `n`          | New workspace (open dialog)     |
| `D`          | Delete selected workspace       |
| `s`          | Start agent (no-op with flash message if already running) |
| `S`          | Stop agent                      |
| `y`          | Approve agent prompt            |
| `N`          | Reject agent prompt             |
| `r`          | Refresh workspace list          |
| `l` / `Right`| Focus right (preview) pane     |
| `Tab`        | Switch pane                     |
| `Shift+Tab`  | Switch pane                     |
| `\`          | Toggle sidebar visibility       |

### Output Preview (right pane focused)

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / `Down` | Scroll down                    |
| `k` / `Up`   | Scroll up                      |
| `Ctrl+d`     | Page down                       |
| `Ctrl+u`     | Page up                         |
| `G`          | Jump to bottom (resume auto-scroll) |
| `g g`        | Jump to top                     |
| `s`          | Start agent                     |
| `S`          | Stop agent                      |
| `y`          | Approve agent prompt            |
| `N`          | Reject agent prompt             |
| `h` / `Left` / `Esc` | Focus left (list) pane |
| `Tab`        | Switch pane                     |
| `Shift+Tab`  | Switch pane                     |
| `\`          | Toggle sidebar visibility       |

### Interactive Mode (-- INSERT --)

All keys forwarded to tmux except:

| Key              | Action                     |
|------------------|----------------------------|
| `Ctrl+\`         | Exit interactive mode       |
| `Double-Escape`  | Exit interactive mode (150ms window) |
| `Alt+C`          | Copy selection (or visible region) to system clipboard |
| `Alt+V`          | Paste system clipboard into tmux pane |
| Mouse drag       | Select text (character-level) |

### Multi-Key Sequences

500ms timeout for multi-key sequences. Registry checks for pending
partial matches before dispatching single-key actions.

Currently only `g g` (go to top) uses this. Implement only if
FrankenTUI's input model supports key buffering without significant
effort. Otherwise, drop `gg` and use single-key alternatives.

### New Workspace Dialog

Modal overlay with fields:

1. **Name**: text input (validated: alphanumeric + hyphens + underscores,
   no spaces). Used for workspace label and directory name.
2. **Existing branch** (optional): git branch selector/text input. If set,
   Grove attaches to this branch verbatim and ignores base branch.
3. **Agent**: toggle between Claude / Codex
4. **Base branch**: defaults to the main worktree's current branch,
   editable. Used only when creating a new branch.
5. **Prompt** (optional): free-text initial task description passed to
   agent
6. **Skip permissions**: checkbox (off by default, requires explicit warning
   acknowledgement)

| Key          | Action                          |
|--------------|---------------------------------|
| `Tab`        | Next field                      |
| `Shift+Tab`  | Previous field                  |
| `Ctrl+s` / `Ctrl+Enter` | Confirm and create  |
| `Esc`        | Cancel                          |

**On confirm:**

1. If **Existing branch** is set: create worktree from that branch
   (no new branch created)
2. Otherwise: create new branch from base branch and workspace name
3. Write marker files (`.grove/agent`, `.grove/base`)
4. Update `.gitignore`
5. Copy `.env` files from main worktree
6. Run `.grove/setup.sh` if it exists
7. Create tmux session
8. Launch agent with optional prompt

### Quit Flow

1. Press `q`
2. Confirm dialog ("Quit Grove?")
3. On confirm: issue `Cmd::SaveState`, exit TUI, runtime stops subscriptions.
   All tmux sessions left running.
4. On cancel: return to list view.

### Delete Workspace Flow

1. Confirm dialog with:
   - Title: "Delete Worktree?"
   - Workspace name, branch, path
   - Warning: "This will remove the working directory. Uncommitted
     changes will be lost."
   - Checkbox: "Delete local branch" (off by default)
   - Delete / Cancel buttons
2. Kill tmux session (if running)
3. Run `git worktree remove {path}`. If that fails (unmerged commits),
   retry with `--force`.
4. If "Delete local branch" checked:
   - Detect default branch via `git symbolic-ref refs/remotes/origin/HEAD`
     (fallback: `main`, `master`)
   - Refuse deletion if `{branch}` is the detected default branch
   - Otherwise run `git branch -d {branch}` then fallback to `git branch -D {branch}`
5. Marker files are deleted with the worktree directory.
6. Main worktree cannot be deleted (keybinding is a no-op).

## Technical Implementation

### Dependencies

```toml
[dependencies]
# FrankenTUI source must be reproducible for all contributors.
# Choose one strategy before implementation:
# 1) vendored `vendor/frankentui/...` workspace members committed in-repo, or
# 2) git dependencies pinned to an exact commit SHA.
# Developer-local relative paths (e.g. ../frankentui/...) are not allowed.
```

FrankenTUI provides the full stack: terminal session management (RAII
alt-screen, raw mode, mouse capture), Elm MVU runtime, diff-based rendering,
layout solver, hit testing, and subscriptions.

Additional crates (from crates.io):

```toml
clap = { version = "4", features = ["derive"] }  # CLI args
dirs = "6"                                         # XDG paths for state
serde = { version = "1", features = ["derive"] }   # state serialization
serde_json = "1"                                   # state file format
regex = "1"                                        # mouse escape stripping
lazy_static = "1"                                  # compiled regex statics
wait-timeout = "0.2"                               # tmux command timeouts
```

No need for crossterm directly: FrankenTUI wraps it. No need for
tokio/async-std: FrankenTUI's subscription system handles background work
with threads and channels.

### FrankenTUI Application Model

The core Elm pattern. Grove's entire state lives in a single `App` struct
that implements `ftui_runtime::Model`.

```rust
use ftui_core::event::{Event, KeyEvent, MouseEvent, PasteEvent};
use ftui_render::frame::Frame;
use ftui_runtime::{Cmd, Model, Subscription};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ── Messages ──────────────────────────────────────────────────────

enum Msg {
    // Input events (converted from ftui Event)
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(PasteEvent),
    Resize { width: u16, height: u16 },

    // Polling results
    CaptureResult {
        workspace_name: String,
        output: String,
        cursor: Option<CursorInfo>,
        generation: u64,
    },
    CaptureUnchanged {
        workspace_name: String,
        status: Status,
        waiting_for: Option<String>,
        cursor: Option<CursorInfo>,
        generation: u64,
    },
    PollTick,
    BatchCaptureResult(Vec<CaptureBatchItem>),

    // Workspace operations (async results)
    WorktreeCreated(Result<Workspace, String>),
    WorktreeDeleted(Result<String, String>),
    AgentLaunched { workspace_name: String, pane_id: String },
    AgentStopped { workspace_name: String },
    SetupScriptDone { workspace_name: String, result: Result<(), String> },

    // UI events
    FlashExpired,
    EscapeTimeout,
    MultiKeyTimeout,
    Noop,
}

struct CaptureBatchItem {
    workspace_name: String,
    output: String,
    cursor: Option<CursorInfo>,
    generation: u64,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(k) => Msg::Key(k),
            Event::Mouse(m) => Msg::Mouse(m),
            Event::Paste(p) => Msg::Paste(p),
            Event::Resize { width, height } => Msg::Resize { width, height },
            Event::Tick => Msg::Noop,
            Event::Focus(_) => Msg::Noop,
            Event::Clipboard(_) => Msg::Noop,
        }
    }
}

// ── View Mode ─────────────────────────────────────────────────────

enum ViewMode {
    List,          // sidebar focused, j/k navigates workspaces
    Preview,       // preview focused, j/k scrolls output
    Interactive,   // keystrokes forwarded to tmux
}

// ── Modal State ───────────────────────────────────────────────────

enum ActiveModal {
    None,
    NewWorkspace(NewWorkspaceState),
    ConfirmDelete(ConfirmDeleteState),
    ConfirmQuit,
}

// ── App Model ─────────────────────────────────────────────────────

struct App {
    // Core state
    workspaces: Vec<Workspace>,
    selected_index: usize,
    view_mode: ViewMode,
    modal: ActiveModal,
    repo_root: PathBuf,
    repo_name: String,

    // Interactive mode
    interactive: InteractiveState,

    // Preview
    preview_offset: usize, // scroll offset (0 = bottom)
    auto_scroll: bool,

    // Layout
    width: u16,
    height: u16,
    sidebar_width_pct: u16, // 20-60, persisted
    sidebar_hidden: bool,
    dragging_divider: bool,

    // Polling generation, per workspace/session for stale timer invalidation
    poll_generation: HashMap<String, u64>,
    next_poll_at: HashMap<String, Instant>,
    recently_polled_sessions: HashMap<String, Instant>, // 30s TTL, batch-capture candidate set

    // Flash messages
    flash_message: Option<String>,
    flash_is_error: bool,
    flash_expiry: Option<Instant>,

    // Multi-key
    pending_key: Option<(char, Instant)>,

    // Mouse/scroll burst guards (interactive + preview snap-back)
    last_scroll_time: Instant,
    scroll_burst_count: u32,
}
```

#### Model Trait Implementation

```rust
impl Model for App {
    type Message = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        // Seed per-workspace deadlines; first due immediately.
        let now = Instant::now();
        for ws in self.workspaces.iter().filter(|ws| ws.has_agent()) {
            self.next_poll_at.insert(ws.name.clone(), now);
        }
        Cmd::None
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        // Clear expired flash messages
        self.clear_expired_flash();

        // Modal intercepts all input when active
        if !matches!(self.modal, ActiveModal::None) {
            return self.update_modal(msg);
        }

        // Interactive mode intercepts all input except exit keys
        if matches!(self.view_mode, ViewMode::Interactive) {
            return self.update_interactive(msg);
        }

        match msg {
            Msg::Key(key) => self.update_key(key),
            Msg::Mouse(mouse) => self.update_mouse(mouse),
            Msg::Paste(paste) => Cmd::None,
            Msg::Resize { width, height } => self.handle_resize(width, height),
            Msg::CaptureResult { .. } => self.handle_capture(msg),
            Msg::CaptureUnchanged { .. } => self.handle_capture_unchanged(msg),
            Msg::PollTick => self.handle_poll_tick(),
            Msg::BatchCaptureResult(results) => self.handle_batch_capture(results),
            Msg::FlashExpired => { self.flash_message = None; Cmd::None }
            Msg::EscapeTimeout => self.handle_escape_timeout(),
            Msg::MultiKeyTimeout => self.handle_multi_key_timeout(),
            Msg::Noop => Cmd::None,
            // ... workspace operation results
            _ => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame) {
        // ftui-runtime uses Frame::new(), so hit testing must be enabled explicitly.
        frame.enable_hit_testing();

        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());

        // Split: content area + status line
        let rows = Flex::vertical()
            .constraints([Constraint::Fill, Constraint::Fixed(1)])
            .split(area);

        // Render main content (two-pane or modal overlay)
        self.render_main(frame, rows[0]);

        // Render status line
        self.render_status_line(frame, rows[1]);

        // Render modal overlay (if active)
        if !matches!(self.modal, ActiveModal::None) {
            self.render_modal(frame, area);
        }
    }

    fn subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
        // Adaptive polling subscriptions based on workspace states
        // See "Subscription Architecture" section
        self.build_subscriptions()
    }
}
```

### Command Patterns

FrankenTUI commands are side effects returned from `update()`. Grove uses
these patterns:

```rust
// Fire-and-forget background task (tmux operations)
fn capture_pane(workspace_name: String, generation: u64) -> Cmd<Msg> {
    Cmd::Task(
        TaskSpec { weight: 1.0, estimate_ms: 10.0, name: Some("capture".into()) },
        Box::new(move || {
            let mode = capture_mode_for(&workspace_name); // interactive, direct-preview, normal
            let output = tmux::capture_pane_with_join(&workspace_name, 600, mode.join_wrapped);
            let cursor = if mode.capture_cursor {
                tmux::query_cursor(&mode.cursor_target).ok()
            } else {
                None
            };
            Msg::CaptureResult {
                workspace_name,
                output: output.unwrap_or_default(),
                cursor,
                generation,
            }
        }),
    )
}

// Batch path: singleflight coordinator + active-session registry.
fn capture_due_batch(due: Vec<DuePoll>) -> Cmd<Msg> {
    Cmd::Task(
        TaskSpec { weight: 1.0, estimate_ms: 20.0, name: Some("batch-capture".into()) },
        Box::new(move || {
            for item in &due {
                active_registry_mark(item.workspace.clone());
            }
            let active_sessions = active_registry_sessions(Duration::from_secs(30));
            let results = capture_coordinator_run_batch(active_sessions, Duration::from_millis(300));
            Msg::BatchCaptureResult(results.into_items(due))
        }),
    )
}

// Sequential: create worktree then launch agent
fn create_workspace(spec: WorkspaceSpec) -> Cmd<Msg> {
    Cmd::Task(
        TaskSpec::default(),
        Box::new(move || {
            let result = worktree::create(&spec)
                .and_then(|ws| setup::run(&ws).map(|_| ws))
                .and_then(|ws| tmux::create_session(&ws).map(|_| ws));
            Msg::WorktreeCreated(result)
        }),
    )
}

// Flash message with auto-dismiss
fn show_flash(message: String, is_error: bool) -> Cmd<Msg> {
    Cmd::Batch(vec![
        Cmd::Msg(Msg::FlashExpired), // clear any existing
        Cmd::Task(
            TaskSpec::default(),
            Box::new(move || {
                std::thread::sleep(Duration::from_secs(3));
                Msg::FlashExpired
            }),
        ),
    ])
}
```

### Subscription Architecture

Subscriptions are declared each frame in `subscriptions()`. FrankenTUI's
`SubscriptionManager` diffs active vs declared subscriptions, starting and
stopping as needed. Use a single subscription tick, then schedule per-workspace
poll deadlines in model state (sidecar-style timer map + generation guards).

```rust
fn build_subscriptions(&self) -> Vec<Box<dyn Subscription<Msg>>> {
    if !self.workspaces.iter().any(|ws| ws.has_agent()) {
        return vec![];
    }
    vec![Box::new(Every::new(Duration::from_millis(50), || Msg::PollTick))]
}

fn handle_poll_tick(&mut self) -> Cmd<Msg> {
    let now = Instant::now();
    let mut due: Vec<DuePoll> = vec![];

    for ws in self.workspaces.iter().filter(|ws| ws.has_agent()) {
        let key = ws.name.clone();
        let next_at = self.next_poll_at.get(&key).copied().unwrap_or(now);
        if next_at <= now {
            let gen = self.poll_generation.get(&key).copied().unwrap_or(0);
            due.push(DuePoll { workspace: key, generation: gen });
            self.next_poll_at.insert(
                ws.name.clone(),
                now + self.poll_interval_for(ws),
            );
        }
    }

    if due.is_empty() {
        return Cmd::None;
    }

    if due.len() > 1 {
        return capture_due_batch(due); // batch coordinator + active-session registry
    }
    let only = due.swap_remove(0);
    capture_workspace(only.workspace, only.generation)
}

fn poll_interval_for(&self, ws: &Workspace) -> Duration {
    let is_selected = self.selected_workspace().map(|s| s.name == ws.name).unwrap_or(false);
    if matches!(self.view_mode, ViewMode::Interactive) && is_selected {
        let since_key = self.interactive.last_key_time.elapsed();
        if since_key < Duration::from_secs(2) {
            return Duration::from_millis(50);
        }
        if since_key < Duration::from_secs(10) {
            return Duration::from_millis(200);
        }
        return Duration::from_millis(500);
    }
    match ws.status {
        Status::Active if is_selected => Duration::from_millis(200),
        Status::Thinking if is_selected => Duration::from_millis(200),
        Status::Waiting if is_selected => Duration::from_secs(2),
        Status::Active | Status::Thinking | Status::Waiting => Duration::from_secs(10),
        Status::Done | Status::Error => Duration::from_secs(20),
        Status::Idle => Duration::ZERO,
    }
}
```

Generation-aware scheduling stays mandatory:
- Include generation with every scheduled capture request.
- Drop results where generation != current generation for that workspace.
- Increment generation on enter-interactive, debounce key poll reschedule,
  stop, delete, and session replacement.

### ANSI Rendering Pipeline

The preview pane must render tmux capture-pane output, which contains raw
ANSI escape sequences (colors, bold, cursor movement). Two approaches:

**Approach 1: ANSI-to-Cell parser (chosen).** Parse ANSI sequences and
write directly into ftui's `Buffer` cells. This gives full fidelity and
integrates with ftui's diff system.

```rust
/// Parse ANSI-encoded terminal output and write into an ftui Buffer region.
/// Handles: SGR (colors, bold, underline, reverse), cursor movement, line wrapping.
fn render_ansi_output(
    buffer: &mut Buffer,
    area: Rect,
    lines: &[String],
    scroll_offset: usize,
) {
    let visible_lines = &lines[scroll_start..scroll_end];
    let mut style = Style::default();

    for (row_idx, line) in visible_lines.iter().enumerate() {
        let y = area.y + row_idx as u16;
        let mut x = area.x;

        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Parse escape sequence, update `style`
                parse_sgr(&mut chars, &mut style);
                continue;
            }
            if x < area.right() {
                buffer.set_cell(x, y, Cell::new(ch, style.fg, style.bg, style.attrs));
                x += char_width(ch);
            }
        }
    }
}
```

The parser handles SGR codes (38;2;r;g;b for 24-bit fg, 48;2;r;g;b for bg,
0 for reset, 1 for bold, etc.). Partial sequences at line boundaries are
discarded. This matches how sidecar processes ANSI output for rendering.

**Hash-based change detection** (from sidecar): before parsing, hash the
raw output string. If hash matches the previous capture, skip all parsing
and rendering. Strip mouse escape sequences (`\x1b[<...M`, `\x1b[?1000h`)
before hashing to avoid false change signals.

### Layout Computation

Two-pane layout using ftui's `Flex` constraint solver.

```rust
fn render_main(&self, frame: &mut Frame, area: Rect) {
    let sidebar_hidden = self.sidebar_hidden;

    if sidebar_hidden {
        // Full-width preview
        self.render_preview(frame, area);
        return;
    }

    // Compute pixel widths from percentage
    let sidebar_w = (area.width as u32 * self.sidebar_width_pct as u32 / 100) as u16;
    let divider_w = 1;

    let cols = Flex::horizontal()
        .constraints([
            Constraint::Fixed(sidebar_w),
            Constraint::Fixed(divider_w),
            Constraint::Fill,
        ])
        .split(area);

    // Register hit regions for mouse interaction.
    // register_hit() is a no-op unless frame.enable_hit_testing() was called.
    frame.register_hit(cols[0], hit::SIDEBAR, HitRegion::Content, 0);
    frame.register_hit(cols[1], hit::DIVIDER, HitRegion::Handle, 0);
    frame.register_hit(cols[2], hit::PREVIEW, HitRegion::Content, 0);

    self.render_sidebar(frame, cols[0]);
    self.render_divider(frame, cols[1]);
    self.render_preview(frame, cols[2]);
}
```

**Sidebar layout** (per workspace item, 2 lines each):

```rust
fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
    let content_area = area.inner(Margin { horizontal: 1, vertical: 1 });
    let item_height = 2u16;

    for (i, ws) in self.visible_workspaces().enumerate() {
        let y = content_area.y + (i as u16 * item_height);
        if y + item_height > content_area.bottom() {
            break;
        }

        let item_rect = Rect::new(content_area.x, y, content_area.width, item_height);

        // Register hit region for click-to-select
        frame.register_hit(item_rect, hit::WORKSPACE_ITEM, HitRegion::Content, i as u64);

        let is_selected = i == self.selected_index;
        self.render_workspace_item(frame, item_rect, ws, is_selected);
    }
}
```

### Widget Architecture

Grove does not use ftui's built-in `List` or `Table` widgets for the
workspace sidebar or preview pane. The rendering is custom: write directly
into the Frame's buffer for full control over ANSI pass-through, cursor
overlay, and two-line item layout.

**Stateful widgets used from ftui:**
- `TextInput` -- for modal dialog text fields (workspace name, base branch,
  prompt)
- `Block` -- for panel borders around sidebar and preview panes
- `Paragraph` -- for simple text in dialogs and status line

**Custom rendering** (direct buffer writes):
- Workspace list items (two-line layout with status icons)
- Preview pane (ANSI terminal output with scroll)
- Interactive mode (ANSI output + cursor overlay)
- Status line (context-dependent hints)

### Mouse Hit Region Architecture

FrankenTUI's `HitGrid` provides pixel-level hit testing. Regions are
registered during `view()` and tested during `update()` on mouse events.
Because ftui runtime builds frames with `Frame::new()`, call
`frame.enable_hit_testing()` at the start of `view()`.

```rust
// Hit region IDs (HitId wraps a u32, data payload remains u64)
mod hit {
    use ftui_render::frame::HitId;
    pub const SIDEBAR: HitId = HitId::new(1);
    pub const DIVIDER: HitId = HitId::new(2);
    pub const PREVIEW: HitId = HitId::new(3);
    pub const WORKSPACE_ITEM: HitId = HitId::new(4);
    pub const MODAL_OVERLAY: HitId = HitId::new(10);
    pub const MODAL_BUTTON: HitId = HitId::new(11);
    pub const MODAL_FIELD: HitId = HitId::new(12);
    pub const MODAL_CHECKBOX: HitId = HitId::new(13);
}

fn update_mouse(&mut self, mouse: MouseEvent) -> Cmd<Msg> {
    // last_frame_hits is refreshed after each render from frame.hit_test data.
    let hit = self.last_frame_hits.hit_test(mouse.x, mouse.y);

    if self.modal_is_open() && self.hit_is_background(hit) {
        return Cmd::None; // absorb background events while modal active
    }

    if matches!(self.view_mode, ViewMode::Interactive)
        && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        && !matches!(hit, Some((hit::PREVIEW, _, _)))
    {
        self.exit_interactive();
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            match hit {
                Some((hit::WORKSPACE_ITEM, _, data)) => {
                    self.selected_index = data as usize;
                    self.view_mode = ViewMode::List;
                    self.preview_offset = 0;
                    self.auto_scroll = true;
                    Cmd::None
                }
                Some((hit::DIVIDER, HitRegion::Handle, _)) => {
                    self.dragging_divider = true;
                    self.drag_start_x = mouse.x;
                    self.drag_start_pct = self.sidebar_width_pct;
                    Cmd::None
                }
                Some((hit::PREVIEW, _, _)) => {
                    if self.selected_has_running_agent() {
                        self.enter_interactive()
                    } else {
                        self.view_mode = ViewMode::Preview;
                        Cmd::None
                    }
                }
                Some((hit::SIDEBAR, _, _)) => {
                    self.view_mode = ViewMode::List;
                    Cmd::None
                }
                _ => Cmd::None,
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if self.modal_is_open() {
                return Cmd::None;
            }
            if self.dragging_divider {
                self.handle_divider_drag(mouse.x)
            } else if matches!(self.view_mode, ViewMode::Interactive) {
                self.handle_text_selection_drag(mouse.x, mouse.y)
            } else {
                Cmd::None
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if self.modal_is_open() {
                return Cmd::None;
            }
            if self.dragging_divider {
                self.dragging_divider = false;
                self.persist_sidebar_width()
            } else {
                Cmd::None
            }
        }
        MouseEventKind::ScrollUp => {
            if self.modal_is_open() && self.hit_is_background(hit) { Cmd::None } else { self.scroll_preview(-3) }
        }
        MouseEventKind::ScrollDown => {
            if self.modal_is_open() && self.hit_is_background(hit) { Cmd::None } else { self.scroll_preview(3) }
        }
        _ => Cmd::None,
    }
}
```

**Pane divider drag:**

```rust
fn handle_divider_drag(&mut self, mouse_x: u16) -> Cmd<Msg> {
    let dx = mouse_x as i32 - self.drag_start_x as i32;
    let pct_delta = (dx * 100) / self.width as i32;
    let new_pct = (self.drag_start_pct as i32 + pct_delta)
        .clamp(20, 60) as u16;
    self.sidebar_width_pct = new_pct;
    Cmd::None
}
```

### Interactive Mode State Machine

Interactive mode has a tight loop: forward keys to tmux, poll output,
render with cursor overlay. The state machine integrates with ftui's
update cycle.

```
┌──────────┐   Enter/Click    ┌──────────────────┐
│ List or  │ ──────────────── │ Entering          │
│ Preview  │                  │ (resize tmux pane │
└──────────┘                  │  + immediate poll)│
     ▲                        └────────┬─────────┘
     │                                 │
     │ Ctrl+\ or                       ▼
     │ Double-Escape          ┌──────────────────┐
     │◄─────────────────────  │ Interactive       │
     │                        │ (keys→tmux,       │
     │ Session death          │  poll→render)     │
     │◄─────────────────────  └──────────────────┘
```

**Entering interactive mode:**

```rust
fn enter_interactive(&mut self) -> Cmd<Msg> {
    let ws = &self.workspaces[self.selected_index];
    let ws_name = ws.name.clone();
    let session = ws.agent_session.as_ref();

    // Must have a running agent
    let session = match session {
        Some(s) => s,
        None => {
            if ws.is_orphaned {
                return self.restart_agent(ws);
            }
            return show_flash("No agent running. Press 's' to start.".into(), true);
        }
    };

    self.view_mode = ViewMode::Interactive;
    self.interactive = InteractiveState {
        active: true,
        target_pane: session.tmux_pane.clone(),
        target_session: session.tmux_session.clone(),
        last_key_time: Instant::now(),
        cursor_row: 0,
        cursor_col: 0,
        cursor_visible: false,
        pane_height: 0,
        pane_width: 0,
        bracketed_paste: false,
        escape_pending: false,
        poll_generation: 0,
        selection: None,
    };
    let gen = self.poll_generation.get(&ws_name).copied().unwrap_or(0) + 1;
    self.poll_generation.insert(ws_name.clone(), gen);
    self.interactive.poll_generation = gen;

    // Resize tmux pane to match preview area, then immediate poll
    let preview_rect = self.preview_rect();
    let pane = session.tmux_pane.clone();
    let session_name = session.tmux_session.clone();
    let gen = self.interactive.poll_generation;

    Cmd::Sequence(vec![
        set_window_size_manual(session_name.clone()),
        resize_tmux_pane(pane.clone(), preview_rect.width, preview_rect.height),
        verify_resize_or_retry(pane, preview_rect.width, preview_rect.height),
        capture_pane(session_name, gen),
    ])
}
```

**Key forwarding in interactive mode:**

```rust
fn update_interactive(&mut self, msg: Msg) -> Cmd<Msg> {
    match msg {
        Msg::Key(key) => {
            // Exit keys
            if is_ctrl_backslash(&key) {
                return self.exit_interactive();
            }
            if key.code == KeyCode::Escape {
                if self.interactive.escape_pending {
                    // Double-escape within 150ms: exit
                    self.interactive.escape_pending = false;
                    return self.exit_interactive();
                }
                self.interactive.escape_pending = true;
                return Cmd::Task(
                    TaskSpec::default(),
                    Box::new(|| {
                        std::thread::sleep(Duration::from_millis(150));
                        Msg::EscapeTimeout
                    }),
                );
            }

            // Sidecar-style input hygiene for scroll/mouse burst leakage.
            if self.in_post_scroll_filter_window() && key_looks_like_mouse_fragment(&key) {
                self.interactive.escape_pending = false;
                return Cmd::None;
            }
            if key_is_bare_left_bracket(&key) && self.in_escape_or_mouse_proximity_gate() {
                self.interactive.escape_pending = false;
                return Cmd::None;
            }

            // Copy/Paste
            if is_alt_c(&key) {
                return self.copy_selection();
            }
            if is_alt_v(&key) {
                return self.paste_clipboard();
            }

            if self.preview_offset > 0 && !self.should_snap_back_for_key(&key) {
                return Cmd::None;
            }
            if self.preview_offset > 0 && self.should_snap_back_for_key(&key) {
                self.preview_offset = 0;
                self.auto_scroll = true;
            }

            // Forward all other keys to tmux
            self.interactive.escape_pending = false;
            self.interactive.last_key_time = Instant::now();
            let tmux_key = map_key_to_tmux(&key);
            let session = self.interactive.target_session.clone();
            let ws_name = self.workspaces[self.selected_index].name.clone();
            let gen = self.poll_generation.get(&ws_name).copied().unwrap_or(0) + 1;
            self.poll_generation.insert(ws_name.clone(), gen);
            self.next_poll_at.insert(ws_name, Instant::now() + Duration::from_millis(20));

            Cmd::Sequence(vec![
                send_key_to_tmux(session.clone(), tmux_key),
                capture_pane(session, gen), // immediate refresh; tick loop handles next cadence
            ])
        }
        Msg::EscapeTimeout => {
            if self.interactive.escape_pending {
                // Single escape: forward to tmux
                self.interactive.escape_pending = false;
                let session = self.interactive.target_session.clone();
                send_key_to_tmux(session, "Escape".into())
            } else {
                Cmd::None
            }
        }
        Msg::CaptureResult { output, cursor, generation, .. } => {
            if generation != self.interactive.poll_generation {
                return Cmd::None; // stale, discard
            }
            self.handle_interactive_capture(output, cursor)
        }
        Msg::CaptureUnchanged { cursor, generation, .. } => {
            if generation != self.interactive.poll_generation {
                return Cmd::None; // stale, discard
            }
            self.update_interactive_cursor(cursor)
        }
        Msg::Mouse(mouse) => self.update_interactive_mouse(mouse),
        Msg::Paste(paste) => self.handle_paste(paste),
        Msg::Resize { width, height } => {
            self.width = width;
            self.height = height;
            let preview = self.preview_rect();
            let pane = self.interactive.target_pane.clone();
            let gen = self.interactive.poll_generation;
            let session = self.interactive.target_session.clone();
            Cmd::Sequence(vec![
                set_window_size_manual(session.clone()),
                resize_tmux_pane(pane.clone(), preview.width, preview.height),
                verify_resize_or_retry(pane, preview.width, preview.height),
                capture_pane(session, gen),
            ])
        }
        _ => Cmd::None,
    }
}
```

```rust
fn should_snap_back_for_key(&self, key: &KeyEvent) -> bool {
    if self.last_scroll_time.elapsed() < Duration::from_millis(120) {
        return false;
    }
    if key.code == KeyCode::Escape {
        return false;
    }
    if key_looks_like_mouse_fragment(key) || key_is_multi_rune_fragment(key) {
        return false;
    }
    true
}
```

### Cursor Overlay Rendering

In interactive mode, the agent's cursor is overlaid on the captured output.
Following sidecar's approach, cursor position is queried atomically with
output during the poll task (never during `view()`).

```rust
fn render_interactive_output(
    buffer: &mut Buffer,
    area: Rect,
    lines: &[String],
    cursor: &CursorInfo,
    interactive: &InteractiveState,
) {
    // 1. Render ANSI output normally
    render_ansi_output(buffer, area, lines, 0);

    // 2. Adjust cursor position for display vs pane height mismatch
    let display_height = area.height;
    let pane_height = interactive.pane_height;
    let relative_row = if pane_height > display_height {
        cursor.row as i32 - (pane_height as i32 - display_height as i32)
    } else {
        cursor.row as i32 + (display_height as i32 - pane_height as i32)
    };

    // 3. Clamp to visible area (sidecar behavior: keep cursor visible)
    let clamped_row = relative_row.clamp(0, display_height.saturating_sub(1) as i32);
    let clamped_col = (cursor.col as i32).clamp(0, area.width.saturating_sub(1) as i32) as u16;
    let y = area.y + clamped_row as u16;
    let x = area.x + clamped_col;

    // 4. Apply reverse-video cursor block
    if interactive.cursor_visible {
        let cell = buffer.get_cell(x, y);
        buffer.set_cell(x, y, Cell::new(
            cell.content_or(' '),
            cell.bg,   // swap fg/bg for reverse video
            cell.fg,
            cell.attrs | CellAttrs::REVERSE,
        ));
    }
}
```

### Modal Dialog Architecture

Modals render as overlays on top of the main content. In ftui, this means
writing into the Frame's buffer at a higher z-order (later writes overwrite
earlier ones). The modal also registers hit regions that take priority over
background regions.

```rust
fn render_modal(&self, frame: &mut Frame, area: Rect) {
    match &self.modal {
        ActiveModal::None => {}
        ActiveModal::NewWorkspace(state) => {
            render_new_workspace_dialog(frame, area, state);
        }
        ActiveModal::ConfirmDelete(state) => {
            render_confirm_delete_dialog(frame, area, state);
        }
        ActiveModal::ConfirmQuit => {
            render_confirm_quit_dialog(frame, area);
        }
    }
}

fn render_centered_dialog(
    frame: &mut Frame,
    area: Rect,
    width: u16,
    height: u16,
    title: &str,
    content_renderer: impl FnOnce(&mut Frame, Rect),
) {
    // 1. Semi-transparent overlay (dim background)
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let cell = frame.buffer.get_cell(x, y);
            frame.buffer.set_cell(x, y, cell.with_fg_dimmed());
        }
    }

    // 2. Register full-screen hit region to block clicks outside
    frame.register_hit(area, hit::MODAL_OVERLAY, HitRegion::Content, 0);

    // 3. Center the dialog box
    let dialog_x = area.x + (area.width.saturating_sub(width)) / 2;
    let dialog_y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_rect = Rect::new(dialog_x, dialog_y, width, height);

    // 4. Render border
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .render(dialog_rect, frame);

    // 5. Render content inside border
    let inner = dialog_rect.inner(Margin::uniform(1));
    content_renderer(frame, inner);
}
```

**New Workspace dialog** (the most complex modal):

```rust
struct NewWorkspaceState {
    name_input: TextInputState,
    base_branch_input: TextInputState,
    prompt_input: TextInputState,   // multi-line
    agent_type: AgentType,          // toggle: Claude | Codex
    skip_permissions: bool,
    focused_field: NewWorkspaceField,
    validation_error: Option<String>,
}

enum NewWorkspaceField {
    Name,
    AgentType,
    BaseBranch,
    Prompt,
    SkipPermissions,
    Confirm,
}
```

Tab/Shift+Tab cycles `focused_field`. Enter on `Confirm` (or Ctrl+s/
Ctrl+Enter from any field) validates and creates the workspace.

### Tmux Wrapper Module

Thin synchronous wrappers around `std::process::Command`. All tmux
operations are called from background tasks (via `Cmd::Task`) to avoid
blocking the UI thread.

```rust
// src/tmux.rs

pub fn create_session(name: &str, working_dir: &Path) -> Result<String, TmuxError> {
    run_tmux(&["new-session", "-d", "-s", name, "-c", &working_dir.display().to_string()])?;
    run_tmux(&["set-option", "-t", name, "history-limit", "10000"])?;

    // Capture pane ID
    let output = run_tmux(&["list-panes", "-t", name, "-F", "#{pane_id}"])?;
    Ok(output.trim().to_string())
}

const TMUX_CAPTURE_TIMEOUT: Duration = Duration::from_secs(2);
const TMUX_BATCH_CAPTURE_TIMEOUT: Duration = Duration::from_secs(3);
const TMUX_DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const CAPTURE_MAX_BYTES: usize = 2 * 1024 * 1024;
const CAPTURE_LINE_COUNT: u16 = 600;

pub fn capture_pane_with_join(target: &str, scrollback: u16, join_wrapped: bool) -> Result<String, TmuxError> {
    let start_line = format!("-{scrollback}");
    let mut args = vec!["capture-pane", "-p", "-e"];
    if join_wrapped {
        args.push("-J");
    }
    args.extend(["-S", start_line.as_str(), "-t", target]);

    run_tmux_with_timeout(&args, TMUX_CAPTURE_TIMEOUT)
        .map(|s| trim_captured_output_utf8_safe(&s, CAPTURE_MAX_BYTES))
}

pub fn capture_pane(target: &str, scrollback: u16) -> Result<String, TmuxError> {
    capture_pane_with_join(target, scrollback, true)
}

pub fn capture_pane_no_join(target: &str, scrollback: u16) -> Result<String, TmuxError> {
    capture_pane_with_join(target, scrollback, false)
}

pub fn capture_batch(sessions: &[String], join_wrapped: bool) -> Result<String, TmuxError> {
    let mut script = String::new();
    for session in sessions {
        let quoted = sh_quote(session);
        script.push_str("echo \"===GROVE_SESSION:");
        script.push_str(session);
        script.push_str("===\"\n");
        if join_wrapped {
            script.push_str("tmux capture-pane -p -e -J");
        } else {
            script.push_str("tmux capture-pane -p -e");
        }
        script.push_str(&format!(" -S -{} -t {} 2>/dev/null\n", CAPTURE_LINE_COUNT, quoted));
    }
    run_shell_with_timeout(&script, TMUX_BATCH_CAPTURE_TIMEOUT)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

pub fn query_cursor(pane_id: &str) -> Result<CursorInfo, TmuxError> {
    let output = run_tmux(&[
        "display-message", "-t", pane_id, "-p",
        "#{cursor_x},#{cursor_y},#{cursor_flag},#{pane_height},#{pane_width}",
    ])?;
    CursorInfo::parse(&output)
}

pub fn send_key(session: &str, key: &str) -> Result<(), TmuxError> {
    run_tmux(&["send-keys", "-t", session, key])
}

pub fn send_literal(session: &str, text: &str) -> Result<(), TmuxError> {
    run_tmux(&["send-keys", "-l", "-t", session, text])
}

pub fn resize_pane(pane_id: &str, width: u16, height: u16) -> Result<(), TmuxError> {
    run_tmux(&[
        "resize-pane", "-t", pane_id,
        "-x", &width.to_string(),
        "-y", &height.to_string(),
    ])
}

pub fn kill_session(session: &str) -> Result<(), TmuxError> {
    run_tmux(&["kill-session", "-t", session])
}

pub fn list_sessions() -> Result<Vec<String>, TmuxError> {
    let output = run_tmux(&["list-sessions", "-F", "#{session_name}"])?;
    Ok(output.lines().map(|l| l.to_string()).collect())
}

fn run_tmux(args: &[&str]) -> Result<String, TmuxError> {
    run_tmux_with_timeout(args, TMUX_DEFAULT_TIMEOUT)
}

fn run_tmux_with_timeout(args: &[&str], timeout: Duration) -> Result<String, TmuxError> {
    let mut cmd = std::process::Command::new("tmux");
    cmd.args(args);
    run_command_with_timeout(cmd, timeout)
}

fn run_shell_with_timeout(script: &str, timeout: Duration) -> Result<String, TmuxError> {
    let mut cmd = std::process::Command::new("bash");
    cmd.args(["-c", script]);
    run_command_with_timeout(cmd, timeout)
}

fn run_command_with_timeout(mut cmd: std::process::Command, timeout: Duration) -> Result<String, TmuxError> {
    use wait_timeout::ChildExt;
    use std::process::Stdio;

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| TmuxError::Exec(e.to_string()))?;

    match child.wait_timeout(timeout).map_err(|e| TmuxError::Exec(e.to_string()))? {
        Some(_) => {
            let output = child.wait_with_output().map_err(|e| TmuxError::Exec(e.to_string()))?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(TmuxError::Command(String::from_utf8_lossy(&output.stderr).trim().to_string()))
            }
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err(TmuxError::Timeout(timeout))
        }
    }
}

fn trim_captured_output_utf8_safe(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    // Keep tail, then advance to next UTF-8 boundary.
    let mut start = s.len() - max_bytes;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    let tail = &s[start..];
    match tail.find('\n') {
        Some(i) if i + 1 < tail.len() => tail[i + 1..].to_string(),
        _ => tail.to_string(),
    }
}

pub enum TmuxError {
    Exec(String),
    Command(String),
    Parse(String),
    Timeout(Duration),
}
```

### State Persistence

Use two layers:
- FrankenTUI runtime persistence (`PersistenceConfig`, `Cmd::SaveState`,
  `Cmd::RestoreState`) for widget-level UI state and periodic checkpoints.
- Grove app-state file at `~/.local/state/grove/<repo-hash>.json` for
  app-level layout preferences.

```rust
#[derive(Serialize, Deserialize, Default)]
struct PersistedState {
    sidebar_width_pct: Option<u16>,
    sidebar_hidden: Option<bool>,
}

fn state_path(repo_root: &Path) -> PathBuf {
    let hash = hash_path(repo_root); // stable hash of canonical path
    dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/state"))
        .join("grove")
        .join(format!("{hash}.json"))
}

fn program_config() -> ProgramConfig {
    ProgramConfig::fullscreen()
        .with_mouse_capture_policy(MouseCapturePolicy::Auto)
        .with_persistence(
            PersistenceConfig::with_registry(build_state_registry())
                .checkpoint_every(Duration::from_secs(30))
                .auto_load(true)
                .auto_save(true)
        )
}
```

Load app-state on startup (missing file = defaults). Save app-state on
sidebar resize drag end and sidebar toggle. Use `Cmd::SaveState` on explicit
layout changes and let runtime auto-save on clean exit.

### Output Change Detection

Hash-based, following sidecar. Avoids re-parsing identical output.

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

struct OutputBuffer {
    lines: Vec<String>,
    last_raw_hash: u64,
    last_clean_hash: u64,
    last_raw_len: usize,
    capacity: usize, // 500 rendered lines, capture fetches 600 for margin
}

impl OutputBuffer {
    fn update(&mut self, raw_output: &str) -> bool {
        let raw_hash = {
            let mut h = DefaultHasher::new();
            raw_output.hash(&mut h);
            h.finish()
        };
        if raw_hash == self.last_raw_hash && raw_output.len() == self.last_raw_len {
            return false; // skip expensive cleaning when raw content is identical
        }

        let cleaned = strip_mouse_escapes(raw_output);
        let clean_hash = {
            let mut h = DefaultHasher::new();
            cleaned.hash(&mut h);
            h.finish()
        };

        if clean_hash == self.last_clean_hash {
            self.last_raw_hash = raw_hash;
            self.last_raw_len = raw_output.len();
            return false; // no change
        }
        self.last_raw_hash = raw_hash;
        self.last_clean_hash = clean_hash;
        self.last_raw_len = raw_output.len();

        // trim trailing newline to avoid extra empty last line
        self.lines = cleaned.trim_end_matches('\n').lines().map(|l| l.to_string()).collect();
        if self.lines.len() > self.capacity {
            let excess = self.lines.len() - self.capacity;
            self.lines.drain(..excess);
        }
        true
    }
}

fn strip_mouse_escapes(s: &str) -> String {
    // Remove: \x1b[<...M, \x1b[<...m, \x1b[?1000h, \x1b[?1006h, etc.
    // Also remove partial mouse fragments like "[<64;10;5M" (lost ESC).
    // These change on every mouse move and cause false change detection.
    lazy_static::lazy_static! {
        static ref MOUSE_RE: Regex = Regex::new(
            r"\x1b\[<\d+;\d+;\d+[Mm]|\x1b\[\?(?:1000|1002|1003|1005|1006|1015|2004)[hl]|\[<\d+;\d+;\d+[Mm]?"
        ).unwrap();
    }
    MOUSE_RE.replace_all(s, "").to_string()
}
```

If `update()` returns `false`, still run session-file status detection and
cursor updates, then emit `Msg::CaptureUnchanged` instead of skipping the
poll entirely.

### Key-to-Tmux Mapping

Translation from ftui's `KeyEvent` to tmux `send-keys` format.

```rust
struct TmuxKey {
    key: String,
    literal: bool, // true = use -l flag
}

fn map_key_to_tmux(key: &KeyEvent) -> TmuxKey {
    // Modified arrows and Shift+Tab are sent as literal CSI sequences.
    if key.modifiers.contains(Modifiers::SHIFT) && key.code == KeyCode::Up {
        return TmuxKey { key: "\x1b[1;2A".into(), literal: true };
    }
    if key.modifiers.contains(Modifiers::SHIFT) && key.code == KeyCode::Down {
        return TmuxKey { key: "\x1b[1;2B".into(), literal: true };
    }
    if key.modifiers.contains(Modifiers::SHIFT) && key.code == KeyCode::Right {
        return TmuxKey { key: "\x1b[1;2C".into(), literal: true };
    }
    if key.modifiers.contains(Modifiers::SHIFT) && key.code == KeyCode::Left {
        return TmuxKey { key: "\x1b[1;2D".into(), literal: true };
    }
    if key.modifiers.contains(Modifiers::SHIFT) && key.code == KeyCode::Tab {
        return TmuxKey { key: "\x1b[Z".into(), literal: true };
    }

    // Ctrl+ combinations
    if key.modifiers.contains(Modifiers::CTRL) {
        if let KeyCode::Char(c) = key.code {
            return TmuxKey { key: format!("C-{c}"), literal: false };
        }
    }

    match key.code {
        KeyCode::Enter => TmuxKey { key: "Enter".into(), literal: false },
        KeyCode::Tab => TmuxKey { key: "Tab".into(), literal: false },
        KeyCode::Backspace => TmuxKey { key: "BSpace".into(), literal: false },
        KeyCode::Delete => TmuxKey { key: "DC".into(), literal: false },
        KeyCode::Up => TmuxKey { key: "Up".into(), literal: false },
        KeyCode::Down => TmuxKey { key: "Down".into(), literal: false },
        KeyCode::Left => TmuxKey { key: "Left".into(), literal: false },
        KeyCode::Right => TmuxKey { key: "Right".into(), literal: false },
        KeyCode::Home => TmuxKey { key: "Home".into(), literal: false },
        KeyCode::End => TmuxKey { key: "End".into(), literal: false },
        KeyCode::PageUp => TmuxKey { key: "PPage".into(), literal: false },
        KeyCode::PageDown => TmuxKey { key: "NPage".into(), literal: false },
        KeyCode::Escape => TmuxKey { key: "Escape".into(), literal: false },
        KeyCode::F(n) => TmuxKey { key: format!("F{n}"), literal: false },
        KeyCode::Char(c) => TmuxKey { key: c.to_string(), literal: true },
        _ => TmuxKey { key: String::new(), literal: true },
    }
}
```

### Status Detection Implementation

Run both detectors every poll, then combine:
- tmux detector decides Thinking/Done/Error (only when output changed)
- session files decide Active/Waiting (always run, even if output unchanged)
- unchanged captures still emit status/cursor updates (`CaptureUnchanged`)

```rust
fn detect_status_for_poll(
    ws: &Workspace,
    output_changed: bool,
    output_for_patterns: &str,
    previous_status: Status,
) -> (Status, Option<String>) {
    let mut status = previous_status;
    let mut waiting_for = None;

    if output_changed {
        status = detect_status_from_tmux(output_for_patterns);
        if status == Status::Waiting {
            waiting_for = extract_prompt(output_for_patterns);
        }
    }

    // Always check session files, output can stay unchanged while status flips.
    if matches!(status, Status::Active | Status::Waiting) {
        if let Some(session_status) = detect_status_from_session_files(ws) {
            status = session_status;
            if status == Status::Waiting && waiting_for.is_none() {
                waiting_for = Some("Waiting for input".into());
            }
        }
    }

    (status, waiting_for)
}

fn detect_status_from_session_files(ws: &Workspace) -> Option<Status> {
    match ws.agent_type {
        AgentType::Claude => detect_claude_session_status(&ws.path),
        AgentType::Codex => detect_codex_session_status(&ws.path),
    }
}

fn detect_claude_session_status(worktree_path: &Path) -> Option<Status> {
    // Claude project dir uses path chars normalized to dashes.
    let abs = worktree_path.canonicalize().ok()?;
    let project = claude_project_dir_name(&abs);
    let dir = dirs::home_dir()?.join(".claude/projects").join(project);
    let sessions = recent_files_by_mtime(&dir, "agent-*.jsonl");

    for session in sessions {
        if file_mtime_within(&session, Duration::from_secs(30)) {
            return Some(Status::Active);
        }
        if any_subagent_mtime_within(&dir, &session, Duration::from_secs(30)) {
            return Some(Status::Active);
        }
        if let Some(status) = last_user_or_assistant_from_jsonl_tail(&session) {
            return Some(status);
        }
    }
    None
}

fn detect_codex_session_status(worktree_path: &Path) -> Option<Status> {
    let abs = worktree_path.canonicalize().ok()?;
    let root = dirs::home_dir()?.join(".codex/sessions");
    let session = find_codex_session_by_cwd_cached(&root, &abs)?; // caches path + session_meta cwd

    if file_mtime_within(&session, Duration::from_secs(30)) {
        return Some(Status::Active);
    }
    last_codex_role_from_jsonl_tail(&session)
}
```

### Worktree Operations

Synchronous wrappers around git commands. Called from `Cmd::Task`.

```rust
// src/worktree.rs

pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeEntry>, GitError> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .output()?;

    let mut entries = parse_porcelain(&String::from_utf8_lossy(&output.stdout))?;

    // If a tracked worktree directory is gone and its branch no longer exists,
    // prune stale git metadata and omit it from the UI list.
    let mut needs_prune = false;
    entries.retain(|e| {
        if e.is_main {
            return true;
        }
        if !e.path.exists() && !local_branch_exists(repo_root, &e.branch) {
            needs_prune = true;
            return false;
        }
        true
    });
    if needs_prune {
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(repo_root)
            .output();
    }

    Ok(entries)
}

pub fn create_worktree(
    repo_root: &Path,
    path: &Path,
    branch: &str,
    base: &str,
    existing_branch: bool,
) -> Result<(), GitError> {
    let args = if existing_branch {
        vec!["worktree", "add", &path.display().to_string(), branch]
    } else {
        vec!["worktree", "add", "-b", branch, &path.display().to_string(), base]
    };

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(GitError::Command(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

pub fn remove_worktree(repo_root: &Path, path: &Path, force: bool) -> Result<(), GitError> {
    let mut args = vec!["worktree", "remove", &path.display().to_string()];
    if force {
        args.push("--force");
    }

    let output = Command::new("git").args(&args).current_dir(repo_root).output()?;

    if output.status.success() {
        Ok(())
    } else if !force {
        // Retry with force (unmerged commits)
        remove_worktree(repo_root, path, true)
    } else {
        Err(GitError::Command(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

pub fn default_branch(repo_root: &Path) -> Option<String> {
    // Prefer remote HEAD (origin/HEAD -> origin/main)
    let out = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(name) = s.trim().rsplit('/').next() {
            return Some(name.to_string());
        }
    }
    Some("main".to_string())
}

pub fn delete_local_branch(repo_root: &Path, branch: &str) -> Result<(), GitError> {
    if default_branch(repo_root).as_deref() == Some(branch) {
        return Err(GitError::Command(format!("refusing to delete default branch {branch}")));
    }
    let safe = Command::new("git")
        .args(["branch", "-d", branch])
        .current_dir(repo_root)
        .output()?;
    if safe.status.success() {
        return Ok(());
    }
    let force = Command::new("git")
        .args(["branch", "-D", branch])
        .current_dir(repo_root)
        .output()?;
    if force.status.success() {
        Ok(())
    } else {
        Err(GitError::Command(String::from_utf8_lossy(&force.stderr).to_string()))
    }
}

fn local_branch_exists(repo_root: &Path, branch: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn ensure_gitignore(repo_root: &Path) -> Result<(), io::Error> {
    let gitignore_path = repo_root.join(".gitignore");
    let entries = [".grove/agent", ".grove/base", ".grove/start.sh", ".grove/setup.sh"];

    let existing = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let missing: Vec<&&str> = entries.iter()
        .filter(|e| !existing.lines().any(|l| l.trim() == **e))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)?;

    // Ensure newline before appending
    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    for entry in missing {
        writeln!(file, "{entry}")?;
    }

    Ok(())
}

/// Compute worktree directory path:
/// ~/.grove/workspaces/<repo>-<repo-hash>/<repo>-<workspace-name>/
pub fn worktree_path(repo_root: &Path, repo_name: &str, workspace_name: &str) -> PathBuf {
    let home = dirs::home_dir().expect("home directory should be available");
    let bucket = format!("{repo_name}-<repo-hash>");
    home.join(".grove")
        .join("workspaces")
        .join(bucket)
        .join(format!("{repo_name}-{workspace_name}"))
}
```

## Module Structure

```
src/
  main.rs              -- entry point, CLI args, Program::run
  model.rs             -- App model (Elm state), Message enum
  view.rs              -- view() rendering, delegates to mode-specific renderers
  update.rs            -- update() message handling
  workspace.rs         -- Workspace, AgentType, Status types
  tmux.rs              -- tmux session management (create, kill, capture, send-keys)
  worktree.rs          -- git worktree operations (create, remove, list, gitignore)
  setup.rs             -- workspace setup (env copy, setup script execution)
  keymap.rs            -- keybinding registry with multi-key support
  interactive.rs       -- interactive mode state, key forwarding, cursor overlay
  polling.rs           -- adaptive output polling, hash-based change detection
  capture_cache.rs     -- active-session registry, batch-capture cache/coordinator
  status.rs            -- agent status detection (output patterns, session files)
  session_detect.rs    -- per-agent session-file detectors + codex cwd/session cache
  mouse.rs             -- mouse event handling (click, drag, scroll, pane resize)
  widgets/
    workspace_list.rs  -- left pane widget (two-line items, status icons)
    output_preview.rs  -- right pane widget (preview + interactive rendering)
    dialog.rs          -- modal dialogs (new workspace, confirm delete, quit)
    status_line.rs     -- bottom status bar (mode indicator, hints, flash messages)
    pane_divider.rs    -- draggable pane divider
```

No `shell.rs` module by design in v1, shell sessions remain explicitly out of
scope for this PRD.

## Startup Flow

1. Parse CLI args (optional: project root, defaults to cwd)
2. Verify git repo (exit with error if not)
3. Verify tmux available (exit with error if not)
4. Discover workspaces: `git worktree list --porcelain`, filter by
   `.grove/agent` marker. Include main worktree as `is_main`.
5. Discover live sessions: `tmux list-sessions`, filter `grove-ws-*`
6. Reconcile: match sessions to worktrees, flag orphans/missing
7. If current cwd is missing (deleted worktree), resolve and switch to the
   main worktree path before launching UI
8. Start FrankenTUI Program in alt-screen mode (`ProgramConfig::fullscreen()`)
   with mouse policy auto (`with_mouse_capture_policy(MouseCapturePolicy::Auto)`)
   and runtime persistence configured (`with_persistence(...)`)
9. Begin tmux output polling subscription (adaptive intervals)

## Operational Targets

- Startup: p95 under 1.5s with up to 30 worktrees.
- Input latency: key-to-render p95 under 100ms in interactive mode.
- CPU: under 15% with one active selected session, under 35% with five active
  sessions (developer laptop baseline).
- Memory: under 250MB RSS with ten active sessions at 600 captured lines each.
- UI thread safety: no blocking process calls on UI thread, all git/tmux calls
  run in background tasks with timeouts.

## Edge Cases

- **Orphaned worktrees**: `.grove/agent` exists but tmux session is gone.
  Mark `is_orphaned`. Enter on this workspace auto-restarts the agent.
- **Orphaned sessions**: tmux session exists but worktree directory gone.
  Show warning indicator, allow cleanup.
- **Name collisions**: reject workspace names that match existing worktrees
  or tmux sessions.
- **Existing branch**: if existing branch field is set, attach to that branch
  even when it contains `/` or `.`. Workspace `name` stays slug-safe.
- **Agent crash**: detect non-zero exit via tmux, show error status, allow
  restart with `s`.
- **Start on running agent**: show flash message "Agent already running".
  User must Stop first, then Start.
- **Session death in interactive mode**: if `tmux send-keys` fails with
  "can't find pane" or "no such session", auto-exit interactive mode and
  show flash message in status bar.
- **Multiple Grove instances**: not handled initially. Single instance per
  project. Marker files are safe for concurrent reads; tmux sessions are
  inherently shared.
- **Missing worktree directory**: if directory is gone but git still tracks
  the worktree, auto-prune via `git worktree prune`.
- **Current workdir deleted while Grove runs**: detect missing cwd on refresh,
  resolve owning main worktree from sibling repos, and switch context to main.
- **Main worktree protection**: `D` (delete) is a no-op on the main
  worktree. It cannot be removed.
- **Double-Escape timing**: single Escape is forwarded to tmux after 150ms
  if no second Escape arrives. Two Escapes within 150ms exit interactive.

## Implementation Planning

`PRD.md` stays the product + technical spec. Execution sequencing should be
tracked in project tasks and PR descriptions.

Planning constraints:

- Ship in phases, each phase independently testable and reviewable
- No long validation gaps, each phase ends with a manual TUI milestone
- Each phase includes unit tests for all behavior added in that phase
- A phase is only complete when unit tests pass and manual milestone checks
  pass
- Scope per phase should stay small enough to finish in roughly 1-3 dev days

## Future Considerations (Explicitly Deferred)

These are not in scope but the design should not preclude them:

- Additional launch/runtime support for non-Claude/Codex agents
  (Gemini, Aider, Custom command, etc.)
- Task tracker integration
- Configuration file / keymap overrides
- Git stats display (+/- lines, commits ahead/behind)
- Shell sessions (non-agent workspaces)
- PR creation/merge workflows
- Kanban view
- Diff view (v2: add as a second tab in the preview pane)
- Multi-repo support
- Prompt templates with variable substitution
