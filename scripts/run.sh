#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"

workspace_version() {
  python3 - <<'PY' "${repo_root}/Cargo.toml"
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    cargo_toml = handle.read()

match = re.search(r'^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
if not match:
    raise SystemExit("missing workspace.package.version")

print(match.group(1), end="")
PY
}

current_runtime_mode() {
  printf '%s' "${RADROOTS_APP_RUNTIME_MODE:-localhost-dev}"
}

current_run_id() {
  if [[ -n "${RADROOTS_APP_RUN_ID:-}" ]]; then
    printf '%s' "${RADROOTS_APP_RUN_ID}"
    return
  fi

  RADROOTS_APP_RUNTIME_MODE_FOR_RUN_ID="$(current_runtime_mode)" python3 - <<'PY'
import os
import secrets
import time

runtime_mode = os.environ["RADROOTS_APP_RUNTIME_MODE_FOR_RUN_ID"].strip().lower() or "unknown"
timestamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
suffix = secrets.token_hex(8)
print(f"app-{runtime_mode}-{timestamp}-{suffix}", end="")
PY
}

current_platform_name() {
  case "$(uname -s)" in
    Darwin) printf 'macos' ;;
    Linux) printf 'linux' ;;
    *) uname -s | tr '[:upper:]' '[:lower:]' ;;
  esac
}

current_bundle_identifier() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    printf 'org.radroots.app.macos'
    return
  fi

  printf 'org.radroots.app.desktop'
}

current_os_version() {
  printf '%s-%s' "$(current_platform_name)" "$(uname -r)"
}

build_runtime_config_json() {
  local runtime_mode="$1"
  local run_id="$2"
  local bundle_identifier="$3"
  local platform_name="$4"

  RADROOTS_APP_RUNTIME_CONFIG_SCHEMA="radroots.app.runtime-config.v1" \
  RADROOTS_APP_RUNTIME_MODE="${runtime_mode}" \
  RADROOTS_APP_RUN_ID="${run_id}" \
  RADROOTS_APP_BUNDLE_IDENTIFIER="${bundle_identifier}" \
  RADROOTS_APP_BUNDLE_NAME="Radroots" \
  RADROOTS_APP_MARKETING_VERSION="$(workspace_version)" \
  RADROOTS_APP_BUILD_NUMBER="${RADROOTS_APP_BUILD:-dev}" \
  RADROOTS_APP_PLATFORM_NAME="${platform_name}" \
  RADROOTS_APP_OS_VERSION="$(current_os_version)" \
  RADROOTS_APP_HOST_LOCALE="${LANG:-system-default}" \
  RADROOTS_APP_RUNTIME_ORIGIN="gpui://localhost" \
  RADROOTS_APP_LOCAL_LOG_ROOT="${repo_root}/logs" \
  python3 - <<'PY'
import json
import os

print(json.dumps({
    "schema_version": os.environ["RADROOTS_APP_RUNTIME_CONFIG_SCHEMA"],
    "runtime_mode": os.environ["RADROOTS_APP_RUNTIME_MODE"],
    "run_id": os.environ["RADROOTS_APP_RUN_ID"],
    "bundle_identifier": os.environ["RADROOTS_APP_BUNDLE_IDENTIFIER"],
    "bundle_name": os.environ["RADROOTS_APP_BUNDLE_NAME"],
    "marketing_version": os.environ["RADROOTS_APP_MARKETING_VERSION"],
    "build_number": os.environ["RADROOTS_APP_BUILD_NUMBER"],
    "platform_name": os.environ["RADROOTS_APP_PLATFORM_NAME"],
    "operating_system_version": os.environ["RADROOTS_APP_OS_VERSION"],
    "host_locale": os.environ["RADROOTS_APP_HOST_LOCALE"],
    "runtime_origin": os.environ["RADROOTS_APP_RUNTIME_ORIGIN"],
    "local_log_root": os.environ["RADROOTS_APP_LOCAL_LOG_ROOT"],
}, sort_keys=True, separators=(",", ":")), end="")
PY
}

cd "${repo_root}"

runtime_mode="$(current_runtime_mode)"
run_id="$(current_run_id)"
platform_name="$(current_platform_name)"
bundle_identifier="$(current_bundle_identifier)"

export RADROOTS_APP_RUN_ID="${run_id}"
export RADROOTS_APP_RUNTIME_CONFIG_JSON="$(
  build_runtime_config_json \
    "${runtime_mode}" \
    "${run_id}" \
    "${bundle_identifier}" \
    "${platform_name}"
)"

if [[ "$(uname -s)" == "Darwin" ]]; then
  exec "${repo_root}/platforms/macos/Scripts/run-macos-host.sh" "$@"
fi

exec cargo run -p radroots_studio_app -- "$@"
