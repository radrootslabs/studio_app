use crate::security::{
    APPLE_NOSTR_NAMESPACE, AppleSecretAccessPolicy, load_secret, remove_secret, store_secret,
};
use radroots_identity::RadrootsIdentityId;
use radroots_nostr_accounts::prelude::{RadrootsNostrAccountsError, RadrootsNostrSecretVault};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct RadrootsAppleKeychainVault {
    service_name: String,
}

impl RadrootsAppleKeychainVault {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn account_name(account_id: &RadrootsIdentityId) -> &str {
        account_id.as_str()
    }
}

impl RadrootsNostrSecretVault for RadrootsAppleKeychainVault {
    fn store_secret_hex(
        &self,
        account_id: &RadrootsIdentityId,
        secret_key_hex: &str,
    ) -> Result<(), RadrootsNostrAccountsError> {
        let secret_key_hex = Zeroizing::new(secret_key_hex.to_owned());
        store_secret(
            self.service_name.as_str(),
            APPLE_NOSTR_NAMESPACE,
            Self::account_name(account_id),
            secret_key_hex.as_bytes(),
            AppleSecretAccessPolicy::SECURE_LOCAL_SECRET,
        )
    }

    fn load_secret_hex(
        &self,
        account_id: &RadrootsIdentityId,
    ) -> Result<Option<String>, RadrootsNostrAccountsError> {
        let Some(secret) = load_secret(
            self.service_name.as_str(),
            APPLE_NOSTR_NAMESPACE,
            Self::account_name(account_id),
        )?
        else {
            return Ok(None);
        };

        let secret = Zeroizing::new(secret);
        let secret = std::str::from_utf8(secret.as_slice()).map_err(|source| {
            RadrootsNostrAccountsError::Vault(format!(
                "apple keychain secret was not valid utf-8: {source}"
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
            APPLE_NOSTR_NAMESPACE,
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
            RadrootsAppleKeychainVault::account_name(&account_id),
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606"
        );
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    #[test]
    fn vault_operations_report_unavailable_off_apple() {
        let vault = RadrootsAppleKeychainVault::new(crate::APPLE_NOSTR_SERVICE);
        let account_id = RadrootsIdentityId::parse(
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606",
        )
        .expect("account id");

        let load = vault
            .load_secret_hex(&account_id)
            .expect_err("load off apple");
        assert!(load.to_string().starts_with("vault error:"));

        let store = vault
            .store_secret_hex(&account_id, "deadbeef")
            .expect_err("store off apple");
        assert!(store.to_string().starts_with("vault error:"));

        let remove = vault
            .remove_secret(&account_id)
            .expect_err("remove off apple");
        assert!(remove.to_string().starts_with("vault error:"));
    }
}
