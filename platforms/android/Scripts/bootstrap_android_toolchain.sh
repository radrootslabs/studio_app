#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
source "${script_dir}/android_toolchain_config.sh"

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return
  fi
  echo "missing required command: $1" >&2
  exit 1
}

require_java_17() {
  require_command java

  local version_output
  version_output="$(java -version 2>&1 | head -n 1)"
  local java_major
  java_major="$(echo "${version_output}" | sed -E 's/.*version "([0-9]+).*/\1/')"
  if [[ -z "${java_major}" || "${java_major}" -lt 17 ]]; then
    echo "android bootstrap requires java 17 or newer" >&2
    exit 1
  fi
}

checksum_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi

  echo "missing required command: sha256sum or shasum" >&2
  exit 1
}

download_if_missing() {
  local url="$1"
  local destination="$2"

  if [[ -f "${destination}" ]]; then
    return
  fi

  mkdir -p "$(dirname "${destination}")"
  curl -fsSL "${url}" -o "${destination}"
}

validate_zip_archive() {
  unzip -tqq "$1" >/dev/null 2>&1
}

ensure_valid_zip_download() {
  local url="$1"
  local destination="$2"

  download_if_missing "${url}" "${destination}"
  if validate_zip_archive "${destination}"; then
    return
  fi

  rm -f "${destination}"
  download_if_missing "${url}" "${destination}"
  if ! validate_zip_archive "${destination}"; then
    echo "invalid zip archive: ${destination}" >&2
    exit 1
  fi
}

ensure_gradle_distribution() {
  if [[ -x "${android_gradle_bin}" ]]; then
    return
  fi

  local gradle_zip="${android_download_dir}/gradle-${android_gradle_version}-bin.zip"
  ensure_valid_zip_download "$(android_gradle_distribution_url)" "${gradle_zip}"

  local actual_checksum
  actual_checksum="$(checksum_file "${gradle_zip}")"
  if [[ "${actual_checksum}" != "${android_gradle_distribution_sha256}" ]]; then
    rm -f "${gradle_zip}"
    ensure_valid_zip_download "$(android_gradle_distribution_url)" "${gradle_zip}"
    actual_checksum="$(checksum_file "${gradle_zip}")"
    if [[ "${actual_checksum}" != "${android_gradle_distribution_sha256}" ]]; then
      echo "gradle distribution checksum mismatch" >&2
      exit 1
    fi
  fi

  rm -rf "$(dirname "${android_gradle_home}")"
  mkdir -p "$(dirname "${android_gradle_home}")"
  unzip -q "${gradle_zip}" -d "$(dirname "${android_gradle_home}")"
}

ensure_android_cmdline_tools() {
  if [[ -x "${android_sdkmanager_bin}" ]]; then
    return
  fi

  local platform_name
  platform_name="$(android_cmdline_tools_platform)"
  if [[ "${platform_name}" == "unsupported" ]]; then
    echo "android bootstrap supports only darwin and linux hosts" >&2
    exit 1
  fi

  local cmdline_zip="${android_download_dir}/$(android_cmdline_tools_zip_name)"
  local tmp_dir="${android_tooling_dir}/tmp/cmdline-tools"

  ensure_valid_zip_download "$(android_cmdline_tools_url)" "${cmdline_zip}"

  rm -rf "${tmp_dir}" "${android_sdk_dir}/cmdline-tools/latest"
  mkdir -p "${tmp_dir}" "${android_sdk_dir}/cmdline-tools/latest"
  unzip -q "${cmdline_zip}" -d "${tmp_dir}"
  mv "${tmp_dir}/cmdline-tools/"* "${android_sdk_dir}/cmdline-tools/latest/"
  rm -rf "${tmp_dir}"
}

accept_android_licenses() {
  set +o pipefail
  yes | "${android_sdkmanager_bin}" --sdk_root="${android_sdk_dir}" --licenses >/dev/null
  set -o pipefail
}

ensure_android_sdk_packages() {
  accept_android_licenses
  "${android_sdkmanager_bin}" --sdk_root="${android_sdk_dir}" "${android_sdk_packages[@]}"
}

ensure_android_emulator_packages() {
  accept_android_licenses
  local packages=()
  while IFS= read -r package; do
    packages+=("${package}")
  done < <(android_emulator_packages)
  "${android_sdkmanager_bin}" --sdk_root="${android_sdk_dir}" "${packages[@]}"
}

ensure_cargo_ndk() {
  if [[ -x "${android_cargo_ndk_bin}" ]]; then
    local installed_version
    installed_version="$(PATH="${android_cargo_bin_dir}:${PATH}" cargo ndk --version | awk '{print $2}')"
    if [[ "${installed_version}" == "${android_cargo_ndk_version}" ]]; then
      return
    fi
  fi

  cargo install \
    --locked \
    --force \
    --root "${android_cargo_install_root}" \
    --version "${android_cargo_ndk_version}" \
    cargo-ndk
}

ensure_rust_target() {
  if rustup target list --installed | grep -Fx "${android_rust_target}" >/dev/null 2>&1; then
    return
  fi
  rustup target add "${android_rust_target}"
}

write_local_properties() {
  cat <<EOF > "${android_local_properties_path}"
sdk.dir=${android_sdk_dir}
EOF
}

main() {
  local with_emulator="false"
  local print_gradle_bin="false"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --with-emulator)
        with_emulator="true"
        shift
        ;;
      --print-gradle-bin)
        print_gradle_bin="true"
        shift
        ;;
      *)
        echo "unknown bootstrap option: $1" >&2
        exit 1
        ;;
    esac
  done

  require_command curl
  require_command unzip
  require_command cargo
  require_command rustup
  require_java_17

  mkdir -p \
    "${android_tooling_dir}" \
    "${android_download_dir}" \
    "${android_gradle_user_home}" \
    "${android_user_home}" \
    "${android_emulator_home}" \
    "${android_avd_home}" \
    "${android_cargo_install_root}"

  ensure_gradle_distribution
  ensure_android_cmdline_tools
  ensure_android_sdk_packages
  if [[ "${with_emulator}" == "true" ]]; then
    ensure_android_emulator_packages
  fi
  ensure_cargo_ndk
  ensure_rust_target
  write_local_properties

  if [[ "${print_gradle_bin}" == "true" ]]; then
    printf '%s\n' "${android_gradle_bin}"
  fi
}

main "$@"
