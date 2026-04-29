#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
platform_root="$(cd "${script_dir}/.." && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"
requested_configuration="${CONFIGURATION:-Debug}"
bundle_name="Radroots.app"
plist_template="${platform_root}/App/Resources/Info.plist"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

append_env_value() {
  local var_name="$1"
  local value="$2"
  local current="${!var_name:-}"

  if [[ -n "${current}" ]]; then
    export "${var_name}=${value} ${current}"
  else
    export "${var_name}=${value}"
  fi
}

prepare_macos_build_env() {
  local sdk_path
  local bindgen_args

  sdk_path="$(xcrun --sdk macosx --show-sdk-path)"
  if [[ -z "${sdk_path}" || ! -d "${sdk_path}" ]]; then
    echo "unable to resolve macos sdk path via xcrun" >&2
    exit 1
  fi

  if [[ ! -f "${sdk_path}/usr/include/dispatch/dispatch.h" ]]; then
    echo "missing macos dispatch header: ${sdk_path}/usr/include/dispatch/dispatch.h" >&2
    exit 1
  fi

  export SDKROOT="${sdk_path}"
  bindgen_args="--sysroot=${SDKROOT} -I${SDKROOT}/usr/include"

  append_env_value BINDGEN_EXTRA_CLANG_ARGS "${bindgen_args}"
  append_env_value BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin "${bindgen_args}"
  append_env_value BINDGEN_EXTRA_CLANG_ARGS_x86_64_apple_darwin "${bindgen_args}"
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
  (
    cd "${repo_root}"
    cargo metadata --format-version 1 --no-deps | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"], end="")'
  )
}

configure_build_lane() {
  case "${requested_configuration}" in
    Debug|debug)
      bundle_configuration="Debug"
      cargo_profile="debug"
      ;;
    Release|release)
      bundle_configuration="Release"
      cargo_profile="release"
      ;;
    *)
      echo "unsupported CONFIGURATION: ${requested_configuration}" >&2
      exit 1
      ;;
  esac
}

require_command cargo
require_command git
require_command python3
require_command /usr/libexec/PlistBuddy
require_command xcrun

configure_build_lane
prepare_macos_build_env

bundle_root="${platform_root}/.derived-data/Build/Products/${bundle_configuration}/${bundle_name}"
contents_root="${bundle_root}/Contents"
executable_root="${contents_root}/MacOS"
resources_root="${contents_root}/Resources"
plist_path="${contents_root}/Info.plist"
binary_target="${executable_root}/Radroots"
app_icon_path="${resources_root}/AppIcon.icns"

(
  cd "${repo_root}"
  if [[ "${cargo_profile}" == "release" ]]; then
    cargo build -p radroots_studio_app --release
  else
    cargo build -p radroots_studio_app
  fi
)

binary_source="$(cargo_target_dir)/${cargo_profile}/radroots_studio_app"

if [[ ! -x "${binary_source}" ]]; then
  echo "missing desktop launcher binary: ${binary_source}" >&2
  exit 1
fi

rm -rf "${bundle_root}"
mkdir -p "${executable_root}" "${resources_root}"
cp "${plist_template}" "${plist_path}"
cp "${binary_source}" "${binary_target}"
chmod +x "${binary_target}"
"${script_dir}/build_icon.sh" "${app_icon_path}"

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $(workspace_version)" "${plist_path}"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${RADROOTS_APP_BUILD:-1}" "${plist_path}"

printf '%s\n' "${bundle_root}"
