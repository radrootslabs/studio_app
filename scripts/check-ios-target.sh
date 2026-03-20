#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
ios_target_dir="${app_root}/target"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

ios_sim_rust_targets_for_host() {
  case "$(uname -m)" in
    arm64|aarch64)
      printf '%s\n' "aarch64-apple-ios-sim"
      ;;
    x86_64)
      printf '%s\n' "aarch64-apple-ios-sim" "x86_64-apple-ios"
      ;;
    *)
      echo "unsupported host architecture for ios simulator: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

require_rust_target() {
  local target="$1"
  if rustup target list --installed | grep -Fx "${target}" >/dev/null 2>&1; then
    return
  fi
  echo "missing required rust target: ${target}" >&2
  exit 1
}

require_command cargo
require_command rustup

cd "${app_root}"

declare -a targets=()
if [[ -n "${IOS_SIM_RUST_TARGET:-}" ]]; then
  targets=("${IOS_SIM_RUST_TARGET}")
else
  while IFS= read -r target; do
    targets+=("${target}")
  done < <(ios_sim_rust_targets_for_host)
fi

for target in "${targets[@]}"; do
  require_rust_target "${target}"
  CARGO_TARGET_DIR="${ios_target_dir}" \
    cargo check --manifest-path "${app_root}/Cargo.toml" -p radroots-app-ios --target "${target}"
done
