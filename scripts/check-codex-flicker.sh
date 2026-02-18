#!/usr/bin/env bash
set -euo pipefail

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

for binary in tmux git cargo awk grep sed mktemp; do
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

sample_count="${GROVE_FLICKER_SAMPLE_COUNT:-120}"
sample_interval_seconds="${GROVE_FLICKER_SAMPLE_INTERVAL_SEC:-0.05}"
style_high_watermark="${GROVE_FLICKER_STYLE_HIGH_WATERMARK:-20}"
max_selection_steps="${GROVE_FLICKER_MAX_SELECTION_STEPS:-200}"
codex_cmd_override="${GROVE_FLICKER_CODEX_CMD:-bash ${emitter_path}}"

session_name="grove-flicker-$$-$(date +%s)"
worktree_branch="grove-flicker-$$-$(date +%s)"
worktree_dir=""
sample_log=""

cleanup() {
  set +e
  if [[ -n "${session_name}" ]]; then
    tmux kill-session -t "${session_name}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${worktree_dir}" && -d "${worktree_dir}" ]]; then
    git -C "${repo_root}" worktree remove --force "${worktree_dir}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${worktree_branch}" ]]; then
    git -C "${repo_root}" branch -D "${worktree_branch}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

plain_capture() {
  tmux capture-pane -p -t "${session_name}" -S -200
}

escaped_capture() {
  tmux capture-pane -p -e -t "${session_name}" -S -200
}

selected_workspace_line() {
  plain_capture | awk '/^[[:space:]]*[>▸] / { print; exit }'
}

sgr_count() {
  local frame="$1"
  local matches
  matches="$(printf '%s' "${frame}" | grep -o $'\x1b\\[[0-9;]*m' || true)"
  if [[ -z "${matches}" ]]; then
    echo 0
    return
  fi
  printf '%s\n' "${matches}" | wc -l | tr -d ' '
}

cd "${repo_root}"

base_branch="$(git rev-parse --abbrev-ref HEAD)"
if [[ "${base_branch}" == "HEAD" || -z "${base_branch}" ]]; then
  base_branch="main"
fi

repo_name="$(basename "${repo_root}")"
worktree_dir="$(mktemp -d "${TMPDIR:-/tmp}/${repo_name}-flicker-XXXXXX")"

git worktree add -q -b "${worktree_branch}" "${worktree_dir}" HEAD
mkdir -p "${worktree_dir}/.grove"
printf 'codex\n' >"${worktree_dir}/.grove/agent"
printf '%s\n' "${base_branch}" >"${worktree_dir}/.grove/base"

cargo build --quiet

launch_command="GROVE_CODEX_CMD='${codex_cmd_override}' '${binary_path}'"
tmux new-session -d -s "${session_name}" -c "${repo_root}" "${launch_command}"

sleep 1

workspace_name="$(basename "${worktree_dir}")"
workspace_name="${workspace_name#${repo_name}-}"

for ((step = 0; step < max_selection_steps; step += 1)); do
  selected_line="$(selected_workspace_line)"
  if [[ "${selected_line}" == *"${workspace_name}"* ]]; then
    break
  fi
  tmux send-keys -t "${session_name}" j
  sleep 0.03
done

selected_line="$(selected_workspace_line)"
if [[ "${selected_line}" != *"${workspace_name}"* ]]; then
  echo "failed to select harness workspace '${workspace_name}'" >&2
  echo "selected line: ${selected_line}" >&2
  echo "current pane:" >&2
  plain_capture >&2
  exit 1
fi

tmux send-keys -t "${session_name}" s
sleep 0.1
tmux send-keys -t "${session_name}" Enter
sleep 0.4
tmux send-keys -t "${session_name}" Enter
sleep 0.2

sample_log="$(mktemp "${TMPDIR:-/tmp}/grove-flicker-samples-XXXXXX.log")"

styled_frames=0
plain_frames=0
mode_transitions=0
last_mode=""

for ((sample = 0; sample < sample_count; sample += 1)); do
  frame="$(escaped_capture)"
  count="$(sgr_count "${frame}")"
  mode="plain"
  if (( count >= style_high_watermark )); then
    mode="styled"
  fi

  if [[ "${mode}" == "styled" ]]; then
    styled_frames=$((styled_frames + 1))
  else
    plain_frames=$((plain_frames + 1))
  fi

  if [[ -n "${last_mode}" && "${last_mode}" != "${mode}" ]]; then
    mode_transitions=$((mode_transitions + 1))
  fi
  last_mode="${mode}"

  printf 'sample=%03d mode=%s sgr=%s\n' "${sample}" "${mode}" "${count}" >>"${sample_log}"
  tmux send-keys -t "${session_name}" a
  sleep "${sample_interval_seconds}"
done

echo "styled frames: ${styled_frames}"
echo "plain frames: ${plain_frames}"
echo "mode transitions: ${mode_transitions}"
echo "sample log: ${sample_log}"

if (( styled_frames > 0 && plain_frames > 0 && mode_transitions >= 6 )); then
  echo "flicker detected: frame styling mode oscillated during interaction" >&2
  tail -n 20 "${sample_log}" >&2
  exit 1
fi

echo "no flicker detected by style-oscillation heuristic"
