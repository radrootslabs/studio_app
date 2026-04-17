#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"
superproject_root="$(git -C "${script_dir}" rev-parse --show-superproject-working-tree || true)"
output_path="${1:-}"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_command git
require_command sips
require_command iconutil

if [[ -n "${superproject_root}" && -f "${superproject_root}/logo.png" ]]; then
  source_artwork="${superproject_root}/logo.png"
else
  source_artwork="${repo_root}/logo.png"
fi

if [[ -z "${output_path}" ]]; then
  echo "usage: $0 <output-icns-path>" >&2
  exit 1
fi

if [[ ! -f "${source_artwork}" ]]; then
  echo "missing source artwork: ${source_artwork}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

normalized_png="${tmp_dir}/logo.normalized.png"
iconset_dir="${tmp_dir}/AppIcon.iconset"

mkdir -p "${iconset_dir}" "$(dirname "${output_path}")"
sips -s format png "${source_artwork}" --out "${normalized_png}" >/dev/null

source_width="$(
  sips -g pixelWidth "${normalized_png}" | awk '/pixelWidth/ {print $2}'
)"
source_height="$(
  sips -g pixelHeight "${normalized_png}" | awk '/pixelHeight/ {print $2}'
)"

if [[ "${source_width}" -lt 1024 || "${source_height}" -lt 1024 ]]; then
  printf '%s\n' \
    "warning: macos icon source is ${source_width}x${source_height}; 1024x1024 is recommended for crisp AppIcon.icns output" \
    >&2
fi

generate_icon() {
  local filename="$1"
  local size_px="$2"
  sips -z "${size_px}" "${size_px}" "${normalized_png}" \
    --out "${iconset_dir}/${filename}" >/dev/null
}

generate_icon "icon_16x16.png" 16
generate_icon "icon_16x16@2x.png" 32
generate_icon "icon_32x32.png" 32
generate_icon "icon_32x32@2x.png" 64
generate_icon "icon_128x128.png" 128
generate_icon "icon_128x128@2x.png" 256
generate_icon "icon_256x256.png" 256
generate_icon "icon_256x256@2x.png" 512
generate_icon "icon_512x512.png" 512
generate_icon "icon_512x512@2x.png" 1024

iconutil -c icns "${iconset_dir}" -o "${output_path}"
