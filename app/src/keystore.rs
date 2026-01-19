#![forbid(unsafe_code)]

use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKeystoreError {
    Keystore(RadrootsClientKeystoreError),
}

pub type AppKeystoreResult<T> = Result<T, AppKeystoreError>;

impl AppKeystoreError {
    pub const fn message(&self) -> &'static str {
        match self {
            AppKeystoreError::Keystore(err) => err.message(),
        }
    }
}

impl std::fmt::Display for AppKeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppKeystoreError {}

impl From<RadrootsClientKeystoreError> for AppKeystoreError {
    fn from(err: RadrootsClientKeystoreError) -> Self {
        AppKeystoreError::Keystore(err)
    }
}

pub async fn app_keystore_nostr_keys<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> AppKeystoreResult<Vec<String>> {
    keystore.keys().await.map_err(AppKeystoreError::from)
}

pub async fn app_keystore_nostr_public_key<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> AppKeystoreResult<Option<String>> {
    match keystore.keys().await {
        Ok(mut keys) => Ok(keys.pop()),
        Err(RadrootsClientKeystoreError::NostrNoResults) => Ok(None),
        Err(err) => Err(AppKeystoreError::from(err)),
    }
}

pub async fn app_keystore_nostr_ensure_key<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> AppKeystoreResult<String> {
    match app_keystore_nostr_public_key(keystore).await? {
        Some(key) => Ok(key),
        None => keystore.generate().await.map_err(AppKeystoreError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_keystore_nostr_ensure_key,
        app_keystore_nostr_public_key,
        app_keystore_nostr_keys,
        AppKeystoreError,
    };
    use async_trait::async_trait;
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
    };

    struct TestKeystore {
        keys_result: RadrootsClientKeystoreResult<Vec<String>>,
        generate_result: RadrootsClientKeystoreResult<String>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientKeystoreNostr for TestKeystore {
        async fn generate(&self) -> RadrootsClientKeystoreResult<String> {
            self.generate_result.clone()
        }

        async fn add(&self, _secret_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn read(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
            self.keys_result.clone()
        }

        async fn remove(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }
    }

    #[test]
    fn keystore_public_key_maps_empty_to_none() {
        let keystore = TestKeystore {
            keys_result: Err(RadrootsClientKeystoreError::NostrNoResults),
            generate_result: Ok("generated".to_string()),
        };
        let result = futures::executor::block_on(app_keystore_nostr_public_key(&keystore))
            .expect("nostr key");
        assert!(result.is_none());
    }

    #[test]
    fn keystore_public_key_returns_existing() {
        let keystore = TestKeystore {
            keys_result: Ok(vec!["a".to_string(), "b".to_string()]),
            generate_result: Ok("generated".to_string()),
        };
        let result = futures::executor::block_on(app_keystore_nostr_public_key(&keystore))
            .expect("nostr key");
        assert_eq!(result.as_deref(), Some("b"));
    }

    #[test]
    fn keystore_keys_maps_errors() {
        let keystore = TestKeystore {
            keys_result: Err(RadrootsClientKeystoreError::IdbUndefined),
            generate_result: Ok("generated".to_string()),
        };
        let err = futures::executor::block_on(app_keystore_nostr_keys(&keystore))
            .expect_err("nostr key");
        assert_eq!(
            err,
            AppKeystoreError::Keystore(RadrootsClientKeystoreError::IdbUndefined)
        );
    }

    #[test]
    fn keystore_ensure_generates_when_empty() {
        let keystore = TestKeystore {
            keys_result: Err(RadrootsClientKeystoreError::NostrNoResults),
            generate_result: Ok("generated".to_string()),
        };
        let result = futures::executor::block_on(app_keystore_nostr_ensure_key(&keystore))
            .expect("nostr key");
        assert_eq!(result, "generated");
    }

    #[test]
    fn keystore_ensure_returns_existing() {
        let keystore = TestKeystore {
            keys_result: Ok(vec!["a".to_string()]),
            generate_result: Ok("generated".to_string()),
        };
        let result = futures::executor::block_on(app_keystore_nostr_ensure_key(&keystore))
            .expect("nostr key");
        assert_eq!(result, "a");
    }
}
