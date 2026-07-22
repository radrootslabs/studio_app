use std::{
    fs,
    path::{Path, PathBuf},
};

const WORKSPACE_SQLX_DEPENDENCY: &str = r#"sqlx = { version = "0.9.0", default-features = false, features = ["derive", "sqlite-bundled"] }"#;
const LIBSQLITE3_SYS_VERSION: &str = "0.37.0";
const LIBSQLITE3_SYS_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";
const LIBSQLITE3_SYS_CHECKSUM: &str =
    "b1f111c8c41e7c61a49cd34e44c7619462967221a6443b0ec299e0ac30cfb9b1";

#[test]
fn app_sqlite_runtime_uses_sqlx_bundled_sqlite_only() {
    let app_root = app_root();
    let workspace_manifest = read_source(app_root.join("Cargo.toml").as_path());

    assert!(
        workspace_manifest.contains(WORKSPACE_SQLX_DEPENDENCY),
        "workspace SQLx dependency must stay pinned to SQLx 0.9.0 with sqlite-bundled"
    );

    let manifest: toml::Value =
        toml::from_str(workspace_manifest.as_str()).expect("workspace Cargo.toml should parse");
    let libsqlite3_patch = manifest
        .get("patch")
        .and_then(|patch| patch.get("crates-io"))
        .and_then(|crates_io| crates_io.get("libsqlite3-sys"));
    assert!(
        libsqlite3_patch.is_none(),
        "workspace must resolve libsqlite3-sys from crates.io without a source override"
    );

    let lockfile = read_source(app_root.join("Cargo.lock").as_path());
    let lockfile_findings = libsqlite3_lock_findings(lockfile.as_str());
    assert!(
        lockfile_findings.is_empty(),
        "app SQLite lockfile findings:\n{}",
        lockfile_findings.join("\n")
    );

    let findings = sqlite_runtime_drift_findings(app_root.as_path());
    assert!(
        findings.is_empty(),
        "app SQLite runtime drift findings:\n{}",
        findings.join("\n")
    );
}

fn libsqlite3_lock_findings(lockfile: &str) -> Vec<String> {
    let lock: toml::Value = match toml::from_str(lockfile) {
        Ok(lock) => lock,
        Err(error) => return vec![format!("Cargo.lock is not valid TOML: {error}")],
    };
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let libsqlite3_packages = packages
        .iter()
        .filter(|package| {
            package.get("name").and_then(toml::Value::as_str) == Some("libsqlite3-sys")
        })
        .collect::<Vec<_>>();

    if libsqlite3_packages.len() != 1 {
        return vec![format!(
            "Cargo.lock must contain exactly one libsqlite3-sys package, found {}",
            libsqlite3_packages.len()
        )];
    }

    let package = libsqlite3_packages[0];
    let expected_fields = [
        ("version", LIBSQLITE3_SYS_VERSION),
        ("source", LIBSQLITE3_SYS_SOURCE),
        ("checksum", LIBSQLITE3_SYS_CHECKSUM),
    ];
    expected_fields
        .into_iter()
        .filter_map(|(field, expected)| {
            let actual = package.get(field).and_then(toml::Value::as_str);
            (actual != Some(expected)).then(|| {
                format!(
                    "libsqlite3-sys {field} must be `{expected}`, found `{}`",
                    actual.unwrap_or("<missing>")
                )
            })
        })
        .collect()
}

fn sqlite_runtime_drift_findings(app_root: &Path) -> Vec<String> {
    sqlite_guard_paths(app_root)
        .into_iter()
        .flat_map(|path| {
            let source = read_source(path.as_path());
            let relative_path = path
                .strip_prefix(app_root)
                .expect("guard path should be app-relative")
                .to_string_lossy()
                .replace('\\', "/");
            forbidden_sqlite_findings(relative_path.as_str(), source.as_str())
        })
        .collect()
}

fn forbidden_sqlite_findings(path: &str, source: &str) -> Vec<String> {
    let mut findings = Vec::new();

    for pattern in ["rusqlite", "SqliteExecutor"] {
        for line in token_match_lines(source, pattern) {
            findings.push(format!(
                "{path}:{line} contains forbidden SQLite runtime token `{pattern}`"
            ));
        }
    }

    for pattern in [
        "libsqlite3-sys = { path =",
        "sqlite-wasm-rs",
        "rsqlite-vfs",
        "bundled-sqlcipher",
        "features = [\"bundled\"]",
        "features = [\"bundled-full\"]",
        "features = [\"bundled-sqlcipher\"]",
    ] {
        for line in literal_match_lines(source, pattern) {
            findings.push(format!(
                "{path}:{line} contains forbidden SQLite runtime literal `{pattern}`"
            ));
        }
    }

    let lowercase_source = source.to_lowercase();
    for line in literal_match_lines(lowercase_source.as_str(), "sqlcipher") {
        findings.push(format!(
            "{path}:{line} contains forbidden SQLCipher runtime literal"
        ));
    }

    findings
}

#[test]
fn sqlite_lock_guard_rejects_local_or_ambiguous_sources() {
    let valid = format!(
        r#"[[package]]
name = "libsqlite3-sys"
version = "{LIBSQLITE3_SYS_VERSION}"
source = "{LIBSQLITE3_SYS_SOURCE}"
checksum = "{LIBSQLITE3_SYS_CHECKSUM}"
"#
    );
    assert!(libsqlite3_lock_findings(valid.as_str()).is_empty());

    let local = format!(
        r#"[[package]]
name = "libsqlite3-sys"
version = "{LIBSQLITE3_SYS_VERSION}"
"#
    );
    assert_eq!(libsqlite3_lock_findings(local.as_str()).len(), 2);

    let duplicate = format!("{valid}\n{valid}");
    assert_eq!(libsqlite3_lock_findings(duplicate.as_str()).len(), 1);
}

fn sqlite_guard_paths(app_root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![app_root.join("Cargo.toml"), app_root.join("Cargo.lock")];
    collect_guard_paths(app_root.join("crates").as_path(), &mut paths);
    paths.sort();
    paths
}

fn collect_guard_paths(root: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!("failed to read guard directory {}: {error}", root.display())
    }) {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to inspect guard directory {}: {error}",
                root.display()
            )
        });
        let path = entry.path();

        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("target") {
                collect_guard_paths(path.as_path(), paths);
            }
            continue;
        }

        if path.file_name().and_then(|name| name.to_str()) == Some("source_guards.rs") {
            continue;
        }

        if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("rs" | "toml" | "lock")
        ) {
            paths.push(path);
        }
    }
}

fn token_match_lines(source: &str, pattern: &str) -> Vec<usize> {
    source
        .match_indices(pattern)
        .filter_map(|(index, _)| {
            let before = source[..index].chars().next_back();
            let after = source[index + pattern.len()..].chars().next();

            if before.is_some_and(is_rust_identifier_character)
                || after.is_some_and(is_rust_identifier_character)
            {
                None
            } else {
                Some(line_number(source, index))
            }
        })
        .collect()
}

fn literal_match_lines(source: &str, pattern: &str) -> Vec<usize> {
    source
        .match_indices(pattern)
        .map(|(index, _)| line_number(source, index))
        .collect()
}

fn read_source(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read source {}: {error}", path.display()))
}

fn app_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("store crate should live under app crates directory")
        .to_path_buf()
}

fn is_rust_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn line_number(source: &str, index: usize) -> usize {
    source[..index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
