#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaResource {
    pub base_url: String,
    pub hash: String,
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaImageUploadResult {
    pub base_url: String,
    pub file_hash: String,
    pub file_ext: String,
}

pub fn fmt_media_image_upload_result_url(result: &MediaImageUploadResult) -> String {
    format!(
        "{}/{}.{}",
        result.base_url, result.file_hash, result.file_ext
    )
}

#[cfg(test)]
mod tests {
    use super::{fmt_media_image_upload_result_url, MediaImageUploadResult};

    #[test]
    fn fmt_media_url_builds_path() {
        let result = MediaImageUploadResult {
            base_url: "https://example.com".to_string(),
            file_hash: "hash".to_string(),
            file_ext: "png".to_string(),
        };
        assert_eq!(
            fmt_media_image_upload_result_url(&result),
            "https://example.com/hash.png"
        );
    }
}
