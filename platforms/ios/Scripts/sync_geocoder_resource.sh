#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
app_root="$(cd "$script_dir/../../.." && pwd -P)"

source_db="$app_root/assets/geocoder/geonames.db"
target_dir="${TARGET_BUILD_DIR}/${UNLOCALIZED_RESOURCES_FOLDER_PATH}"
target_db="$target_dir/geonames.db"

mkdir -p "$target_dir"

if [[ -f "$source_db" ]]; then
  cp "$source_db" "$target_db"
  printf 'synced ios geocoder asset: %s\n' "$target_db"
  exit 0
fi

if [[ -f "$target_db" ]]; then
  rm -f "$target_db"
fi

printf 'ios geocoder asset not present at build time: %s\n' "$source_db"
