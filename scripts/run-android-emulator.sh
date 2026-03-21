#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
android_root="${app_root}/platforms/android"
bundle_id="org.radroots.app.android"
activity_name="${bundle_id}/.MainActivity"

source "${android_root}/Scripts/android_toolchain_config.sh"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

running_emulator_serial() {
  local target_avd="$1"
  while read -r serial state _; do
    [[ "${serial}" == emulator-* ]] || continue
    [[ "${state}" == "device" || "${state}" == "offline" ]] || continue
    if [[ "$("${android_adb_bin}" -s "${serial}" emu avd name 2>/dev/null | sed -n '1p' | tr -d '\r')" == "${target_avd}" ]]; then
      printf '%s\n' "${serial}"
      return
    fi
  done < <("${android_adb_bin}" devices | tail -n +2)
}

ensure_avd() {
  local avd_name="$1"
  if [[ -d "${android_avd_home}/${avd_name}.avd" ]]; then
    return
  fi

  mkdir -p "${android_avd_home}" "${android_emulator_home}"
  printf 'no\n' | \
    ANDROID_AVD_HOME="${android_avd_home}" \
    ANDROID_EMULATOR_HOME="${android_emulator_home}" \
    "${android_avdmanager_bin}" create avd --force --name "${avd_name}" --package "$(android_emulator_system_image_package)"
}

wait_for_boot_complete() {
  local serial="$1"
  "${android_adb_bin}" -s "${serial}" wait-for-device >/dev/null
  until [[ "$("${android_adb_bin}" -s "${serial}" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; do
    sleep 2
  done
}

launch_emulator_if_needed() {
  local avd_name="$1"
  local serial
  serial="$(running_emulator_serial "${avd_name}" || true)"
  if [[ -n "${serial}" ]]; then
    printf '%s\n' "${serial}"
    return
  fi

  ANDROID_AVD_HOME="${android_avd_home}" \
    ANDROID_EMULATOR_HOME="${android_emulator_home}" \
    nohup "${android_emulator_bin}" -avd "${avd_name}" -no-snapshot-save >/tmp/radroots-android-emulator.log 2>&1 &

  for _ in $(seq 1 60); do
    serial="$(running_emulator_serial "${avd_name}" || true)"
    if [[ -n "${serial}" ]]; then
      printf '%s\n' "${serial}"
      return
    fi
    sleep 2
  done

  echo "android emulator failed to start" >&2
  exit 1
}

require_command mktemp

avd_name="${1:-$(android_avd_name)}"

"${android_root}/Scripts/bootstrap_android_toolchain.sh" --with-emulator

build_log="$(mktemp)"
trap 'rm -f "${build_log}"' EXIT

if ! "${script_dir}/build-android-host.sh" | tee "${build_log}"; then
  exit 1
fi

apk_path="$(tail -n 1 "${build_log}")"
if [[ ! -f "${apk_path}" ]]; then
  echo "missing built android apk: ${apk_path}" >&2
  exit 1
fi

ensure_avd "${avd_name}"
serial="$(launch_emulator_if_needed "${avd_name}")"
wait_for_boot_complete "${serial}"

"${android_adb_bin}" -s "${serial}" install -r "${apk_path}"
"${android_adb_bin}" -s "${serial}" shell am start -n "${activity_name}"
