#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
ios_root="$(cd "${script_dir}/.." && pwd -P)"
app_root="$(cd "${ios_root}/../.." && pwd -P)"
ios_target_dir="${app_root}/target"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_rust_target() {
  local target="$1"
  if rustup target list --installed | grep -Fx "${target}" >/dev/null 2>&1; then
    return
  fi
  echo "missing required rust target: ${target}" >&2
  exit 1
}

profile_for_configuration() {
  case "${1}" in
    Release)
      echo "release"
      ;;
    *)
      echo "debug"
      ;;
  esac
}

build_target() {
  local target="$1"
  local profile="$2"
  local cargo_args=(
    build
    --manifest-path "${app_root}/Cargo.toml"
    -p radroots-app-ios
    --target "${target}"
  )
  if [[ "${profile}" == "release" ]]; then
    cargo_args+=(--release)
  fi
  CARGO_TARGET_DIR="${ios_target_dir}" cargo "${cargo_args[@]}"
}

build_targets() {
  local profile="$1"
  shift
  for target in "$@"; do
    require_rust_target "${target}"
    build_target "${target}" "${profile}"
  done
}

require_command cargo
require_command rustup

configuration="${CONFIGURATION:-Debug}"
profile="$(profile_for_configuration "${configuration}")"
sdk_name="${SDK_NAME:-}"
archs="${ARCHS:-}"

if [[ -n "${sdk_name}" ]]; then
  case "${sdk_name}" in
    iphoneos*)
      build_targets "${profile}" aarch64-apple-ios
      ;;
    iphonesimulator*)
      if [[ " ${archs} " == *" x86_64 "* ]]; then
        build_targets "${profile}" aarch64-apple-ios-sim x86_64-apple-ios
      else
        build_targets "${profile}" aarch64-apple-ios-sim
      fi
      ;;
    *)
      echo "unsupported iOS SDK_NAME: ${sdk_name}" >&2
      exit 1
      ;;
  esac
  exit 0
fi

build_targets "${profile}" aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
