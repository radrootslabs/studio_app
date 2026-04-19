#!/usr/bin/env bash

radroots_studio_app_workspace_version() {
  local repo_root="$1"

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

radroots_studio_app_runtime_mode() {
  printf '%s' "${RADROOTS_APP_RUNTIME_MODE:-localhost-dev}"
}

radroots_studio_app_run_id() {
  local runtime_mode="$1"

  if [[ -n "${RADROOTS_APP_RUN_ID:-}" ]]; then
    printf '%s' "${RADROOTS_APP_RUN_ID}"
    return
  fi

  RADROOTS_APP_RUNTIME_MODE_FOR_RUN_ID="${runtime_mode}" python3 - <<'PY'
import os
import secrets
import time

runtime_mode = os.environ["RADROOTS_APP_RUNTIME_MODE_FOR_RUN_ID"].strip().lower() or "unknown"
timestamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
suffix = secrets.token_hex(8)
print(f"app-{runtime_mode}-{timestamp}-{suffix}", end="")
PY
}

radroots_studio_app_platform_name() {
  case "$(uname -s)" in
    Darwin) printf 'macos' ;;
    Linux) printf 'linux' ;;
    *) uname -s | tr '[:upper:]' '[:lower:]' ;;
  esac
}

radroots_studio_app_bundle_identifier() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    printf 'org.radroots.app.macos'
    return
  fi

  printf 'org.radroots.app.desktop'
}

radroots_studio_app_os_version() {
  printf '%s-%s' "$(radroots_studio_app_platform_name)" "$(uname -r)"
}

radroots_studio_app_local_log_root() {
  local repo_root="$1"

  if [[ -n "${RADROOTS_APP_LOCAL_LOG_ROOT:-}" ]]; then
    printf '%s' "${RADROOTS_APP_LOCAL_LOG_ROOT}"
    return
  fi

  printf '%s' "${repo_root}/logs"
}

radroots_studio_app_default_nostr_relay_url() {
  if [[ -n "${RADROOTS_APP_DEFAULT_NOSTR_RELAY_URL:-}" ]]; then
    printf '%s' "${RADROOTS_APP_DEFAULT_NOSTR_RELAY_URL}"
    return
  fi

  printf 'missing required env: RADROOTS_APP_DEFAULT_NOSTR_RELAY_URL\n' >&2
  exit 1
}

radroots_studio_app_build_runtime_config_json() {
  local repo_root="$1"
  local runtime_mode="$2"
  local run_id="$3"
  local default_nostr_relay_url="$4"
  local bundle_identifier="$5"
  local platform_name="$6"
  local local_log_root="$7"

  RADROOTS_APP_RUNTIME_CONFIG_SCHEMA="radroots.app.runtime-config.v1" \
  RADROOTS_APP_RUNTIME_MODE="${runtime_mode}" \
  RADROOTS_APP_RUN_ID="${run_id}" \
  RADROOTS_APP_DEFAULT_NOSTR_RELAY_URL="${default_nostr_relay_url}" \
  RADROOTS_APP_BUNDLE_IDENTIFIER="${bundle_identifier}" \
  RADROOTS_APP_BUNDLE_NAME="Radroots" \
  RADROOTS_APP_MARKETING_VERSION="$(radroots_studio_app_workspace_version "${repo_root}")" \
  RADROOTS_APP_BUILD_NUMBER="${RADROOTS_APP_BUILD:-dev}" \
  RADROOTS_APP_PLATFORM_NAME="${platform_name}" \
  RADROOTS_APP_OS_VERSION="$(radroots_studio_app_os_version)" \
  RADROOTS_APP_HOST_LOCALE="${LANG:-system-default}" \
  RADROOTS_APP_RUNTIME_ORIGIN="gpui://localhost" \
  RADROOTS_APP_LOCAL_LOG_ROOT="${local_log_root}" \
  python3 - <<'PY'
import json
import os

print(json.dumps({
    "schema_version": os.environ["RADROOTS_APP_RUNTIME_CONFIG_SCHEMA"],
    "runtime_mode": os.environ["RADROOTS_APP_RUNTIME_MODE"],
    "run_id": os.environ["RADROOTS_APP_RUN_ID"],
    "default_nostr_relay_url": os.environ["RADROOTS_APP_DEFAULT_NOSTR_RELAY_URL"],
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
