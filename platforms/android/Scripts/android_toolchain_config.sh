#!/usr/bin/env bash

android_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
android_dir="$(cd "${android_script_dir}/.." && pwd -P)"
app_root="$(cd "${android_dir}/../.." && pwd -P)"

android_tooling_dir="${android_dir}/.tooling"
android_download_dir="${android_tooling_dir}/downloads"
android_sdk_dir="${android_tooling_dir}/android-sdk"
android_gradle_user_home="${android_tooling_dir}/gradle-user-home"
android_user_home="${android_tooling_dir}/android-user-home"
android_emulator_home="${android_tooling_dir}/emulator-home"
android_avd_home="${android_tooling_dir}/avd"
android_cargo_install_root="${android_tooling_dir}/cargo"
android_cargo_bin_dir="${android_cargo_install_root}/bin"
android_local_properties_path="${android_dir}/local.properties"

android_sdk_api_level="34"
android_build_tools_version="34.0.0"
android_ndk_version="26.1.10909125"
android_gradle_version="8.7"
android_gradle_distribution_sha256="544c35d6bd849ae8a5ed0bcea39ba677dc40f49df7d1835561582da2009b961d"
android_cmdline_tools_version="14742923"
android_cargo_ndk_version="4.1.2"

android_gradle_home="${android_tooling_dir}/gradle/gradle-${android_gradle_version}"
android_gradle_bin="${android_gradle_home}/bin/gradle"
android_sdkmanager_bin="${android_sdk_dir}/cmdline-tools/latest/bin/sdkmanager"
android_avdmanager_bin="${android_sdk_dir}/cmdline-tools/latest/bin/avdmanager"
android_ndk_dir="${android_sdk_dir}/ndk/${android_ndk_version}"
android_emulator_bin="${android_sdk_dir}/emulator/emulator"
android_adb_bin="${android_sdk_dir}/platform-tools/adb"
android_cargo_ndk_bin="${android_cargo_bin_dir}/cargo-ndk"

android_rust_target="aarch64-linux-android"
android_abi="arm64-v8a"

android_sdk_packages=(
  "platform-tools"
  "platforms;android-${android_sdk_api_level}"
  "build-tools;${android_build_tools_version}"
  "ndk;${android_ndk_version}"
)

android_gradle_distribution_url() {
  echo "https://services.gradle.org/distributions/gradle-${android_gradle_version}-bin.zip"
}

android_cmdline_tools_platform() {
  case "$(uname -s)" in
    Darwin)
      echo "mac"
      ;;
    Linux)
      echo "linux"
      ;;
    *)
      echo "unsupported"
      ;;
  esac
}

android_cmdline_tools_zip_name() {
  local platform_name
  platform_name="$(android_cmdline_tools_platform)"
  echo "commandlinetools-${platform_name}-${android_cmdline_tools_version}_latest.zip"
}

android_cmdline_tools_url() {
  echo "https://dl.google.com/android/repository/$(android_cmdline_tools_zip_name)"
}

android_emulator_system_image_package() {
  echo "system-images;android-${android_sdk_api_level};google_apis;${android_abi}"
}

android_emulator_packages() {
  printf '%s\n' "emulator" "$(android_emulator_system_image_package)"
}

android_avd_name() {
  echo "RadRoots_API_${android_sdk_api_level}"
}
