use crate::security::{
    ANDROID_NOSTR_NAMESPACE, load_secret, remove_secret, remove_secret_namespace, store_secret,
};
use radroots_identity::RadrootsIdentityId;
use radroots_nostr_accounts::prelude::{RadrootsNostrAccountsError, RadrootsNostrSecretVault};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub(crate) struct RadrootsAndroidKeystoreVault {
    service_name: String,
    namespace: String,
}

impl RadrootsAndroidKeystoreVault {
    pub(crate) fn new(service_name: impl Into<String>) -> Self {
        Self::new_with_namespace(service_name, ANDROID_NOSTR_NAMESPACE)
    }

    pub(crate) fn new_with_namespace(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            namespace: namespace.into(),
        }
    }

    fn account_name(account_id: &RadrootsIdentityId) -> &str {
        account_id.as_str()
    }

    pub(crate) fn purge_namespace(&self) -> Result<(), RadrootsNostrAccountsError> {
        remove_secret_namespace(self.service_name.as_str(), self.namespace.as_str())
    }
}

impl RadrootsNostrSecretVault for RadrootsAndroidKeystoreVault {
    fn store_secret_hex(
        &self,
        account_id: &RadrootsIdentityId,
        secret_key_hex: &str,
    ) -> Result<(), RadrootsNostrAccountsError> {
        let secret_key_hex = Zeroizing::new(secret_key_hex.to_owned());
        store_secret(
            self.service_name.as_str(),
            self.namespace.as_str(),
            Self::account_name(account_id),
            secret_key_hex.as_bytes(),
            true,
            false,
            true,
        )
    }

    fn load_secret_hex(
        &self,
        account_id: &RadrootsIdentityId,
    ) -> Result<Option<String>, RadrootsNostrAccountsError> {
        let Some(secret) = load_secret(
            self.service_name.as_str(),
            self.namespace.as_str(),
            Self::account_name(account_id),
        )?
        else {
            return Ok(None);
        };

        let secret = Zeroizing::new(secret);
        let secret = std::str::from_utf8(secret.as_slice()).map_err(|source| {
            RadrootsNostrAccountsError::Vault(format!(
                "android keystore secret was not valid utf-8: {source}"
            ))
        })?;
        Ok(Some(secret.to_owned()))
    }

    fn remove_secret(
        &self,
        account_id: &RadrootsIdentityId,
    ) -> Result<(), RadrootsNostrAccountsError> {
        remove_secret(
            self.service_name.as_str(),
            self.namespace.as_str(),
            Self::account_name(account_id),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_name_uses_account_id_string() {
        let account_id = RadrootsIdentityId::parse(
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606",
        )
        .expect("account id");

        assert_eq!(
            RadrootsAndroidKeystoreVault::account_name(&account_id),
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606"
        );
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn vault_operations_report_unavailable_off_android() {
        let vault = RadrootsAndroidKeystoreVault::new(crate::security::ANDROID_NOSTR_SERVICE);
        let account_id = RadrootsIdentityId::parse(
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606",
        )
        .expect("account id");

        let load = vault
            .load_secret_hex(&account_id)
            .expect_err("load off android");
        assert!(load.to_string().starts_with("vault error:"));

        let store = vault
            .store_secret_hex(&account_id, "deadbeef")
            .expect_err("store off android");
        assert!(store.to_string().starts_with("vault error:"));

        let remove = vault
            .remove_secret(&account_id)
            .expect_err("remove off android");
        assert!(remove.to_string().starts_with("vault error:"));
    }
}
