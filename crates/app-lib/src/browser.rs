#![forbid(unsafe_code)]

#[cfg(any(test, target_arch = "wasm32"))]
use once_cell::sync::Lazy;
#[cfg(any(test, target_arch = "wasm32"))]
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserPlatformInfo {
    pub os: String,
    pub browser: String,
    pub version: String,
}

#[cfg(any(test, target_arch = "wasm32"))]
static REMOVE_EXCESS_MOZILLA_AND_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^mozilla/\d\.\d\W").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static BROWSER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+)/(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)").expect("regex")
});
#[cfg(any(test, target_arch = "wasm32"))]
static ENGINE_AND_VERSION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(ver|cri|gec)").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static VERSION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"version/(\d+(\.\d+)*)").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static MOBILE_OS_IPHONE: Lazy<Regex> = Lazy::new(|| Regex::new("iphone").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static MOBILE_OS_IPAD: Lazy<Regex> = Lazy::new(|| Regex::new("ipad|macintosh").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static MOBILE_OS_ANDROID: Lazy<Regex> = Lazy::new(|| Regex::new("android").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static DESKTOP_OS_WINDOWS: Lazy<Regex> = Lazy::new(|| Regex::new("win").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static DESKTOP_OS_MAC: Lazy<Regex> = Lazy::new(|| Regex::new("macintosh").expect("regex"));
#[cfg(any(test, target_arch = "wasm32"))]
static DESKTOP_OS_LINUX: Lazy<Regex> = Lazy::new(|| Regex::new("linux").expect("regex"));

pub fn browser_platform() -> Option<BrowserPlatformInfo> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let navigator = window.navigator();
        let ua = navigator.user_agent().ok()?;
        let max_touch_points = navigator.max_touch_points();
        return Some(parse_user_agent_string(&ua, max_touch_points));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn parse_user_agent_string(ua_string: &str, max_touch_points: i32) -> BrowserPlatformInfo {
    let ua = REMOVE_EXCESS_MOZILLA_AND_VERSION
        .replace(&ua_string.to_lowercase(), "")
        .to_string();

    let mobile_os = if MOBILE_OS_IPHONE.is_match(&ua) && max_touch_points >= 1 {
        Some("iphone")
    } else if MOBILE_OS_IPAD.is_match(&ua) && max_touch_points >= 1 {
        Some("ipad")
    } else if MOBILE_OS_ANDROID.is_match(&ua) && max_touch_points >= 1 {
        Some("android")
    } else {
        None
    };
    let desktop_os = if DESKTOP_OS_WINDOWS.is_match(&ua) {
        Some("windows")
    } else if DESKTOP_OS_MAC.is_match(&ua) {
        Some("mac")
    } else if DESKTOP_OS_LINUX.is_match(&ua) {
        Some("linux")
    } else {
        None
    };
    let os = mobile_os.or(desktop_os).unwrap_or("");

    let browser_matches = BROWSER_PATTERN
        .find_iter(&ua)
        .map(|capture| capture.as_str().to_string())
        .collect::<Vec<_>>();
    let safari_version = VERSION_PATTERN
        .captures(&ua)
        .and_then(|caps| caps.get(1).map(|match_value| match_value.as_str().to_string()));
    let browser_offset = if browser_matches.len() > 2 {
        browser_matches
            .get(1)
            .map(|match_value| !ENGINE_AND_VERSION_PATTERN.is_match(match_value))
            .unwrap_or(false)
    } else {
        false
    };
    let browser_index = browser_matches
        .len()
        .saturating_sub(1 + if browser_offset { 1 } else { 0 });
    let (browser, version) = browser_matches
        .get(browser_index)
        .and_then(|match_value| {
            let mut parts = match_value.split('/');
            let browser = parts.next().unwrap_or("").to_string();
            let version = parts.next().unwrap_or("").to_string();
            Some((browser, version))
        })
        .unwrap_or_else(|| (String::new(), String::new()));
    let version = safari_version.unwrap_or(version);

    BrowserPlatformInfo {
        os: os.to_string(),
        browser,
        version,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_user_agent_string, BrowserPlatformInfo};

    #[test]
    fn parse_user_agent_detects_browser() {
        let info = parse_user_agent_string(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            0,
        );
        assert_eq!(
            info,
            BrowserPlatformInfo {
                os: "mac".to_string(),
                browser: "chrome".to_string(),
                version: "120.0.0.0".to_string()
            }
        );
    }
}
