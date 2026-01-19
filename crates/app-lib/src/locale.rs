#![forbid(unsafe_code)]

const DEFAULT_LOCALE: &str = "en";

pub fn resolve_locale(locales: &[&str], navigator_locale: Option<&str>) -> String {
    let fallback = locales.first().copied().unwrap_or(DEFAULT_LOCALE);
    let fallback_lower = fallback.to_ascii_lowercase();
    let Some(nav_locale) = navigator_locale else {
        return fallback_lower;
    };
    let nav_lower = nav_locale.to_ascii_lowercase();
    if locales
        .iter()
        .any(|locale| locale.eq_ignore_ascii_case(&nav_lower))
    {
        return nav_lower;
    }
    let prefix = nav_lower.chars().take(2).collect::<String>();
    if !prefix.is_empty()
        && locales
            .iter()
            .any(|locale| locale.eq_ignore_ascii_case(&prefix))
    {
        return prefix;
    }
    fallback_lower
}

pub fn get_locale(locales: &[&str]) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let navigator_locale = web_sys::window().map(|window| window.navigator().language());
        resolve_locale(locales, navigator_locale.as_deref())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        resolve_locale(locales, None)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_locale;

    #[test]
    fn resolve_locale_prefers_exact_match() {
        let locales = ["en", "fr"];
        assert_eq!(resolve_locale(&locales, Some("fr")), "fr");
    }

    #[test]
    fn resolve_locale_prefers_prefix_match() {
        let locales = ["en", "fr"];
        assert_eq!(resolve_locale(&locales, Some("fr-CA")), "fr");
    }

    #[test]
    fn resolve_locale_falls_back() {
        let locales = ["en", "fr"];
        assert_eq!(resolve_locale(&locales, Some("es-ES")), "en");
    }
}
