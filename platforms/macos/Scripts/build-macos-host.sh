#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
platform_root="$(cd "${script_dir}/.." && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"
configuration="${CONFIGURATION:-Debug}"
bundle_name="Radroots.app"
bundle_root="${platform_root}/.derived-data/Build/Products/${configuration}/${bundle_name}"
contents_root="${bundle_root}/Contents"
executable_root="${contents_root}/MacOS"
resources_root="${contents_root}/Resources"
plist_template="${platform_root}/App/Resources/Info.plist"
plist_path="${contents_root}/Info.plist"
binary_target="${executable_root}/Radroots"
app_icon_path="${resources_root}/AppIcon.icns"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

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

cargo_target_dir() {
  cargo metadata --format-version 1 --no-deps | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"], end="")'
}

require_command cargo
require_command git
require_command python3
require_command /usr/libexec/PlistBuddy

(
  cd "${repo_root}"
  cargo build -p radroots_studio_app
)

binary_source="$(cargo_target_dir)/debug/radroots_studio_app"

if [[ ! -x "${binary_source}" ]]; then
  echo "missing desktop launcher binary: ${binary_source}" >&2
  exit 1
fi

rm -rf "${bundle_root}"
mkdir -p "${executable_root}" "${resources_root}"
cp "${plist_template}" "${plist_path}"
cp "${binary_source}" "${binary_target}"
chmod +x "${binary_target}"
"${script_dir}/generate-macos-app-icon.sh" "${app_icon_path}"

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $(workspace_version)" "${plist_path}"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${RADROOTS_APP_BUILD:-1}" "${plist_path}"

printf '%s\n' "${bundle_root}"
