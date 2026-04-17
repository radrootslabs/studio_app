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
bundle_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleName' "${plist_path}"
)"
bundle_id="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${plist_path}"
)"
icon_name="$(
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "${plist_path}"
)"

[[ "${bundle_name}" == "Radroots" ]] || {
  echo "unexpected CFBundleName: ${bundle_name}" >&2
  exit 1
}

[[ "${bundle_id}" == "org.radroots.app.macos" ]] || {
  echo "unexpected CFBundleIdentifier: ${bundle_id}" >&2
  exit 1
}

[[ -x "${app_path}/Contents/MacOS/${executable_name}" ]] || {
  echo "missing bundle executable: ${app_path}/Contents/MacOS/${executable_name}" >&2
  exit 1
}

[[ -f "${app_path}/Contents/Resources/${icon_name}.icns" ]] || {
  echo "missing bundle icon: ${app_path}/Contents/Resources/${icon_name}.icns" >&2
  exit 1
}
