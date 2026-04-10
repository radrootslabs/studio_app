use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "macos" && target_os != "ios" {
        return;
    }

    let package_dir = swift_package_dir();
    println!(
        "cargo:rerun-if-changed={}",
        package_dir.join("Package.swift").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        package_dir.join("Sources").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        package_dir.join("Tests").display()
    );

    let ffi_library = "libRadRootsAppleSecurityFFIDynamic.dylib";
    run_swift_build(package_dir.as_path(), "RadRootsAppleSecurityFFIDynamic");

    let build_dir = find_library_dir(package_dir.join(".build"), ffi_library)
        .expect("swift ffi library dir");
    let swift_runtime_dir = swift_runtime_dir(target_os.as_str());
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        swift_runtime_dir.display()
    );
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", build_dir.display());
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        swift_runtime_dir.display()
    );
    println!("cargo:rustc-link-lib=dylib=RadRootsAppleSecurityFFIDynamic");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=LocalAuthentication");
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=dylib=objc");
}

fn swift_package_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("../../../native/apple/swift/RadRootsAppleSecurity")
}

fn swift_runtime_dir(target_os: &str) -> PathBuf {
    let swift_bin = run_stdout(Command::new("xcrun").arg("--toolchain").arg("swift").arg("--find").arg("swift"));
    let swift_bin = PathBuf::from(swift_bin.trim());
    let toolchain_dir = swift_bin
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("swift toolchain dir");
    find_swift_runtime_dir(toolchain_dir.join("usr/lib"), target_os).expect("swift runtime dir")
}

fn find_swift_runtime_dir(root: PathBuf, target_os: &str) -> Option<PathBuf> {
    let platform_dir = match target_os {
        "macos" => "macosx",
        "ios" => "iphoneos",
        other => other,
    };
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().is_some_and(|name| name == "libswift_Concurrency.dylib")
                && path
                    .components()
                    .any(|component| component.as_os_str() == platform_dir)
            {
                return path.parent().map(Path::to_path_buf);
            }
        }
    }
    None
}

fn find_library_dir(root: PathBuf, library_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().is_some_and(|name| name == library_name) {
                return path.parent().map(Path::to_path_buf);
            }
        }
    }
    None
}

fn run_swift_build(package_dir: &Path, product: &str) {
    let output = Command::new("swift")
        .arg("build")
        .arg("--product")
        .arg(product)
        .current_dir(package_dir)
        .output()
        .expect("failed to run swift build");

    if output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!(
        "swift build --product {product} failed in {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
        package_dir.display()
    );
}

fn run_stdout(command: &mut Command) -> String {
    let output = command.output().expect("failed to run command");
    if output.status.success() {
        return String::from_utf8(output.stdout).expect("utf-8 stdout");
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("command failed: {command:?}\nstderr:\n{stderr}");
}
