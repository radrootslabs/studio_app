use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const GEOCODER_DB_FILENAME: &str = "geonames.db";
const GEOCODER_REVISION_FILENAME: &str = "geonames.revision";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    sync_optional_geocoder_assets();

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let package_dir = manifest_dir.join("../../native/apple/swift/RadRootsAppleSecurity");
    let info_plist_path = manifest_dir.join("macos/Info.plist");

    emit_rerun_paths(&package_dir);
    println!("cargo:rerun-if-changed={}", info_plist_path.display());

    let configuration = if env::var("PROFILE").ok().as_deref() == Some("release") {
        "release"
    } else {
        "debug"
    };
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("target arch");

    run_swift_build(&package_dir, configuration, &arch);
    let bin_path = swift_bin_path(&package_dir, configuration, &arch);

    let dylib_path = bin_path.join("libRadRootsAppleSecurityFFIDynamic.dylib");
    if !dylib_path.is_file() {
        panic!(
            "swift package did not produce expected dynamic library at {}",
            dylib_path.display()
        );
    }

    println!("cargo:rustc-link-search=native={}", bin_path.display());
    println!("cargo:rustc-link-lib=dylib=RadRootsAppleSecurityFFIDynamic");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=framework=LocalAuthentication");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", bin_path.display());
    println!(
        "cargo:rustc-link-arg-bin=radroots-app-desktop=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        info_plist_path.display()
    );
}

fn sync_optional_geocoder_assets() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source_db_path = manifest_dir.join(format!("../../assets/geocoder/{GEOCODER_DB_FILENAME}"));
    let source_revision_path = manifest_dir.join(format!(
        "../../assets/geocoder/{GEOCODER_REVISION_FILENAME}"
    ));
    println!("cargo:rerun-if-changed={}", source_db_path.display());
    println!("cargo:rerun-if-changed={}", source_revision_path.display());

    let profile_dir = target_profile_dir();
    let target_db_path = profile_dir.join(GEOCODER_DB_FILENAME);
    let target_revision_path = profile_dir.join(GEOCODER_REVISION_FILENAME);

    if source_db_path.is_file() {
        if !source_revision_path.is_file() {
            panic!(
                "stamped desktop geocoder revision asset missing at {}",
                source_revision_path.display()
            );
        }

        std::fs::copy(&source_db_path, &target_db_path).unwrap_or_else(|err| {
            panic!(
                "failed to copy optional desktop geocoder asset from {} to {}: {err}",
                source_db_path.display(),
                target_db_path.display()
            )
        });
        std::fs::copy(&source_revision_path, &target_revision_path).unwrap_or_else(|err| {
            panic!(
                "failed to copy optional desktop geocoder revision from {} to {}: {err}",
                source_revision_path.display(),
                target_revision_path.display()
            )
        });
        return;
    }

    if target_db_path.exists() {
        std::fs::remove_file(&target_db_path).unwrap_or_else(|err| {
            panic!(
                "failed to remove stale desktop geocoder asset at {}: {err}",
                target_db_path.display()
            )
        });
    }
    if target_revision_path.exists() {
        std::fs::remove_file(&target_revision_path).unwrap_or_else(|err| {
            panic!(
                "failed to remove stale desktop geocoder revision at {}: {err}",
                target_revision_path.display()
            )
        });
    }
}

fn target_profile_dir() -> PathBuf {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    out_dir
        .ancestors()
        .nth(3)
        .unwrap_or_else(|| panic!("unexpected cargo OUT_DIR layout: {}", out_dir.display()))
        .to_path_buf()
}

fn emit_rerun_paths(package_dir: &Path) {
    println!(
        "cargo:rerun-if-changed={}",
        package_dir.join("Package.swift").display()
    );
    emit_rerun_dir(&package_dir.join("Sources"));
}

fn emit_rerun_dir(dir: &Path) {
    if !dir.is_dir() {
        return;
    }

    let mut entries = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            emit_rerun_dir(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn run_swift_build(package_dir: &Path, configuration: &str, arch: &str) {
    let status = Command::new("swift")
        .arg("build")
        .arg("--package-path")
        .arg(package_dir)
        .arg("--product")
        .arg("RadRootsAppleSecurityFFIDynamic")
        .arg("--configuration")
        .arg(configuration)
        .arg("--arch")
        .arg(arch)
        .status()
        .unwrap_or_else(|err| panic!("failed to run swift build: {err}"));

    if !status.success() {
        panic!("swift build failed for RadRootsAppleSecurityFFIDynamic");
    }
}

fn swift_bin_path(package_dir: &Path, configuration: &str, arch: &str) -> PathBuf {
    let output = Command::new("swift")
        .arg("build")
        .arg("--package-path")
        .arg(package_dir)
        .arg("--product")
        .arg("RadRootsAppleSecurityFFIDynamic")
        .arg("--configuration")
        .arg(configuration)
        .arg("--arch")
        .arg(arch)
        .arg("--show-bin-path")
        .output()
        .unwrap_or_else(|err| panic!("failed to resolve swift bin path: {err}"));

    if !output.status.success() {
        panic!(
            "swift build --show-bin-path failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("swift bin path utf-8")
            .trim(),
    )
}
