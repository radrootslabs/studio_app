#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
bundle_id="org.radroots.app.ios"
device_selector="${1:-${IOS_SIMULATOR_DEVICE:-iPhone 16}}"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

resolve_simulator_udid() {
  local selector="$1"
  if [[ "${selector}" =~ ^[0-9A-F-]{36}$ ]]; then
    printf '%s\n' "${selector}"
    return
  fi

  local line
  line="$(
    xcrun simctl list devices available |
      awk -v name="${selector}" '$0 ~ ("^[[:space:]]+" name " \\(") { print; exit }'
  )"

  if [[ -z "${line}" ]]; then
    echo "unable to find available iOS simulator: ${selector}" >&2
    exit 1
  fi

  printf '%s\n' "${line}" | awk -F '[()]' '{ print $2 }'
}

require_command open
require_command xcrun
require_command mktemp

build_log="$(mktemp)"
trap 'rm -f "${build_log}"' EXIT

if ! "${script_dir}/build-ios-host.sh" | tee "${build_log}"; then
  exit 1
fi

app_path="$(tail -n 1 "${build_log}")"
if [[ ! -d "${app_path}" ]]; then
  echo "missing built iOS app bundle: ${app_path}" >&2
  exit 1
fi

device_udid="$(resolve_simulator_udid "${device_selector}")"

xcrun simctl boot "${device_udid}" >/dev/null 2>&1 || true
xcrun simctl bootstatus "${device_udid}" -b
open -a Simulator --args -CurrentDeviceUDID "${device_udid}" >/dev/null 2>&1 || open -a Simulator >/dev/null 2>&1
xcrun simctl install "${device_udid}" "${app_path}"
xcrun simctl launch "${device_udid}" "${bundle_id}"
