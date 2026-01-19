#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppRoutePathParts {
    pub path: String,
    pub query: String,
    pub hash: String,
}

pub fn parse_route_path(route_path: &str) -> RadrootsAppRoutePathParts {
    let query_idx = route_path.find('?');
    let hash_idx = route_path.find('#');
    let path_end = match (query_idx, hash_idx) {
        (Some(query_idx), Some(hash_idx)) => query_idx.min(hash_idx),
        (Some(query_idx), None) => query_idx,
        (None, Some(hash_idx)) => hash_idx,
        (None, None) => route_path.len(),
    };
    let path = route_path[..path_end].to_string();
    let query = query_idx
        .map(|start| {
            let end = hash_idx.unwrap_or(route_path.len());
            route_path[start..end].to_string()
        })
        .unwrap_or_default();
    let hash = hash_idx
        .map(|start| route_path[start..].to_string())
        .unwrap_or_default();
    RadrootsAppRoutePathParts { path, query, hash }
}

fn has_file_extension(route_path: &str, file_exts: &[&str]) -> bool {
    let lower_path = route_path.to_ascii_lowercase();
    file_exts.iter().any(|ext| lower_path.ends_with(ext))
}

pub fn resolve_route_path(
    route_path: Option<&str>,
    file_name: &str,
    default_route_path: &str,
    file_exts: &[&str],
) -> String {
    let resolved_route_path = route_path.unwrap_or(default_route_path);
    let parts = parse_route_path(resolved_route_path);
    let mut normalized_path = parts.path.as_str();
    if normalized_path.ends_with('/') {
        normalized_path = &normalized_path[..normalized_path.len().saturating_sub(1)];
    }
    if normalized_path.is_empty() {
        return resolved_route_path.to_string();
    }
    if normalized_path == file_name
        || normalized_path.ends_with(&format!("/{file_name}"))
        || has_file_extension(normalized_path, file_exts)
    {
        return format!("{}{}{}", normalized_path, parts.query, parts.hash);
    }
    format!(
        "{}/{}{}{}",
        normalized_path, file_name, parts.query, parts.hash
    )
}

pub fn resolve_wasm_path(
    wasm_path: Option<&str>,
    wasm_file: &str,
    default_wasm_path: &str,
) -> String {
    resolve_route_path(wasm_path, wasm_file, default_wasm_path, &[".wasm"])
}

#[cfg(test)]
mod tests {
    use super::{parse_route_path, resolve_route_path, resolve_wasm_path};

    #[test]
    fn parse_route_path_splits_parts() {
        let parts = parse_route_path("assets/app.js?cache=1#hash");
        assert_eq!(parts.path, "assets/app.js");
        assert_eq!(parts.query, "?cache=1");
        assert_eq!(parts.hash, "#hash");
    }

    #[test]
    fn resolve_route_path_appends_file_name() {
        let path = resolve_route_path(Some("assets"), "app.js", "/app.js", &[".js"]);
        assert_eq!(path, "assets/app.js");
    }

    #[test]
    fn resolve_route_path_keeps_file_path() {
        let path = resolve_route_path(Some("assets/app.js"), "app.js", "/app.js", &[".js"]);
        assert_eq!(path, "assets/app.js");
    }

    #[test]
    fn resolve_wasm_path_defaults_to_wasm_extension() {
        let path = resolve_wasm_path(Some("pkg"), "app_bg.wasm", "/app_bg.wasm");
        assert_eq!(path, "pkg/app_bg.wasm");
    }
}
