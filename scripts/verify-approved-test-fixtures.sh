#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if rg -n \
  --glob '*.rs' \
  --glob '*.kt' \
  --glob '*.swift' \
  --glob '*.sh' \
  --glob '!scripts/verify-approved-test-fixtures.sh' \
  'npub1abc|nsec1example' \
  crates native platforms scripts; then
  echo "found banned placeholder fixture literals" >&2
  exit 1
fi
