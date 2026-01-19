use async_trait::async_trait;

use radroots_nostr::prelude::{RadrootsNostrKeys, RadrootsNostrSecretKey};

use crate::idb::{IDB_CONFIG_KEYSTORE_NOSTR, RadrootsClientIdbConfig};

use super::{
    RadrootsClientKeystore,
    RadrootsClientKeystoreError,
    RadrootsClientKeystoreNostr,
    RadrootsClientKeystoreResult,
    RadrootsClientWebKeystore,
};

pub struct RadrootsClientWebKeystoreNostr {
    keystore: RadrootsClientWebKeystore,
}

impl RadrootsClientWebKeystoreNostr {
    pub fn new(config: Option<RadrootsClientIdbConfig>) -> Self {
        let config = config.unwrap_or(IDB_CONFIG_KEYSTORE_NOSTR);
        let keystore = RadrootsClientWebKeystore::new(Some(config));
        Self { keystore }
    }

    pub fn get_config(&self) -> RadrootsClientIdbConfig {
        self.keystore.get_config()
    }

    async fn add_secret_key(
        &self,
        secret_key: RadrootsNostrSecretKey,
    ) -> RadrootsClientKeystoreResult<String> {
        let secret_hex = secret_key.to_secret_hex();
        let keys = RadrootsNostrKeys::new(secret_key);
        let public_key = keys.public_key.to_hex();
        let _ = self.keystore.add(&public_key, &secret_hex).await?;
        Ok(public_key)
    }
}

#[async_trait(?Send)]
impl RadrootsClientKeystoreNostr for RadrootsClientWebKeystoreNostr {
    async fn generate(&self) -> RadrootsClientKeystoreResult<String> {
        let secret_key = RadrootsNostrSecretKey::generate();
        self.add_secret_key(secret_key).await
    }

    async fn add(&self, secret_key: &str) -> RadrootsClientKeystoreResult<String> {
        let secret_key = RadrootsNostrSecretKey::parse(secret_key)
            .map_err(|_| RadrootsClientKeystoreError::NostrInvalidSecretKey)?;
        self.add_secret_key(secret_key).await
    }

    async fn read(&self, public_key: &str) -> RadrootsClientKeystoreResult<String> {
        let value = self.keystore.read(Some(public_key)).await?;
        value.ok_or(RadrootsClientKeystoreError::MissingKey)
    }

    async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
        let keys = self.keystore.keys().await?;
        if keys.is_empty() {
            return Err(RadrootsClientKeystoreError::NostrNoResults);
        }
        Ok(keys)
    }

    async fn remove(&self, public_key: &str) -> RadrootsClientKeystoreResult<String> {
        let _ = self.keystore.remove(public_key).await?;
        Ok(public_key.to_string())
    }

    async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
        self.keystore.reset().await
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebKeystoreNostr;
    use crate::idb::IDB_CONFIG_KEYSTORE_NOSTR;
    use crate::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

    #[test]
    fn default_config_is_nostr_store() {
        let keystore = RadrootsClientWebKeystoreNostr::new(None);
        assert_eq!(keystore.get_config(), IDB_CONFIG_KEYSTORE_NOSTR);
    }

    #[test]
    fn invalid_secret_key_errors() {
        let keystore = RadrootsClientWebKeystoreNostr::new(None);
        let err = futures::executor::block_on(keystore.add("not-a-key"))
            .expect_err("invalid secret key");
        assert_eq!(err, RadrootsClientKeystoreError::NostrInvalidSecretKey);
    }
}
