#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "$script_dir/../../.." && pwd -P)"

source_db="$app_root/assets/geocoder/geonames.db"
source_revision="$app_root/assets/geocoder/geonames.revision"
target_dir="${TARGET_BUILD_DIR}/${UNLOCALIZED_RESOURCES_FOLDER_PATH}"
target_db="$target_dir/geonames.db"
target_revision="$target_dir/geonames.revision"

mkdir -p "$target_dir"

if [[ -f "$source_db" ]]; then
  if [[ ! -f "$source_revision" ]]; then
    printf 'stamped ios geocoder revision asset missing at build time: %s\n' "$source_revision" >&2
    exit 1
  fi
  cp "$source_db" "$target_db"
  cp "$source_revision" "$target_revision"
  printf 'synced ios geocoder asset: %s\n' "$target_db"
  printf 'synced ios geocoder revision: %s\n' "$target_revision"
  exit 0
fi

if [[ -f "$target_db" ]]; then
  rm -f "$target_db"
fi
if [[ -f "$target_revision" ]]; then
  rm -f "$target_revision"
fi

printf 'ios geocoder asset not present at build time: %s\n' "$source_db"
