#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
android_root="${app_root}/platforms/android"
configuration="${CONFIGURATION:-Debug}"

source "${android_root}/Scripts/android_toolchain_config.sh"

"${android_root}/Scripts/bootstrap_android_toolchain.sh"

gradle_task=":app:assembleDebug"
expected_apk="${android_root}/app/build/outputs/apk/debug/app-debug.apk"
if [[ "${configuration}" == "Release" ]]; then
  gradle_task=":app:assembleRelease"
  expected_apk="${android_root}/app/build/outputs/apk/release/app-release-unsigned.apk"
fi

(
  cd "${android_root}"
  GRADLE_USER_HOME="${android_gradle_user_home}" \
    ANDROID_USER_HOME="${android_user_home}" \
    ANDROID_HOME="${android_sdk_dir}" \
    ANDROID_SDK_ROOT="${android_sdk_dir}" \
    "${android_gradle_bin}" --no-daemon "${gradle_task}"
)

if [[ ! -f "${expected_apk}" ]]; then
  echo "missing expected android apk: ${expected_apk}" >&2
  exit 1
fi

printf '%s\n' "${expected_apk}"
