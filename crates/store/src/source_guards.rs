use std::{
    fs,
    path::{Path, PathBuf},
};

const WORKSPACE_SQLX_DEPENDENCY: &str = r#"sqlx = { version = "0.9.0", default-features = false, features = ["derive", "sqlite-bundled"] }"#;
const WORKSPACE_LIBSQLITE3_PATCH: &str =
    r#"libsqlite3-sys = { path = "../lib/crates/libsqlite3_sys_3_53_3" }"#;

#[test]
fn app_sqlite_runtime_uses_sqlx_bundled_sqlite_only() {
    let app_root = app_root();
    let workspace_manifest = read_source(app_root.join("Cargo.toml").as_path());

    assert!(
        workspace_manifest.contains(WORKSPACE_SQLX_DEPENDENCY),
        "workspace SQLx dependency must stay pinned to SQLx 0.9.0 with sqlite-bundled"
    );
    assert!(
        workspace_manifest.contains(WORKSPACE_LIBSQLITE3_PATCH),
        "workspace must keep the approved SQLite 3.53.3 libsqlite3-sys patch"
    );

    let findings = sqlite_runtime_drift_findings(app_root.as_path());
    assert!(
        findings.is_empty(),
        "app SQLite runtime drift findings:\n{}",
        findings.join("\n")
    );
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
