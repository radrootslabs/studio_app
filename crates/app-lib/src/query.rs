#![forbid(unsafe_code)]

pub fn encode_query_params<K: AsRef<str>, V: AsRef<str>>(params: &[(K, V)]) -> String {
    let mut output = String::new();
    for (key, value) in params {
        let key = key.as_ref().trim();
        let value = value.as_ref().trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push('&');
        }
        output.push_str(key);
        output.push('=');
        for part in url::form_urlencoded::byte_serialize(value.as_bytes()) {
            for ch in part.chars() {
                if ch == '+' {
                    output.push_str("%20");
                } else {
                    output.push(ch);
                }
            }
        }
    }
    if output.is_empty() {
        String::new()
    } else {
        format!("?{output}")
    }
}

pub fn encode_route<K: AsRef<str>, V: AsRef<str>>(route: &str, params: &[(K, V)]) -> String {
    let query = encode_query_params(params);
    if query.is_empty() {
        return route.to_string();
    }
    let base = if route == "/" {
        route.to_string()
    } else {
        let trimmed = route.trim_end_matches('/');
        if trimmed.is_empty() {
            "/".to_string()
        } else {
            trimmed.to_string()
        }
    };
    format!("{base}{query}")
}

#[cfg(test)]
mod tests {
    use super::{encode_query_params, encode_route};

    #[test]
    fn encode_query_params_skips_empty_entries() {
        let params = [("a", "b c"), ("", "skip"), ("c", "")];
        assert_eq!(encode_query_params(&params), "?a=b%20c");
    }

    #[test]
    fn encode_route_appends_query() {
        let params = [("q", "1")];
        assert_eq!(encode_route("/path/", &params), "/path?q=1");
        assert_eq!(encode_route("/", &params), "/?q=1");
    }

    #[test]
    fn encode_route_preserves_route_without_params() {
        let params: [(&str, &str); 0] = [];
        assert_eq!(encode_route("/path/", &params), "/path/");
    }
}
