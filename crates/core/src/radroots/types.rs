use async_trait::async_trait;

use super::RadrootsClientRadrootsError;

pub type RadrootsClientRadrootsResult<T> = Result<T, RadrootsClientRadrootsError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientRadrootsAccountsRequest {
    pub profile_name: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientRadrootsAccountsCreate {
    pub tok: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientRadrootsAccountsActivate {
    pub id: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientMediaResource {
    pub base_url: String,
    pub hash: String,
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientMediaImageUpload {
    pub mime_type: Option<String>,
    pub file_data: Vec<u8>,
    pub secret_key: String,
}

#[async_trait(?Send)]
pub trait RadrootsClientRadroots {
    async fn accounts_request(
        &self,
        opts: RadrootsClientRadrootsAccountsRequest,
    ) -> RadrootsClientRadrootsResult<String>;
    async fn accounts_create(
        &self,
        opts: RadrootsClientRadrootsAccountsCreate,
    ) -> RadrootsClientRadrootsResult<String>;
    async fn accounts_activate(
        &self,
        opts: RadrootsClientRadrootsAccountsActivate,
    ) -> RadrootsClientRadrootsResult<String>;
    async fn media_image_upload(
        &self,
        opts: RadrootsClientMediaImageUpload,
    ) -> RadrootsClientRadrootsResult<RadrootsClientMediaResource>;
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientMediaResource;

    #[test]
    fn media_resource_fields_roundtrip() {
        let resource = RadrootsClientMediaResource {
            base_url: String::from("https://example.com"),
            hash: String::from("hash"),
            ext: String::from("png"),
        };
        assert_eq!(resource.base_url, "https://example.com");
        assert_eq!(resource.hash, "hash");
        assert_eq!(resource.ext, "png");
    }
}
