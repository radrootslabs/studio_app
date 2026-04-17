#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "${script_dir}" rev-parse --show-toplevel)"

cd "${repo_root}"

if [[ "$(uname -s)" == "Darwin" ]]; then
  exec "${repo_root}/platforms/macos/Scripts/run-macos-host.sh" "$@"
fi

exec cargo run -p radroots_studio_app -- "$@"
