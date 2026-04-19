use std::env;
use std::path::{Path, PathBuf};

use mf2_i18n::build::{NativeModuleBuildOptions, build_native_module};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo manifest dir should exist"));
    let app_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("app root should be discoverable from shared i18n crate");
    let config_path = app_root.join("i18n").join("mf2_i18n.toml");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir should exist"));

    let build_output = build_native_module(&NativeModuleBuildOptions::new(
        &config_path,
        &out_dir,
        "app_i18n",
    ))
    .unwrap_or_else(|error| {
        panic!(
            "failed to build app i18n native module from {}: {error}",
            config_path.display()
        )
    });

    for path in build_output.rerun_if_changed_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
