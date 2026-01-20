#![forbid(unsafe_code)]

use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

use crate::app_log_debug_emit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppKeystoreError {
    Keystore(RadrootsClientKeystoreError),
}

pub type RadrootsAppKeystoreResult<T> = Result<T, RadrootsAppKeystoreError>;

impl RadrootsAppKeystoreError {
    pub const fn message(&self) -> &'static str {
        match self {
            RadrootsAppKeystoreError::Keystore(err) => err.message(),
        }
    }
}

impl std::fmt::Display for RadrootsAppKeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppKeystoreError {}

impl From<RadrootsClientKeystoreError> for RadrootsAppKeystoreError {
    fn from(err: RadrootsClientKeystoreError) -> Self {
        RadrootsAppKeystoreError::Keystore(err)
    }
}

pub async fn app_keystore_nostr_keys<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> RadrootsAppKeystoreResult<Vec<String>> {
    let result = keystore.keys().await.map_err(RadrootsAppKeystoreError::from);
    let context = match &result {
        Ok(keys) => Some(format!("count={}", keys.len())),
        Err(err) => Some(err.to_string()),
    };
    let _ = app_log_debug_emit("log.app.keystore.keys", "fetch", context);
    result
}

pub async fn app_keystore_nostr_public_key<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> RadrootsAppKeystoreResult<Option<String>> {
    let _ = app_log_debug_emit("log.app.keystore.public_key", "start", None);
    match keystore.keys().await {
        Ok(mut keys) => {
            let key = keys.pop();
            let context = key.as_ref().map(|value| format!("key={value}"));
            let _ = app_log_debug_emit("log.app.keystore.public_key", "resolved", context);
            Ok(key)
        }
        Err(RadrootsClientKeystoreError::NostrNoResults) => Ok(None),
        Err(err) => Err(RadrootsAppKeystoreError::from(err)),
    }
}

pub async fn app_keystore_nostr_ensure_key<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> RadrootsAppKeystoreResult<String> {
    match app_keystore_nostr_public_key(keystore).await? {
        Some(key) => {
            let _ = app_log_debug_emit("log.app.keystore.ensure", "existing", None);
            Ok(key)
        }
        None => {
            let generated = keystore.generate().await.map_err(RadrootsAppKeystoreError::from)?;
            let _ = app_log_debug_emit("log.app.keystore.ensure", "generated", None);
            Ok(generated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_keystore_nostr_ensure_key,
        app_keystore_nostr_public_key,
        app_keystore_nostr_keys,
        RadrootsAppKeystoreError,
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
            RadrootsAppKeystoreError::Keystore(RadrootsClientKeystoreError::IdbUndefined)
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
