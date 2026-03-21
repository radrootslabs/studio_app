#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "${script_dir}/.." && pwd -P)"
ios_root="${app_root}/platforms/ios"
project_name="RadRootsIOS"
configuration="${CONFIGURATION:-Debug}"
derived_data_dir="${ios_root}/.derived-data"
expected_app="${derived_data_dir}/Build/Products/${configuration}-iphonesimulator/${project_name}.app"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

ios_sim_host_arch() {
  case "$(uname -m)" in
    arm64|aarch64)
      echo "arm64"
      ;;
    x86_64)
      echo "x86_64"
      ;;
    *)
      echo "unsupported host architecture for ios simulator: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

require_command xcodegen
require_command xcodebuild

"${script_dir}/check-ios-target.sh"

host_arch="$(ios_sim_host_arch)"

(
  cd "${ios_root}"
  xcodegen generate
  xcodebuild \
    -project "${project_name}.xcodeproj" \
    -scheme "${project_name}" \
    -configuration "${configuration}" \
    -sdk iphonesimulator \
    -destination "generic/platform=iOS Simulator" \
    -derivedDataPath "${derived_data_dir}" \
    ARCHS="${host_arch}" \
    CODE_SIGNING_ALLOWED=YES \
    ONLY_ACTIVE_ARCH=YES \
    build
)

if [[ ! -d "${expected_app}" ]]; then
  echo "missing expected ios app bundle: ${expected_app}" >&2
  exit 1
fi

printf '%s\n' "${expected_app}"
