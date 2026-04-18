#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
platform_root="$(cd "${script_dir}/.." && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"
date_utc="$(date -u +%F)"

source "${repo_root}/scripts/launch-config.sh"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_command /usr/libexec/PlistBuddy
require_command mktemp

app_path="$("${script_dir}/build-macos-host.sh")"
plist_path="${app_path}/Contents/Info.plist"
executable_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "${plist_path}"
)"
bundle_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleName' "${plist_path}"
)"
bundle_id="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${plist_path}"
)"
icon_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "${plist_path}"
)"

[[ "${bundle_name}" == "Radroots" ]] || {
  echo "unexpected CFBundleName: ${bundle_name}" >&2
  exit 1
}

[[ "${bundle_id}" == "org.radroots.app.macos" ]] || {
  echo "unexpected CFBundleIdentifier: ${bundle_id}" >&2
  exit 1
}

[[ -x "${app_path}/Contents/MacOS/${executable_name}" ]] || {
  echo "missing bundle executable: ${app_path}/Contents/MacOS/${executable_name}" >&2
  exit 1
}

[[ -f "${app_path}/Contents/Resources/${icon_name}.icns" ]] || {
  echo "missing bundle icon: ${app_path}/Contents/Resources/${icon_name}.icns" >&2
  exit 1
}

release_app_path="$(
  CONFIGURATION=Release "${script_dir}/build-macos-host.sh"
)"
[[ "${release_app_path}" == "${platform_root}/.derived-data/Build/Products/Release/Radroots.app" ]] || {
  echo "unexpected release bundle path: ${release_app_path}" >&2
  exit 1
}
[[ -x "${release_app_path}/Contents/MacOS/Radroots" ]] || {
  echo "missing release bundle executable: ${release_app_path}/Contents/MacOS/Radroots" >&2
  exit 1
}

tmp_root="$(mktemp -d)"
cleanup() {
  if [[ -n "${runner_pid:-}" ]] && kill -0 "${runner_pid}" 2>/dev/null; then
    kill "${runner_pid}" 2>/dev/null || true
    wait "${runner_pid}" || true
  fi
  rm -rf "${tmp_root}"
}
trap cleanup EXIT

runtime_mode="localhost-dev"
run_id="$(radroots_studio_app_run_id "${runtime_mode}")"
platform_name="$(radroots_studio_app_platform_name)"
bundle_identifier="$(radroots_studio_app_bundle_identifier)"
local_log_root="${tmp_root}/logs"
structured_log_file="${local_log_root}/apps/local/app/app-macos-native/${date_utc}.jsonl"
latest_log_path="${local_log_root}/apps/local/app/app-macos-native/latest.jsonl"
stdout_file="${local_log_root}/apps/local/app/app-macos-native/raw/stdout.${date_utc}.log"
stderr_file="${local_log_root}/apps/local/app/app-macos-native/raw/stderr.${date_utc}.log"

RADROOTS_APP_RUN_ID="${run_id}" \
RADROOTS_APP_LOCAL_LOG_ROOT="${local_log_root}" \
RADROOTS_APP_RUNTIME_CONFIG_JSON="$(
  radroots_studio_app_build_runtime_config_json \
    "${repo_root}" \
    "${runtime_mode}" \
    "${run_id}" \
    "${bundle_identifier}" \
    "${platform_name}" \
    "${local_log_root}"
)" \
"${script_dir}/run-macos-host.sh" &
runner_pid="$!"

launch_verified=false
for _ in $(seq 1 150); do
  if [[ -f "${structured_log_file}" ]] && grep -q '"event":"runtime.launch"' "${structured_log_file}" 2>/dev/null; then
    launch_verified=true
    break
  fi

  if ! kill -0 "${runner_pid}" 2>/dev/null; then
    wait "${runner_pid}"
    exit $?
  fi

  sleep 0.1
done

[[ "${launch_verified}" == "true" ]] || {
  echo "runtime.launch was not recorded by run-macos-host.sh" >&2
  exit 1
}

[[ -e "${latest_log_path}" ]] || {
  echo "missing latest structured log alias: ${latest_log_path}" >&2
  exit 1
}

[[ -f "${stdout_file}" ]] || {
  echo "missing raw stdout log: ${stdout_file}" >&2
  exit 1
}

[[ -f "${stderr_file}" ]] || {
  echo "missing raw stderr log: ${stderr_file}" >&2
  exit 1
}

kill "${runner_pid}" 2>/dev/null || true
set +e
wait "${runner_pid}"
exit_code="$?"
set -e
[[ "${exit_code}" == "0" ]] || [[ "${exit_code}" == "143" ]] || [[ "${exit_code}" == "130" ]] || {
  echo "unexpected runner exit code after termination: ${exit_code}" >&2
  exit 1
}
runner_pid=""
