#![forbid(unsafe_code)]

pub fn trim_slashes(path: &str) -> String {
    path.trim_matches('/').to_string()
}

pub fn normalize_path(path: &str) -> String {
    let mut output = String::with_capacity(path.len());
    for ch in path.chars() {
        let mapped = match ch {
            '-' => '_',
            '/' => '-',
            _ => ch,
        };
        if mapped == '-' && output.ends_with('-') {
            continue;
        }
        output.push(mapped);
    }
    output
}

pub fn sanitize_path(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_path, sanitize_path, trim_slashes};

    #[test]
    fn trim_slashes_removes_edge_slashes() {
        assert_eq!(trim_slashes("/a/b/"), "a/b");
        assert_eq!(trim_slashes("///a///"), "a");
    }

    #[test]
    fn normalize_path_replaces_chars() {
        assert_eq!(normalize_path("a-b/c"), "a_b-c");
        assert_eq!(normalize_path("a//b"), "a-b");
    }

    #[test]
    fn sanitize_path_strips_invalid_chars() {
        assert_eq!(sanitize_path("ab/c$%"), "abc");
        assert_eq!(sanitize_path("a_b-1"), "a_b-1");
    }
}
