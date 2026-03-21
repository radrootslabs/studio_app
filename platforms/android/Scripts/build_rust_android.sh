#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
source "${script_dir}/android_toolchain_config.sh"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

profile_for_build_type() {
  case "${1}" in
    Release)
      echo "release"
      ;;
    *)
      echo "debug"
      ;;
  esac
}

missing_bootstrap() {
  echo "android build requires bootstrapped local toolchain files under platforms/android/.tooling" >&2
  exit 1
}

require_command cargo
require_command rustup

if [[ ! -d "${android_sdk_dir}" || ! -d "${android_ndk_dir}" || ! -x "${android_cargo_ndk_bin}" ]]; then
  missing_bootstrap
fi

if ! rustup target list --installed | grep -Fx "${android_rust_target}" >/dev/null 2>&1; then
  missing_bootstrap
fi

build_type="${1:-Debug}"
profile="$(profile_for_build_type "${build_type}")"

export PATH="${android_cargo_bin_dir}:${PATH}"
export ANDROID_HOME="${android_sdk_dir}"
export ANDROID_SDK_ROOT="${android_sdk_dir}"
export ANDROID_NDK_HOME="${android_ndk_dir}"
export ANDROID_NDK_ROOT="${android_ndk_dir}"
export ANDROID_USER_HOME="${android_user_home}"

cargo_args=(
  ndk
  -t "${android_abi}"
  -o "${app_root}/target/android/jniLibs"
  build
  --manifest-path "${app_root}/Cargo.toml"
  -p radroots-app-android
)

if [[ "${profile}" == "release" ]]; then
  cargo_args+=(--release)
fi

cargo "${cargo_args[@]}"
