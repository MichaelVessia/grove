#!/usr/bin/env bash
set -euo pipefail

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

for binary in tmux git cargo awk grep mktemp; do
  require_command "${binary}"
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
binary_path="${repo_root}/target/debug/grove"
emitter_path="${script_dir}/fake-codex-flicker-emitter.sh"

if [[ ! -x "${emitter_path}" ]]; then
  echo "expected executable emitter script at ${emitter_path}" >&2
  exit 1
fi

max_selection_steps="${GROVE_REATTACH_MAX_SELECTION_STEPS:-200}"
agent_cmd_override="${GROVE_REATTACH_AGENT_CMD:-bash ${emitter_path}}"

session_one="grove-reattach-a-$$-$(date +%s)"
session_two="grove-reattach-b-$$-$(date +%s)"
worktree_branch="grove-reattach-$$-$(date +%s)"
worktree_dir=""

cleanup() {
  set +e
  tmux kill-session -t "${session_one}" >/dev/null 2>&1 || true
  tmux kill-session -t "${session_two}" >/dev/null 2>&1 || true
  if [[ -n "${worktree_dir}" && -d "${worktree_dir}" ]]; then
    git -C "${repo_root}" worktree remove --force "${worktree_dir}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${worktree_branch}" ]]; then
    git -C "${repo_root}" branch -D "${worktree_branch}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

plain_capture() {
  local session_name="$1"
  tmux capture-pane -p -t "${session_name}" -S -200
}

selected_workspace_line() {
  local session_name="$1"
  plain_capture "${session_name}" | awk '/> / { print; exit }'
}

select_workspace() {
  local session_name="$1"
  local workspace_name="$2"

  for ((step = 0; step < max_selection_steps; step += 1)); do
    local selected_line
    selected_line="$(selected_workspace_line "${session_name}")"
    if [[ "${selected_line}" == *"${workspace_name}"* ]]; then
      return 0
    fi
    tmux send-keys -t "${session_name}" j
    sleep 0.03
  done

  echo "failed to select workspace '${workspace_name}' in ${session_name}" >&2
  plain_capture "${session_name}" >&2
  return 1
}

cd "${repo_root}"

base_branch="$(git rev-parse --abbrev-ref HEAD)"
if [[ "${base_branch}" == "HEAD" || -z "${base_branch}" ]]; then
  base_branch="main"
fi

repo_name="$(basename "${repo_root}")"
worktree_dir="$(mktemp -d "${TMPDIR:-/tmp}/${repo_name}-reattach-XXXXXX")"
git worktree add -q -b "${worktree_branch}" "${worktree_dir}" HEAD
printf 'codex\n' >"${worktree_dir}/.grove-agent"
printf '%s\n' "${base_branch}" >"${worktree_dir}/.grove-base"

cargo build --quiet

workspace_name="$(basename "${worktree_dir}")"
workspace_name="${workspace_name#${repo_name}-}"
workspace_session="grove-ws-${workspace_name}"
launch_command="GROVE_CODEX_CMD='${agent_cmd_override}' '${binary_path}'"

tmux new-session -d -s "${session_one}" -c "${repo_root}" "${launch_command}"
sleep 1
select_workspace "${session_one}" "${workspace_name}"

tmux send-keys -t "${session_one}" s
sleep 0.1
tmux send-keys -t "${session_one}" Enter
sleep 0.6

if ! tmux has-session -t "${workspace_session}" 2>/dev/null; then
  echo "expected running workspace session ${workspace_session}" >&2
  exit 1
fi

tmux send-keys -t "${session_one}" q
sleep 0.3

tmux new-session -d -s "${session_two}" -c "${repo_root}" "${launch_command}"
sleep 1
select_workspace "${session_two}" "${workspace_name}"

tmux send-keys -t "${session_two}" Enter
sleep 0.4

if ! plain_capture "${session_two}" | grep -q -- "-- INSERT --"; then
  echo "expected interactive reattach status after restart" >&2
  plain_capture "${session_two}" >&2
  exit 1
fi

echo "reattach smoke passed"
