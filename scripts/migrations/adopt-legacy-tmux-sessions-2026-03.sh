#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Adopt legacy Grove tmux sessions into 2026-03 tab metadata format.

Default mode is dry-run.

Usage:
  scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh [--apply] [--include-attached]

Options:
  --apply             Execute rename/set-option commands.
  --include-attached  Include sessions with attached clients.
  -h, --help          Show this help.
USAGE
}

is_uint() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

canonical_path() {
  local raw="$1"
  if [ -z "$raw" ]; then
    return
  fi
  if [ -d "$raw" ]; then
    (cd "$raw" && pwd -P) || printf '%s\n' "$raw"
    return
  fi
  printf '%s\n' "$raw"
}

session_workspace_path() {
  local session="$1"
  local pane_path
  pane_path="$(tmux display-message -p -t "${session}:0.0" '#{pane_current_path}' 2>/dev/null || true)"
  if [ -z "$pane_path" ]; then
    pane_path="$(tmux display-message -p -t "$session" '#{pane_current_path}' 2>/dev/null || true)"
  fi

  local workspace_path=""
  if [ -n "$pane_path" ]; then
    workspace_path="$(git -C "$pane_path" rev-parse --show-toplevel 2>/dev/null || true)"
  fi
  if [ -z "$workspace_path" ] && [ -n "$pane_path" ]; then
    workspace_path="$pane_path"
  fi

  canonical_path "$workspace_path"
}

agent_label() {
  case "$1" in
    claude) echo "Claude" ;;
    opencode) echo "OpenCode" ;;
    *) echo "Codex" ;;
  esac
}

infer_agent_marker() {
  local session="$1"

  local option_marker
  option_marker="$(tmux show-options -qv -t "$session" @grove_tab_agent 2>/dev/null || true)"
  option_marker="$(printf '%s' "$option_marker" | tr '[:upper:]' '[:lower:]')"
  case "$option_marker" in
    claude|codex|opencode)
      echo "$option_marker"
      return
      ;;
  esac

  local pane_command
  pane_command="$(tmux list-panes -t "$session" -F '#{pane_current_command}' 2>/dev/null | head -n 1 | tr '[:upper:]' '[:lower:]')"
  case "$pane_command" in
    *claude*) echo "claude"; return ;;
    *opencode*) echo "opencode"; return ;;
    *codex*) echo "codex"; return ;;
  esac

  local capture
  capture="$(tmux capture-pane -p -t "$session" -S -200 2>/dev/null | tr '[:upper:]' '[:lower:]' || true)"
  case "$capture" in
    *"opencode"*) echo "opencode"; return ;;
    *"claude"*) echo "claude"; return ;;
    *"codex"*) echo "codex"; return ;;
  esac

  echo "codex"
}

apply=0
include_attached=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --apply)
      apply=1
      ;;
    --include-attached)
      include_attached=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux not found" >&2
  exit 1
fi

raw_sessions="$(tmux list-sessions -F '#{session_name}|#{session_attached}' 2>/dev/null || true)"
if [ -z "$raw_sessions" ]; then
  echo "no tmux sessions"
  exit 0
fi

tab_seed="$(date +%s)"
if ! is_uint "$tab_seed"; then
  tab_seed=2000000000
fi
tab_counter=0

planned=0
applied=0
skipped_attached=0
skipped_conflict=0
skipped_unknown=0

while IFS='|' read -r session attached_raw; do
  [ -n "$session" ] || continue
  [[ "$session" == grove-ws-* ]] || continue

  kind_meta="$(tmux show-options -qv -t "$session" @grove_tab_kind 2>/dev/null || true)"
  if [ -n "$kind_meta" ]; then
    continue
  fi

  attached_clients=0
  if is_uint "${attached_raw:-}"; then
    attached_clients="$attached_raw"
  fi

  if [ "$include_attached" -eq 0 ] && [ "$attached_clients" -gt 0 ]; then
    skipped_attached=$((skipped_attached + 1))
    echo "SKIP attached session=$session attached=$attached_clients"
    continue
  fi

  kind=""
  target_session="$session"
  title=""
  agent_marker=""

  if [[ "$session" =~ ^(.+)-git$ ]]; then
    kind="git"
    target_session="$session"
    title="Git"
  elif [[ "$session" =~ ^(.+)-shell-([0-9]+)$ ]]; then
    kind="shell"
    ordinal="${BASH_REMATCH[2]}"
    target_session="$session"
    title="Shell $ordinal"
  elif [[ "$session" =~ ^(.+)-shell$ ]]; then
    kind="shell"
    target_session="${BASH_REMATCH[1]}-shell-1"
    title="Shell 1"
  elif [[ "$session" =~ ^(.+)-agent-([0-9]+)$ ]]; then
    kind="agent"
    ordinal="${BASH_REMATCH[2]}"
    target_session="$session"
    agent_marker="$(infer_agent_marker "$session")"
    title="$(agent_label "$agent_marker") $ordinal"
  else
    kind="agent"
    target_session="${session}-agent-1"
    agent_marker="$(infer_agent_marker "$session")"
    title="$(agent_label "$agent_marker") 1"
  fi

  if [ "$target_session" != "$session" ] && tmux has-session -t "$target_session" 2>/dev/null; then
    skipped_conflict=$((skipped_conflict + 1))
    echo "SKIP conflict session=$session target=$target_session already_exists=1"
    continue
  fi

  workspace_path="$(session_workspace_path "$session")"
  if [ -z "$workspace_path" ]; then
    skipped_unknown=$((skipped_unknown + 1))
    echo "SKIP unknown-workspace session=$session"
    continue
  fi

  tab_counter=$((tab_counter + 1))
  tab_id=$((tab_seed * 100 + tab_counter))

  planned=$((planned + 1))
  echo "PLAN session=$session target=$target_session kind=$kind workspace=$workspace_path tab_id=$tab_id agent=${agent_marker:-none} attached=$attached_clients"

  if [ "$apply" -eq 0 ]; then
    continue
  fi

  current_session="$session"
  if [ "$target_session" != "$session" ]; then
    tmux rename-session -t "$session" "$target_session"
    current_session="$target_session"
  fi

  tmux set-option -t "$current_session" @grove_workspace_path "$workspace_path"
  tmux set-option -t "$current_session" @grove_tab_kind "$kind"
  tmux set-option -t "$current_session" @grove_tab_title "$title"
  tmux set-option -t "$current_session" @grove_tab_agent "$agent_marker"
  tmux set-option -t "$current_session" @grove_tab_id "$tab_id"

  applied=$((applied + 1))
  echo "APPLY session=$current_session kind=$kind tab_id=$tab_id"
done <<< "$raw_sessions"

echo "SUMMARY planned=$planned applied=$applied skipped_attached=$skipped_attached skipped_conflict=$skipped_conflict skipped_unknown_workspace=$skipped_unknown"
if [ "$apply" -eq 0 ]; then
  echo "dry-run complete, rerun with --apply to execute"
fi
