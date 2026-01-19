#![forbid(unsafe_code)]

use std::fmt;

use radroots_studio_app_utils::types::{FilePath, FilePathBlob, WebFilePath};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileError {
    WindowUnavailable,
    DocumentUnavailable,
    ElementUnavailable,
    BlobFailure,
    UrlFailure,
    SerializeFailure,
    ReadFailure,
    ParseFailure,
    EmptyFile,
    PickerFailure,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::WindowUnavailable => f.write_str("error.app.file.window_unavailable"),
            FileError::DocumentUnavailable => f.write_str("error.app.file.document_unavailable"),
            FileError::ElementUnavailable => f.write_str("error.app.file.element_unavailable"),
            FileError::BlobFailure => f.write_str("error.app.file.blob_failure"),
            FileError::UrlFailure => f.write_str("error.app.file.url_failure"),
            FileError::SerializeFailure => f.write_str("error.app.file.serialize_failure"),
            FileError::ReadFailure => f.write_str("error.app.file.read_failure"),
            FileError::ParseFailure => f.write_str("error.app.file.parse_failure"),
            FileError::EmptyFile => f.write_str("error.app.file.empty_file"),
            FileError::PickerFailure => f.write_str("error.app.file.picker_failure"),
        }
    }
}

impl std::error::Error for FileError {}

pub fn parse_file_path(file_path: &str) -> Option<WebFilePath> {
    if file_path.starts_with("blob:") {
        let blob_name = file_path.replace("blob:", "").replace("http://", "");
        return Some(WebFilePath::Blob(FilePathBlob {
            blob_path: file_path.to_string(),
            blob_name,
            mime_type: None,
        }));
    }
    let file_path_file = file_path.rsplit('/').next().unwrap_or("");
    let mut parts = file_path_file.split('.');
    let file_name = parts.next().unwrap_or("");
    let mime_type = parts.next().unwrap_or("");
    if file_name.is_empty() || mime_type.is_empty() {
        return None;
    }
    Some(WebFilePath::File(FilePath {
        file_path: file_path.to_string(),
        file_name: file_name.to_string(),
        mime_type: mime_type.to_string(),
    }))
}

pub fn download_json<T: Serialize>(data: &T, filename: &str) -> Result<(), FileError> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let json = serde_json::to_string_pretty(data).map_err(|_| FileError::SerializeFailure)?;
        let array = js_sys::Array::new();
        array.push(&wasm_bindgen::JsValue::from_str(&json));
        let blob = web_sys::Blob::new_with_str_sequence(&array).map_err(|_| FileError::BlobFailure)?;
        let url =
            web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| FileError::UrlFailure)?;
        let window = web_sys::window().ok_or(FileError::WindowUnavailable)?;
        let document = window.document().ok_or(FileError::DocumentUnavailable)?;
        let anchor = document
            .create_element("a")
            .map_err(|_| FileError::ElementUnavailable)?;
        let anchor: web_sys::HtmlAnchorElement =
            anchor.dyn_into().map_err(|_| FileError::ElementUnavailable)?;
        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();
        web_sys::Url::revoke_object_url(&url).map_err(|_| FileError::UrlFailure)?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = data;
        let _ = filename;
        Err(FileError::WindowUnavailable)
    }
}

pub async fn select_file() -> Result<Option<web_sys::File>, FileError> {
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::JsCast;

        let window = web_sys::window().ok_or(FileError::WindowUnavailable)?;
        let document = window.document().ok_or(FileError::DocumentUnavailable)?;
        let input = document
            .create_element("input")
            .map_err(|_| FileError::ElementUnavailable)?;
        let input: web_sys::HtmlInputElement =
            input.dyn_into().map_err(|_| FileError::ElementUnavailable)?;
        input.set_type("file");
        input.set_accept("*/*");

        let (sender, receiver) = futures::channel::oneshot::channel();
        let closure_holder: Rc<RefCell<Option<wasm_bindgen::closure::Closure<dyn FnMut(_)>>>> =
            Rc::new(RefCell::new(None));
        let closure_ref = closure_holder.clone();
        let input_clone = input.clone();
        *closure_holder.borrow_mut() = Some(wasm_bindgen::closure::Closure::wrap(Box::new(
            move |_event: web_sys::Event| {
                let file = input_clone.files().and_then(|list| list.get(0));
                let _ = sender.send(file);
                closure_ref.borrow_mut().take();
            },
        ) as Box<dyn FnMut(_)>));
        if let Some(closure) = closure_holder.borrow().as_ref() {
            input.set_onchange(Some(closure.as_ref().unchecked_ref()));
        }
        input.click();
        receiver.await.map_err(|_| FileError::PickerFailure)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Err(FileError::WindowUnavailable)
    }
}

pub async fn get_file_text(file: Option<web_sys::File>) -> Result<Option<String>, FileError> {
    let Some(file) = file else {
        return Ok(None);
    };
    #[cfg(target_arch = "wasm32")]
    {
        let text = wasm_bindgen_futures::JsFuture::from(file.text())
            .await
            .map_err(|_| FileError::ReadFailure)?;
        Ok(text.as_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = file;
        Err(FileError::WindowUnavailable)
    }
}

pub async fn parse_file_json<T: DeserializeOwned>(
    file: Option<web_sys::File>,
) -> Result<T, FileError> {
    let contents = get_file_text(file).await?;
    let Some(contents) = contents else {
        return Err(FileError::EmptyFile);
    };
    if contents.is_empty() {
        return Err(FileError::EmptyFile);
    }
    serde_json::from_str(&contents).map_err(|_| FileError::ParseFailure)
}

#[cfg(test)]
mod tests {
    use super::{get_file_text, parse_file_path, FileError};

    #[test]
    fn parse_file_path_handles_blob_paths() {
        let parsed = parse_file_path("blob:http://example").expect("parsed");
        match parsed {
            radroots_studio_app_utils::types::WebFilePath::Blob(blob) => {
                assert_eq!(blob.blob_name, "example");
            }
            _ => panic!("expected blob"),
        }
    }

    #[test]
    fn parse_file_path_handles_files() {
        let parsed = parse_file_path("/path/file.txt").expect("parsed");
        match parsed {
            radroots_studio_app_utils::types::WebFilePath::File(file) => {
                assert_eq!(file.file_name, "file");
                assert_eq!(file.mime_type, "txt");
            }
            _ => panic!("expected file"),
        }
    }

    #[test]
    fn get_file_text_none_returns_none() {
        let result = futures::executor::block_on(get_file_text(None)).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn parse_file_json_errors_without_file() {
        let err = futures::executor::block_on(super::parse_file_json::<serde_json::Value>(None))
            .expect_err("err");
        assert_eq!(err, FileError::EmptyFile);
    }
}
