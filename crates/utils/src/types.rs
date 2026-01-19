#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;

pub type ResolveError<T> = Result<T, RadrootsAppUtilsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileBytesFormat {
    Kb,
    Mb,
    Gb,
}

pub type FileMimeType = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePath {
    pub file_path: String,
    pub file_name: String,
    pub mime_type: FileMimeType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePathBlob {
    pub blob_path: String,
    pub blob_name: String,
    pub mime_type: Option<FileMimeType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebFilePath {
    File(FilePath),
    Blob(FilePathBlob),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdbClientConfig {
    pub database: String,
    pub store: String,
}

pub type ValStr = Option<String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultPass {
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultId {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultObj<T> {
    pub result: T,
}

pub type ResultBool = ResultObj<bool>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultsList<T> {
    pub results: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultPublicKey {
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSecretKey {
    pub secret_key: String,
}

impl ResultPass {
    pub const fn ok() -> Self {
        Self { pass: true }
    }
}

pub fn resolve_ok<T>(value: T) -> ResolveError<T> {
    Ok(value)
}

pub fn resolve_err<T>(err: RadrootsAppUtilsError) -> ResolveError<T> {
    Err(err)
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_err, resolve_ok, FileBytesFormat, FilePath, FilePathBlob, IdbClientConfig,
        ResultBool, ResultId, ResultObj, ResultPass, ResultPublicKey, ResultSecretKey, ResultsList,
        ValStr, WebFilePath,
    };
    use crate::error::RadrootsAppUtilsError;

    #[test]
    fn result_pass_is_true() {
        let pass = ResultPass::ok();
        assert!(pass.pass);
    }

    #[test]
    fn resolve_ok_returns_value() {
        let value = resolve_ok(5).expect("value");
        assert_eq!(value, 5);
    }

    #[test]
    fn resolve_err_returns_error() {
        let err = resolve_err::<()>(RadrootsAppUtilsError::Unavailable)
            .expect_err("err");
        assert_eq!(err, RadrootsAppUtilsError::Unavailable);
    }

    #[test]
    fn result_types_store_values() {
        let id = ResultId {
            id: "id".to_string(),
        };
        assert_eq!(id.id, "id");
        let obj = ResultObj { result: 5 };
        assert_eq!(obj.result, 5);
        let bool_obj: ResultBool = ResultObj { result: true };
        assert!(bool_obj.result);
        let list = ResultsList {
            results: vec![1, 2],
        };
        assert_eq!(list.results, vec![1, 2]);
        let public_key = ResultPublicKey {
            public_key: "pub".to_string(),
        };
        assert_eq!(public_key.public_key, "pub");
        let secret_key = ResultSecretKey {
            secret_key: "sec".to_string(),
        };
        assert_eq!(secret_key.secret_key, "sec");
    }

    #[test]
    fn file_path_types_store_values() {
        let path = FilePath {
            file_path: "path".to_string(),
            file_name: "name".to_string(),
            mime_type: "text/plain".to_string(),
        };
        let blob = FilePathBlob {
            blob_path: "blob".to_string(),
            blob_name: "blob.bin".to_string(),
            mime_type: None,
        };
        let file_path = WebFilePath::File(path.clone());
        let blob_path = WebFilePath::Blob(blob.clone());
        assert_eq!(file_path, WebFilePath::File(path));
        assert_eq!(blob_path, WebFilePath::Blob(blob));
        assert_eq!(FileBytesFormat::Kb, FileBytesFormat::Kb);
    }

    #[test]
    fn idb_config_stores_values() {
        let config = IdbClientConfig {
            database: "db".to_string(),
            store: "store".to_string(),
        };
        assert_eq!(config.database, "db");
        assert_eq!(config.store, "store");
        let value: ValStr = None;
        assert!(value.is_none());
    }
}
