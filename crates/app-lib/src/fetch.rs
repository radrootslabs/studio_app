#![forbid(unsafe_code)]

use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchJsonErrorKind {
    Http,
    Network,
    Parse,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchJsonError {
    pub kind: FetchJsonErrorKind,
    pub url: String,
    pub message: String,
    pub status: Option<u16>,
    pub status_text: Option<String>,
}

pub type FetchJsonResult<T> = Result<T, FetchJsonError>;

impl FetchJsonError {
    pub fn http(url: &str, status: u16, status_text: Option<String>) -> Self {
        let message = status_text
            .clone()
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "http_error".to_string());
        Self {
            kind: FetchJsonErrorKind::Http,
            url: url.to_string(),
            message,
            status: Some(status),
            status_text,
        }
    }

    pub fn network(url: &str, message: Option<String>) -> Self {
        let message = message.filter(|text| !text.is_empty())
            .unwrap_or_else(|| "network_error".to_string());
        Self {
            kind: FetchJsonErrorKind::Network,
            url: url.to_string(),
            message,
            status: None,
            status_text: None,
        }
    }

    pub fn parse(url: &str, message: Option<String>) -> Self {
        let message = message.filter(|text| !text.is_empty())
            .unwrap_or_else(|| "parse_error".to_string());
        Self {
            kind: FetchJsonErrorKind::Parse,
            url: url.to_string(),
            message,
            status: None,
            status_text: None,
        }
    }

    pub fn unavailable(url: &str) -> Self {
        Self {
            kind: FetchJsonErrorKind::Unavailable,
            url: url.to_string(),
            message: "fetch_unavailable".to_string(),
            status: None,
            status_text: None,
        }
    }
}

pub async fn fetch_json<T>(url: &str) -> FetchJsonResult<T>
where
    T: DeserializeOwned,
{
    #[cfg(target_arch = "wasm32")]
    {
        fetch_json_wasm(url).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Err(FetchJsonError::unavailable(url))
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json_wasm<T>(url: &str) -> FetchJsonResult<T>
where
    T: DeserializeOwned,
{
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| FetchJsonError::unavailable(url))?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|err| FetchJsonError::network(url, js_error_message(err)))?;
    let response: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| FetchJsonError::network(url, Some("network_error".to_string())))?;
    if !response.ok() {
        let status_text = response.status_text();
        return Err(FetchJsonError::http(
            url,
            response.status(),
            if status_text.is_empty() { None } else { Some(status_text) },
        ));
    }
    let json_promise = response
        .json()
        .map_err(|err| FetchJsonError::parse(url, js_error_message(err)))?;
    let json_value = wasm_bindgen_futures::JsFuture::from(json_promise)
        .await
        .map_err(|err| FetchJsonError::parse(url, js_error_message(err)))?;
    serde_wasm_bindgen::from_value(json_value)
        .map_err(|err| FetchJsonError::parse(url, Some(err.to_string())))
}

#[cfg(target_arch = "wasm32")]
fn js_error_message(err: wasm_bindgen::JsValue) -> Option<String> {
    err.as_string().filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{fetch_json, FetchJsonError, FetchJsonErrorKind};

    #[derive(Debug, serde::Deserialize)]
    struct DummyPayload {
        #[serde(rename = "value")]
        _value: String,
    }

    #[test]
    fn fetch_json_http_error_sets_fields() {
        let err = FetchJsonError::http("https://example", 404, Some("Not Found".to_string()));
        assert_eq!(err.kind, FetchJsonErrorKind::Http);
        assert_eq!(err.url, "https://example");
        assert_eq!(err.status, Some(404));
        assert_eq!(err.status_text.as_deref(), Some("Not Found"));
    }

    #[test]
    fn fetch_json_network_error_defaults_message() {
        let err = FetchJsonError::network("https://example", None);
        assert_eq!(err.kind, FetchJsonErrorKind::Network);
        assert_eq!(err.message, "network_error");
    }

    #[test]
    fn non_wasm_fetch_is_unavailable() {
        let err = futures::executor::block_on(fetch_json::<DummyPayload>("https://example"))
            .expect_err("unavailable");
        assert_eq!(err.kind, FetchJsonErrorKind::Unavailable);
        assert_eq!(err.url, "https://example");
    }
}
