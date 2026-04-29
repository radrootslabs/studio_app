#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"

cd "${repo_root}"
cargo metadata --format-version 1 --no-deps
cargo test -p radroots_studio_app_models pack_day
cargo test -p radroots_studio_app_state pack_day
cargo test -p radroots_studio_app_i18n pack_day
cargo test -p radroots_studio_app pack_day
cargo test -p radroots_studio_app source_guards
cargo check -p radroots_studio_app

if [[ "$(uname -s)" == "Darwin" ]]; then
  "${repo_root}/platforms/macos/Scripts/test_host.sh"
fi
