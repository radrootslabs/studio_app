use async_trait::async_trait;
#[cfg(target_arch = "wasm32")]
use std::str::FromStr;
#[cfg(target_arch = "wasm32")]
use base64::engine::general_purpose::STANDARD;
#[cfg(target_arch = "wasm32")]
use base64::Engine as _;
#[cfg(target_arch = "wasm32")]
use radroots_nostr::prelude::{
    RadrootsNostrEventBuilder,
    RadrootsNostrKeys,
    RadrootsNostrSecretKey,
};
#[cfg(target_arch = "wasm32")]
use serde::Deserialize;
use url::Url;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
use serde_json::Value;
#[cfg(target_arch = "wasm32")]
use crate::crypto::random::fill_random;

use super::{
    RadrootsClientMediaImageUpload,
    RadrootsClientMediaResource,
    RadrootsClientRadroots,
    RadrootsClientRadrootsAccountsActivate,
    RadrootsClientRadrootsAccountsCreate,
    RadrootsClientRadrootsAccountsRequest,
    RadrootsClientRadrootsError,
    RadrootsClientRadrootsResult,
};

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct MediaResourceWire {
    base_url: String,
    hash: String,
    ext: String,
}

pub struct RadrootsClientWebRadroots {
    base_url: Option<String>,
}

impl RadrootsClientWebRadroots {
    pub fn new(base_url: Option<&str>) -> Self {
        let base_url = base_url.and_then(sanitize_base_url);
        Self { base_url }
    }

    pub fn get_base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn require_base_url(&self) -> RadrootsClientRadrootsResult<&str> {
        self.base_url
            .as_deref()
            .ok_or(RadrootsClientRadrootsError::MissingBaseUrl)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_x_nostr_event(
        &self,
        secret_key: &str,
    ) -> RadrootsClientRadrootsResult<String> {
        let secret_key = RadrootsNostrSecretKey::from_str(secret_key)
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        let keys = RadrootsNostrKeys::new(secret_key);
        let content = random_content()?;
        let event = RadrootsNostrEventBuilder::text_note(content)
            .sign_with_keys(&keys)
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        serde_json::to_string(&event)
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)
    }

    #[cfg(target_arch = "wasm32")]
    async fn send_json(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        body: Option<Value>,
    ) -> RadrootsClientRadrootsResult<Option<Value>> {
        let window = web_sys::window().ok_or(RadrootsClientRadrootsError::RequestFailure)?;
        let init = web_sys::RequestInit::new();
        init.set_method(method);
        let header_map = web_sys::Headers::new()
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        for (key, value) in headers {
            header_map
                .set(&key, &value)
                .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        }
        if let Some(body) = body {
            let body = serde_json::to_string(&body)
                .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
            init.set_body(&JsValue::from_str(&body));
        }
        init.set_headers(&header_map);
        let request = web_sys::Request::new_with_str_and_init(url, &init)
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        let response: web_sys::Response = response
            .dyn_into()
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        if !response.ok() {
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        parse_response(response).await
    }

    #[cfg(target_arch = "wasm32")]
    async fn send_bytes(
        &self,
        url: &str,
        method: &str,
        headers: Vec<(String, String)>,
        body: &[u8],
    ) -> RadrootsClientRadrootsResult<Option<Value>> {
        let window = web_sys::window().ok_or(RadrootsClientRadrootsError::RequestFailure)?;
        let init = web_sys::RequestInit::new();
        init.set_method(method);
        let header_map = web_sys::Headers::new()
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        for (key, value) in headers {
            header_map
                .set(&key, &value)
                .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        }
        let bytes = js_sys::Uint8Array::from(body);
        init.set_body(&bytes.into());
        init.set_headers(&header_map);
        let request = web_sys::Request::new_with_str_and_init(url, &init)
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        let response: web_sys::Response = response
            .dyn_into()
            .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
        if !response.ok() {
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        parse_response(response).await
    }
}

#[async_trait(?Send)]
impl RadrootsClientRadroots for RadrootsClientWebRadroots {
    async fn accounts_request(
        &self,
        opts: RadrootsClientRadrootsAccountsRequest,
    ) -> RadrootsClientRadrootsResult<String> {
        let _ = self.require_base_url()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let base_url = self.require_base_url()?;
            let url = format!("{base_url}/v1/accounts/request");
            let event = self.create_x_nostr_event(&opts.secret_key)?;
            let headers = vec![
                ("X-Nostr-Event".to_string(), event),
                ("Content-Type".to_string(), "application/json".to_string()),
            ];
            let body = serde_json::json!({ "profile_name": opts.profile_name });
            let data = self.send_json(&url, "POST", headers, Some(body)).await?;
            if let Some(data) = data {
                if is_pass_response(&data) {
                    if let Some(tok) = string_field(&data, "tok") {
                        return Ok(tok);
                    }
                }
            }
            Err(RadrootsClientRadrootsError::AccountRegistered)
        }
    }

    async fn accounts_create(
        &self,
        opts: RadrootsClientRadrootsAccountsCreate,
    ) -> RadrootsClientRadrootsResult<String> {
        let _ = self.require_base_url()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let base_url = self.require_base_url()?;
            let url = format!("{base_url}/v1/accounts/create");
            let event = self.create_x_nostr_event(&opts.secret_key)?;
            let token = encode_bearer_token(&opts.tok);
            let headers = vec![
                ("X-Nostr-Event".to_string(), event),
                ("Authorization".to_string(), format!("Bearer {token}")),
            ];
            let data = self.send_json(&url, "POST", headers, None).await?;
            if let Some(data) = data {
                if is_pass_response(&data) {
                    if let Some(id) = string_field(&data, "id") {
                        return Ok(id);
                    }
                }
            }
            Err(RadrootsClientRadrootsError::RequestFailure)
        }
    }

    async fn accounts_activate(
        &self,
        opts: RadrootsClientRadrootsAccountsActivate,
    ) -> RadrootsClientRadrootsResult<String> {
        let _ = self.require_base_url()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let base_url = self.require_base_url()?;
            let url = format!("{base_url}/v1/accounts/activate");
            let event = self.create_x_nostr_event(&opts.secret_key)?;
            let headers = vec![
                ("X-Nostr-Event".to_string(), event),
                ("Content-Type".to_string(), "application/json".to_string()),
            ];
            let body = serde_json::json!({ "id": opts.id });
            let data = self.send_json(&url, "POST", headers, Some(body)).await?;
            if let Some(data) = data {
                if is_pass_response(&data) {
                    if let Some(id) = string_field(&data, "id") {
                        return Ok(id);
                    }
                }
            }
            Err(RadrootsClientRadrootsError::RequestFailure)
        }
    }

    async fn media_image_upload(
        &self,
        opts: RadrootsClientMediaImageUpload,
    ) -> RadrootsClientRadrootsResult<RadrootsClientMediaResource> {
        let _ = self.require_base_url()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return Err(RadrootsClientRadrootsError::RequestFailure);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let base_url = self.require_base_url()?;
            let url = format!("{base_url}/v1/media/image/upload");
            let event = self.create_x_nostr_event(&opts.secret_key)?;
            let mime_type = opts
                .mime_type
                .unwrap_or_else(|| String::from("image/png"));
            let headers = vec![
                ("X-Nostr-Event".to_string(), event),
                ("Content-Type".to_string(), mime_type),
            ];
            let data = self
                .send_bytes(&url, "PUT", headers, &opts.file_data)
                .await?;
            if let Some(data) = data {
                if is_pass_response(&data) {
                    if let Some(resource) = parse_media_resource(&data) {
                        return Ok(resource);
                    }
                }
            }
            Err(RadrootsClientRadrootsError::RequestFailure)
        }
    }
}

fn sanitize_base_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    let base = format!("{}{}", parsed.origin().ascii_serialization(), parsed.path());
    Some(base.trim_end_matches('/').to_string())
}

#[cfg(target_arch = "wasm32")]
fn random_content() -> RadrootsClientRadrootsResult<String> {
    let mut bytes = [0u8; 16];
    fill_random(&mut bytes).map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
    Ok(STANDARD.encode(bytes))
}

#[cfg(target_arch = "wasm32")]
fn is_pass_response(value: &Value) -> bool {
    matches!(value.get("pass"), Some(Value::Bool(true)))
}

#[cfg(target_arch = "wasm32")]
fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| value.as_str()).map(|v| v.to_string())
}

#[cfg(target_arch = "wasm32")]
fn parse_media_resource(value: &Value) -> Option<RadrootsClientMediaResource> {
    let resource: MediaResourceWire = serde_json::from_value(value.clone()).ok()?;
    Some(RadrootsClientMediaResource {
        base_url: resource.base_url,
        hash: resource.hash,
        ext: resource.ext,
    })
}

#[cfg(target_arch = "wasm32")]
fn encode_bearer_token(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[cfg(target_arch = "wasm32")]
async fn parse_response(
    response: web_sys::Response,
) -> RadrootsClientRadrootsResult<Option<Value>> {
    let json_response = response.json();
    if let Ok(json_response) = json_response {
        if let Ok(value) = JsFuture::from(json_response).await {
            if let Ok(value) = serde_wasm_bindgen::from_value::<Value>(value) {
                return Ok(Some(value));
            }
        }
    }
    let text_response = response.text().map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
    let text_value = JsFuture::from(text_response)
        .await
        .map_err(|_| RadrootsClientRadrootsError::RequestFailure)?;
    if let Some(text) = text_value.as_string() {
        if let Ok(value) = serde_json::from_str(&text) {
            return Ok(Some(value));
        }
        return Ok(Some(Value::String(text)));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebRadroots;
    use crate::radroots::{
        RadrootsClientRadroots,
        RadrootsClientRadrootsAccountsRequest,
        RadrootsClientRadrootsError,
    };

    #[test]
    fn base_url_sanitizes_trailing_slash() {
        let client = RadrootsClientWebRadroots::new(Some("https://example.com/app/"));
        assert_eq!(client.get_base_url(), Some("https://example.com/app"));
    }

    #[test]
    fn missing_base_url_errors() {
        let client = RadrootsClientWebRadroots::new(None);
        let err = futures::executor::block_on(client.accounts_request(
            RadrootsClientRadrootsAccountsRequest {
                profile_name: "rad".to_string(),
                secret_key: "deadbeef".to_string(),
            },
        ))
        .expect_err("missing base url");
        assert_eq!(err, RadrootsClientRadrootsError::MissingBaseUrl);
    }
}
