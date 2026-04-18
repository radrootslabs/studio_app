#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
date_utc="$(date -u +%F)"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_env() {
  local name="$1"

  if [[ -z "${!name:-}" ]]; then
    echo "missing required environment variable: ${name}" >&2
    exit 1
  fi
}

forward_signal() {
  local signal="$1"

  if [[ -n "${app_pid:-}" ]] && kill -0 "${app_pid}" 2>/dev/null; then
    kill "-${signal}" "${app_pid}" 2>/dev/null || kill "${app_pid}" 2>/dev/null || true
  fi
}

require_command grep
require_command /usr/libexec/PlistBuddy
require_env RADROOTS_APP_RUNTIME_CONFIG_JSON
require_env RADROOTS_APP_LOCAL_LOG_ROOT

app_path="$("${script_dir}/build-macos-host.sh")"
plist_path="${app_path}/Contents/Info.plist"
executable_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "${plist_path}"
)"
executable_path="${app_path}/Contents/MacOS/${executable_name}"
app_log_root="${RADROOTS_APP_LOCAL_LOG_ROOT}/apps/local/app/app-macos-native"
structured_log_file="${app_log_root}/${date_utc}.jsonl"
stdout_file="${app_log_root}/raw/stdout.${date_utc}.log"
stderr_file="${app_log_root}/raw/stderr.${date_utc}.log"

mkdir -p "${app_log_root}/raw"
export RUST_LOG="${RADROOTS_APP_RUST_LOG:-info}"

trap 'forward_signal TERM' TERM
trap 'forward_signal INT' INT
trap 'forward_signal HUP' HUP

"${executable_path}" "$@" >>"${stdout_file}" 2>>"${stderr_file}" &
app_pid="$!"

launch_confirmed=false
for _ in $(seq 1 100); do
  if [[ -f "${structured_log_file}" ]] && grep -q '"event":"runtime.launch"' "${structured_log_file}" 2>/dev/null; then
    launch_confirmed=true
    break
  fi

  if ! kill -0 "${app_pid}" 2>/dev/null; then
    wait "${app_pid}"
    exit $?
  fi

  sleep 0.1
done

if [[ "${launch_confirmed}" != "true" ]]; then
  if kill -0 "${app_pid}" 2>/dev/null; then
    kill "${app_pid}" 2>/dev/null || true
    wait "${app_pid}" || true
  fi
  echo "app launch did not emit runtime.launch within startup timeout" >&2
  exit 1
fi

wait "${app_pid}"
