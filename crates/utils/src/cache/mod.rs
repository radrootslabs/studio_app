#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;

pub const RADROOTS_ASSET_CACHE_NAME: &str = "cache-app-assets-v1";
pub const RADROOTS_ASSET_CACHE_PREFIX: &str = "cache-app-assets-v";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetCacheMode {
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

#[derive(Debug, Clone)]
pub struct AssetCacheRequestInit {
    pub cache: Option<AssetCacheMode>,
}

#[derive(Debug, Clone)]
pub struct AssetCacheFetchConfig {
    pub cache_name: Option<String>,
    pub request_init: Option<AssetCacheRequestInit>,
}

#[cfg(target_arch = "wasm32")]
pub type AssetResponse = web_sys::Response;

#[cfg(not(target_arch = "wasm32"))]
pub type AssetResponse = ();

pub type AssetBytes = Vec<u8>;

pub async fn asset_cache_fetch(
    url: &str,
    config: Option<&AssetCacheFetchConfig>,
) -> Result<AssetResponse, RadrootsAppUtilsError> {
    asset_cache_fetch_impl(url, config).await
}

pub async fn asset_cache_fetch_bytes(
    url: &str,
    config: Option<&AssetCacheFetchConfig>,
) -> Result<Option<AssetBytes>, RadrootsAppUtilsError> {
    #[cfg(target_arch = "wasm32")]
    {
        let response = asset_cache_fetch(url, config).await?;
        if !response.ok() {
            return Ok(None);
        }
        let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer())
            .await
            .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
        let array = js_sys::Uint8Array::new(&buffer);
        let mut bytes = vec![0u8; array.length() as usize];
        array.copy_to(&mut bytes);
        Ok(Some(bytes))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = url;
        let _ = config;
        Err(RadrootsAppUtilsError::Unavailable)
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn cache_name_resolve(config: Option<&AssetCacheFetchConfig>) -> String {
    config
        .and_then(|config| config.cache_name.as_ref().cloned())
        .unwrap_or_else(|| RADROOTS_ASSET_CACHE_NAME.to_string())
}

#[cfg(any(test, target_arch = "wasm32"))]
fn cache_key_resolve(url: &str) -> String {
    url.split('#').next().unwrap_or(url).to_string()
}

#[cfg(target_arch = "wasm32")]
async fn asset_cache_fetch_impl(
    url: &str,
    config: Option<&AssetCacheFetchConfig>,
) -> Result<AssetResponse, RadrootsAppUtilsError> {
    use wasm_bindgen::JsCast;

    let cache_name = cache_name_resolve(config);
    let cache_key = cache_key_resolve(url);
    if let Some(cached) = cache_read(&cache_name, &cache_key).await? {
        return Ok(cached);
    }
    let response = fetch_with_init(url, config).await?;
    if response.ok() || response.type_() == web_sys::ResponseType::Opaque {
        cache_write(&cache_name, &cache_key, response.clone()).await?;
    }
    Ok(response)
}

#[cfg(not(target_arch = "wasm32"))]
async fn asset_cache_fetch_impl(
    _url: &str,
    _config: Option<&AssetCacheFetchConfig>,
) -> Result<AssetResponse, RadrootsAppUtilsError> {
    Err(RadrootsAppUtilsError::Unavailable)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_with_init(
    url: &str,
    config: Option<&AssetCacheFetchConfig>,
) -> Result<AssetResponse, RadrootsAppUtilsError> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or(RadrootsAppUtilsError::Unavailable)?;
    let mut init = web_sys::RequestInit::new();
    if let Some(request_init) = config.and_then(|config| config.request_init.as_ref()) {
        if let Some(cache_mode) = request_init.cache {
            init.cache(cache_mode.to_request_cache());
        }
    }
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(url, &init))
        .await
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    response
        .dyn_into::<web_sys::Response>()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)
}

#[cfg(target_arch = "wasm32")]
async fn cache_read(
    cache_name: &str,
    cache_key: &str,
) -> Result<Option<AssetResponse>, RadrootsAppUtilsError> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or(RadrootsAppUtilsError::Unavailable)?;
    let storage = window
        .caches()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let cache = wasm_bindgen_futures::JsFuture::from(storage.open(cache_name))
        .await
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let cache = cache
        .dyn_into::<web_sys::Cache>()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let cached = wasm_bindgen_futures::JsFuture::from(cache.match_with_str(cache_key))
        .await
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    if cached.is_undefined() || cached.is_null() {
        return Ok(None);
    }
    let response = cached
        .dyn_into::<web_sys::Response>()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    Ok(Some(response))
}

#[cfg(target_arch = "wasm32")]
async fn cache_write(
    cache_name: &str,
    cache_key: &str,
    response: AssetResponse,
) -> Result<(), RadrootsAppUtilsError> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or(RadrootsAppUtilsError::Unavailable)?;
    let storage = window
        .caches()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let cache = wasm_bindgen_futures::JsFuture::from(storage.open(cache_name))
        .await
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let cache = cache
        .dyn_into::<web_sys::Cache>()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    wasm_bindgen_futures::JsFuture::from(cache.put_with_str(cache_key, &response))
        .await
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    Ok(())
}

impl AssetCacheMode {
    #[cfg(target_arch = "wasm32")]
    fn to_request_cache(self) -> web_sys::RequestCache {
        match self {
            AssetCacheMode::Default => web_sys::RequestCache::Default,
            AssetCacheMode::NoStore => web_sys::RequestCache::NoStore,
            AssetCacheMode::Reload => web_sys::RequestCache::Reload,
            AssetCacheMode::NoCache => web_sys::RequestCache::NoCache,
            AssetCacheMode::ForceCache => web_sys::RequestCache::ForceCache,
            AssetCacheMode::OnlyIfCached => web_sys::RequestCache::OnlyIfCached,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cache_key_resolve, cache_name_resolve, AssetCacheFetchConfig, RADROOTS_ASSET_CACHE_NAME};

    #[test]
    fn cache_name_defaults() {
        assert_eq!(cache_name_resolve(None), RADROOTS_ASSET_CACHE_NAME);
    }

    #[test]
    fn cache_name_uses_config() {
        let config = AssetCacheFetchConfig {
            cache_name: Some("custom".to_string()),
            request_init: None,
        };
        assert_eq!(cache_name_resolve(Some(&config)), "custom");
    }

    #[test]
    fn cache_key_strips_hash() {
        assert_eq!(cache_key_resolve("path#hash"), "path");
    }
}
