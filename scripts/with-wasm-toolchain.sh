#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <command> [args...]" >&2
  exit 64
fi

unset NO_COLOR

probe_wasm_clang() {
  local clang_bin="$1"
  local probe_file

  if [ ! -x "$clang_bin" ]; then
    return 1
  fi

  probe_file="$(mktemp)"
  trap 'rm -f "$probe_file"' RETURN
  printf 'int main(void){return 0;}\n' \
    | "$clang_bin" --target=wasm32-unknown-unknown -x c -c - -o "$probe_file" >/dev/null 2>&1
}

if [ -z "${CC_wasm32_unknown_unknown:-}" ]; then
  if probe_wasm_clang /opt/homebrew/opt/llvm/bin/clang; then
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
    export CC_wasm32_unknown_unknown=/opt/homebrew/opt/llvm/bin/clang
  elif command -v clang >/dev/null 2>&1 && probe_wasm_clang "$(command -v clang)"; then
    export CC_wasm32_unknown_unknown
    CC_wasm32_unknown_unknown="$(command -v clang)"
  else
    echo "no wasm-capable clang found; install llvm or set CC_wasm32_unknown_unknown" >&2
    exit 1
  fi
fi

exec "$@"
