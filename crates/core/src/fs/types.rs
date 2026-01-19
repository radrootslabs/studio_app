use async_trait::async_trait;

use super::RadrootsClientFsError;

pub type RadrootsClientFsResult<T> = Result<T, RadrootsClientFsError>;
pub type RadrootsClientFsReadBinResult = RadrootsClientFsResult<Vec<u8>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientFsOpenResult {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientFsFileInfo {
    pub size: u64,
    pub is_file: bool,
    pub is_directory: bool,
    pub accessed_at: Option<u64>,
    pub modified_at: Option<u64>,
    pub created_at: Option<u64>,
}

#[async_trait(?Send)]
pub trait RadrootsClientFs {
    async fn exists(&self, path: &str) -> RadrootsClientFsResult<bool>;
    async fn open(&self, path: &str) -> RadrootsClientFsResult<RadrootsClientFsOpenResult>;
    async fn info(&self, path: &str) -> RadrootsClientFsResult<RadrootsClientFsFileInfo>;
    async fn read_bin(&self, path: &str) -> RadrootsClientFsReadBinResult;
}

#[cfg(test)]
mod tests {
    use super::{RadrootsClientFsFileInfo, RadrootsClientFsOpenResult};

    #[test]
    fn file_info_tracks_flags() {
        let info = RadrootsClientFsFileInfo {
            size: 42,
            is_file: true,
            is_directory: false,
            accessed_at: Some(1),
            modified_at: Some(2),
            created_at: None,
        };
        assert!(info.is_file);
        assert!(!info.is_directory);
        assert_eq!(info.size, 42);
        assert_eq!(info.accessed_at, Some(1));
        assert_eq!(info.modified_at, Some(2));
        assert_eq!(info.created_at, None);
    }

    #[test]
    fn open_result_preserves_path() {
        let open = RadrootsClientFsOpenResult {
            path: String::from("path"),
        };
        assert_eq!(open.path, "path");
    }
}
