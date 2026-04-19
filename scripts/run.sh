#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"

source "${script_dir}/runtime_env.sh"

cd "${repo_root}"

runtime_mode="$(radroots_studio_app_runtime_mode)"
run_id="$(radroots_studio_app_run_id "${runtime_mode}")"
default_nostr_relay_url="$(radroots_studio_app_default_nostr_relay_url)"
platform_name="$(radroots_studio_app_platform_name)"
bundle_identifier="$(radroots_studio_app_bundle_identifier)"
local_log_root="$(radroots_studio_app_local_log_root "${repo_root}")"

export RADROOTS_APP_RUN_ID="${run_id}"
export RADROOTS_APP_LOCAL_LOG_ROOT="${local_log_root}"
export RUST_LOG="${RADROOTS_APP_RUST_LOG:-info}"
export RADROOTS_APP_RUNTIME_CONFIG_JSON="$(
  radroots_studio_app_build_runtime_config_json \
    "${repo_root}" \
    "${runtime_mode}" \
    "${run_id}" \
    "${default_nostr_relay_url}" \
    "${bundle_identifier}" \
    "${platform_name}" \
    "${local_log_root}"
)"

if [[ "$(uname -s)" == "Darwin" ]]; then
  exec "${repo_root}/platforms/macos/Scripts/run_host.sh" "$@"
fi

exec cargo run -p radroots_studio_app -- "$@"
