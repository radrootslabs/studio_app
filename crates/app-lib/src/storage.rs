#![forbid(unsafe_code)]

use crate::path::{normalize_path, sanitize_path, trim_slashes};

pub fn fmt_id_from_path(pathname: &str, raw_id: Option<&str>) -> String {
    let trimmed = trim_slashes(pathname);
    let prefix = normalize_path(&trimmed);
    let suffix = raw_id
        .map(|id| format!("-{}", sanitize_path(id)))
        .unwrap_or_default();
    format!("*{prefix}{suffix}")
}

pub fn fmt_id(raw_id: Option<&str>) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let location = window.location();
        let pathname = location.pathname().ok()?;
        Some(fmt_id_from_path(&pathname, raw_id))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = raw_id;
        None
    }
}

pub fn build_storage_key_with_prefix(prefix: &str, raw_id: &str, base_prefix: &str) -> String {
    let mut output = format!("{prefix}-{}", sanitize_path(raw_id));
    let base_prefix = normalize_path(&trim_slashes(base_prefix));
    if base_prefix.is_empty() {
        return output;
    }
    let base = format!("*{base_prefix}");
    let base_with_dash = format!("{base}-");
    if output.starts_with(&base_with_dash) {
        output.replace_range(..base_with_dash.len(), "*");
    } else if output.starts_with(&base) {
        output.replace_range(..base.len(), "*");
    }
    output
}

pub fn build_storage_key(raw_id: &str, base_prefix: &str) -> Option<String> {
    let prefix = fmt_id(None)?;
    Some(build_storage_key_with_prefix(&prefix, raw_id, base_prefix))
}

#[cfg(test)]
mod tests {
    use super::{build_storage_key_with_prefix, fmt_id_from_path};

    #[test]
    fn fmt_id_from_path_formats_prefix() {
        assert_eq!(fmt_id_from_path("/app/home", None), "*app-home");
        assert_eq!(fmt_id_from_path("/app/home", Some("id")), "*app-home-id");
    }

    #[test]
    fn build_storage_key_with_prefix_replaces_base_prefix() {
        let key = build_storage_key_with_prefix("*app-home", "raw", "/app");
        assert_eq!(key, "*home-raw");
    }
}
