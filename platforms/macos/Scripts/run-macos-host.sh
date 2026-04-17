#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_command /usr/libexec/PlistBuddy

app_path="$("${script_dir}/build-macos-host.sh")"
plist_path="${app_path}/Contents/Info.plist"
executable_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "${plist_path}"
)"
executable_path="${app_path}/Contents/MacOS/${executable_name}"

"${executable_path}" "$@" &
disown
