#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"

cd "${repo_root}"

runtime_mode="${RADROOTS_APP_RUNTIME_MODE:-localhost-dev}"
nostr_relay_urls="${RADROOTS_APP_NOSTR_RELAY_URLS:-}"
local_log_root="${RADROOTS_APP_LOCAL_LOG_ROOT:-${repo_root}/logs}"

if [[ -z "${nostr_relay_urls}" ]]; then
  echo "missing required env: RADROOTS_APP_NOSTR_RELAY_URLS" >&2
  exit 1
fi

export RADROOTS_APP_RUNTIME_MODE="${runtime_mode}"
export RADROOTS_APP_NOSTR_RELAY_URLS="${nostr_relay_urls}"
export RADROOTS_APP_LOCAL_LOG_ROOT="${local_log_root}"
export RUST_LOG="${RADROOTS_APP_RUST_LOG:-info}"

if [[ "$(uname -s)" == "Darwin" ]]; then
  exec "${repo_root}/platforms/macos/Scripts/run_host.sh" "$@"
fi

exec cargo run -p radroots_studio_app -- "$@"
