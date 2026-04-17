use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mf2_i18n_build::compiler::compile_message;
use mf2_i18n_build::id_map::{IdMap, build_id_map};
use mf2_i18n_build::pack_encode::{PackBuildInput, encode_pack};
use mf2_i18n_build::parser::parse_message;
use mf2_i18n_core::PackKind;
use serde::Deserialize;

type Catalog = BTreeMap<String, String>;

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    default_locale: String,
    source_dirs: Vec<PathBuf>,
    project_salt_path: PathBuf,
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo manifest dir should exist"));
    let app_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("app root should be discoverable from shared i18n crate");
    let i18n_root = app_root.join("i18n");
    let config_path = i18n_root.join("mf2-i18n.toml");
    println!("cargo:rerun-if-changed={}", config_path.display());

    let config = load_project_config(&config_path);
    let salt_path = i18n_root.join(&config.project_salt_path);
    println!("cargo:rerun-if-changed={}", salt_path.display());
    let id_salt = load_id_salt(&salt_path);

    let catalogs = load_catalogs(&i18n_root, &config.source_dirs);
    let default_catalog = catalogs
        .get(&config.default_locale)
        .unwrap_or_else(|| {
            panic!(
                "default locale {} catalog should exist",
                config.default_locale
            )
        })
        .clone();
    ensure_catalog_keys_match(&catalogs);

    let id_map = build_id_map(
        default_catalog.keys().cloned().collect::<Vec<_>>(),
        &id_salt,
    )
    .expect("id map should build");

    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("out dir should exist")).join("app_i18n");
    fs::create_dir_all(&out_dir).expect("i18n out dir should be created");

    write_id_map(&out_dir, &id_map);
    for (locale, catalog) in &catalogs {
        write_pack(&out_dir, locale, catalog, &id_map);
    }

    let locale_ids = catalogs.keys().cloned().collect::<Vec<_>>();
    let default_catalog_keys = default_catalog.keys().cloned().collect::<Vec<_>>();
    write_generated_runtime(
        &out_dir,
        &config.default_locale,
        &locale_ids,
        &default_catalog_keys,
    );
}

fn load_project_config(path: &Path) -> ProjectConfig {
    let raw = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "failed to read i18n project config {}: {error}",
            path.display()
        )
    });
    toml::from_str(&raw).unwrap_or_else(|error| {
        panic!(
            "failed to parse i18n project config {}: {error}",
            path.display()
        )
    })
}

fn load_id_salt(path: &Path) -> Vec<u8> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read id salt {}: {error}", path.display()));
    let salt = raw.trim();
    assert!(
        !salt.is_empty(),
        "i18n id salt {} must not be empty",
        path.display()
    );
    salt.as_bytes().to_vec()
}

fn load_catalogs(i18n_root: &Path, source_dirs: &[PathBuf]) -> BTreeMap<String, Catalog> {
    assert!(
        !source_dirs.is_empty(),
        "i18n project config must declare at least one source dir"
    );

    let mut catalogs = BTreeMap::<String, Catalog>::new();
    for source_dir in source_dirs {
        let source_root = i18n_root.join(source_dir);
        println!("cargo:rerun-if-changed={}", source_root.display());
        let entries = fs::read_dir(&source_root).unwrap_or_else(|error| {
            panic!(
                "failed to read source dir {}: {error}",
                source_root.display()
            )
        });

        for entry in entries {
            let entry = entry.unwrap_or_else(|error| {
                panic!(
                    "failed to read source dir entry under {}: {error}",
                    source_root.display()
                )
            });
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let locale = entry.file_name().to_string_lossy().into_owned();
            let messages_path = path.join("messages.json");
            if !messages_path.is_file() {
                continue;
            }
            println!("cargo:rerun-if-changed={}", messages_path.display());

            let catalog = load_catalog(&messages_path);
            let merged = catalogs.entry(locale.clone()).or_default();
            for (key, value) in catalog {
                assert!(
                    merged.insert(key.clone(), value).is_none(),
                    "duplicate i18n message key {key} in locale {locale} from {}",
                    messages_path.display()
                );
            }
        }
    }

    assert!(
        !catalogs.is_empty(),
        "at least one locale catalog is required"
    );
    catalogs
}

fn load_catalog(path: &Path) -> Catalog {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read i18n catalog {}: {error}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse i18n catalog {}: {error}", path.display()))
}

fn ensure_catalog_keys_match(catalogs: &BTreeMap<String, Catalog>) {
    let Some((reference_locale, reference_catalog)) = catalogs.iter().next() else {
        panic!("at least one i18n catalog is required");
    };

    let reference_keys = reference_catalog.keys().cloned().collect::<Vec<_>>();
    for (locale, catalog) in catalogs.iter().skip(1) {
        let keys = catalog.keys().cloned().collect::<Vec<_>>();
        assert_eq!(
            keys, reference_keys,
            "i18n catalog keys for locale {locale} do not match reference locale {reference_locale}"
        );
    }
}

fn write_id_map(out_dir: &Path, id_map: &IdMap) {
    let entries = id_map
        .entries()
        .map(|(key, id)| (key.to_owned(), u32::from(id)))
        .collect::<BTreeMap<_, _>>();
    let id_map_json = serde_json::to_vec_pretty(&entries).expect("id map json should serialize");
    fs::write(out_dir.join("id-map.json"), id_map_json).expect("id map json should write");

    let hash = id_map.hash().expect("id map hash should build");
    let hash_text = format!("sha256:{}\n", hex::encode(hash));
    fs::write(out_dir.join("id-map.sha256"), hash_text).expect("id map hash should write");
}

fn write_pack(out_dir: &Path, locale: &str, catalog: &Catalog, id_map: &IdMap) {
    let id_map_hash = id_map.hash().expect("id map hash should build");
    let mut messages = BTreeMap::new();

    for (key, source) in catalog {
        let parsed = parse_message(source).unwrap_or_else(|error| {
            panic!("failed to parse i18n message for locale {locale} key {key}: {error:?}")
        });
        let compiled = compile_message(&parsed).unwrap_or_else(|error| {
            panic!("failed to compile i18n message for locale {locale} key {key}: {error}")
        });
        let message_id = id_map
            .get(key)
            .unwrap_or_else(|| panic!("missing message id for locale {locale} key {key}"));
        messages.insert(message_id, compiled.program);
    }

    let pack_bytes = encode_pack(&PackBuildInput {
        pack_kind: PackKind::Base,
        id_map_hash,
        locale_tag: locale.to_owned(),
        parent_tag: None,
        build_epoch_ms: 0,
        messages,
    });

    fs::write(out_dir.join(format!("{locale}.mf2pack")), pack_bytes)
        .unwrap_or_else(|error| panic!("failed to write i18n pack for locale {locale}: {error}"));
}

fn write_generated_runtime(
    out_dir: &Path,
    default_locale: &str,
    locale_ids: &[String],
    default_catalog_keys: &[String],
) {
    let packs_source = locale_ids
        .iter()
        .map(|locale| {
            format!(
                "            ({locale:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/app_i18n/{locale}.mf2pack\"))),"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let runtime_source = format!(
        "mod generated {{\n    mf2_i18n_native::define_i18n_module! {{\n        init_policy: strict,\n        default_locale: {default_locale:?},\n        id_map_json: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/app_i18n/id-map.json\")),\n        id_map_hash: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/app_i18n/id-map.sha256\")),\n        packs: [\n{packs_source}\n        ],\n    }}\n}}\n"
    );
    fs::write(out_dir.join("generated_module.rs"), runtime_source)
        .expect("generated runtime module should write");

    let supported_locale_values = locale_ids
        .iter()
        .map(|locale| format!("{locale:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let key_values = default_catalog_keys
        .iter()
        .map(|key| format!("{key:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let catalog_source = format!(
        "const DEFAULT_LOCALE_ID: &str = {default_locale:?};\nconst SUPPORTED_LOCALE_IDS: &[&str] = &[{supported_locale_values}];\n#[cfg(test)] const DEFAULT_CATALOG_KEY_IDS: &[&str] = &[{key_values}];\n"
    );
    fs::write(out_dir.join("generated_catalog.rs"), catalog_source)
        .expect("generated catalog metadata should write");
}
