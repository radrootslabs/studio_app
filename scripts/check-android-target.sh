#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
android_root="${app_root}/platforms/android"

source "${android_root}/Scripts/android_toolchain_config.sh"

"${android_root}/Scripts/bootstrap_android_toolchain.sh"

export PATH="${android_cargo_bin_dir}:${PATH}"
export ANDROID_HOME="${android_sdk_dir}"
export ANDROID_SDK_ROOT="${android_sdk_dir}"
export ANDROID_NDK_HOME="${android_ndk_dir}"
export ANDROID_NDK_ROOT="${android_ndk_dir}"
export ANDROID_USER_HOME="${android_user_home}"

CARGO_TARGET_DIR="${app_root}/target" \
  cargo ndk -t "${android_abi}" check --manifest-path "${app_root}/Cargo.toml" -p radroots_studio_app_android
