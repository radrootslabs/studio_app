use crate::security::{
    ANDROID_NOSTR_NAMESPACE, load_secret, remove_secret, remove_secret_namespace, store_secret,
};
use radroots_secret_vault::{
    RadrootsHostVaultCapabilities, RadrootsHostVaultHardwarePolicy, RadrootsHostVaultPolicy,
    RadrootsHostVaultResidency, RadrootsHostVaultUserPresencePolicy, RadrootsSecretVault,
    RadrootsSecretVaultAccessError,
};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct RadrootsAndroidKeystoreVault {
    service_name: String,
    namespace: String,
}

impl RadrootsAndroidKeystoreVault {
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self::new_with_namespace(service_name, ANDROID_NOSTR_NAMESPACE)
    }

    #[must_use]
    pub fn new_with_namespace(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            namespace: namespace.into(),
        }
    }

    #[must_use]
    pub const fn secure_local_policy() -> RadrootsHostVaultPolicy {
        RadrootsHostVaultPolicy {
            residency: RadrootsHostVaultResidency::DeviceLocalOnly,
            user_presence: RadrootsHostVaultUserPresencePolicy::NotRequired,
            hardware: RadrootsHostVaultHardwarePolicy::PreferHardwareBacked,
        }
    }

    fn capabilities() -> RadrootsHostVaultCapabilities {
        #[cfg(target_os = "android")]
        {
            RadrootsHostVaultCapabilities {
                available: true,
                supports_device_local_only: true,
                supports_user_presence: true,
                supports_hardware_backed: true,
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            RadrootsHostVaultCapabilities::unavailable()
        }
    }

    fn validate_policy(
        policy: RadrootsHostVaultPolicy,
    ) -> Result<(), RadrootsSecretVaultAccessError> {
        Self::capabilities()
            .validate(policy)
            .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }

    fn security_flags(policy: RadrootsHostVaultPolicy) -> (bool, bool, bool) {
        (
            matches!(
                policy.residency,
                RadrootsHostVaultResidency::DeviceLocalOnly
            ),
            matches!(
                policy.user_presence,
                RadrootsHostVaultUserPresencePolicy::Required
            ),
            !matches!(policy.hardware, RadrootsHostVaultHardwarePolicy::Any),
        )
    }

    pub fn store_secret_with_policy(
        &self,
        slot: &str,
        secret: &str,
        policy: RadrootsHostVaultPolicy,
    ) -> Result<(), RadrootsSecretVaultAccessError> {
        Self::validate_policy(policy)?;
        let secret = Zeroizing::new(secret.to_owned());
        let (device_local_only, user_presence_required, prefer_strong_box) =
            Self::security_flags(policy);
        store_secret(
            self.service_name.as_str(),
            self.namespace.as_str(),
            slot,
            secret.as_bytes(),
            device_local_only,
            user_presence_required,
            prefer_strong_box,
        )
        .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn purge_namespace(&self) -> Result<(), RadrootsSecretVaultAccessError> {
        remove_secret_namespace(self.service_name.as_str(), self.namespace.as_str())
            .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }
}

impl RadrootsSecretVault for RadrootsAndroidKeystoreVault {
    fn store_secret(&self, slot: &str, secret: &str) -> Result<(), RadrootsSecretVaultAccessError> {
        self.store_secret_with_policy(slot, secret, Self::secure_local_policy())
    }

    fn load_secret(&self, slot: &str) -> Result<Option<String>, RadrootsSecretVaultAccessError> {
        let Some(secret) =
            load_secret(self.service_name.as_str(), self.namespace.as_str(), slot)
                .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))?
        else {
            return Ok(None);
        };

        let secret = Zeroizing::new(secret);
        let secret = std::str::from_utf8(secret.as_slice()).map_err(|source| {
            RadrootsSecretVaultAccessError::Backend(format!(
                "android keystore secret was not valid utf-8: {source}"
            ))
        })?;
        Ok(Some(secret.to_owned()))
    }

    fn remove_secret(&self, slot: &str) -> Result<(), RadrootsSecretVaultAccessError> {
        remove_secret(self.service_name.as_str(), self.namespace.as_str(), slot)
            .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_secret_vault::{
        RadrootsHostVaultHardwarePolicy, RadrootsHostVaultPolicy, RadrootsHostVaultResidency,
        RadrootsHostVaultUserPresencePolicy,
    };

    #[test]
    fn secure_local_policy_prefers_device_local_hardware_backed_storage() {
        assert_eq!(
            RadrootsAndroidKeystoreVault::secure_local_policy(),
            RadrootsHostVaultPolicy {
                residency: RadrootsHostVaultResidency::DeviceLocalOnly,
                user_presence: RadrootsHostVaultUserPresencePolicy::NotRequired,
                hardware: RadrootsHostVaultHardwarePolicy::PreferHardwareBacked,
            }
        );
    }

    #[test]
    fn security_flags_request_strong_box_for_hardware_backed_policies() {
        assert_eq!(
            RadrootsAndroidKeystoreVault::security_flags(RadrootsHostVaultPolicy {
                residency: RadrootsHostVaultResidency::UserProfile,
                user_presence: RadrootsHostVaultUserPresencePolicy::Required,
                hardware: RadrootsHostVaultHardwarePolicy::Any,
            }),
            (false, true, false)
        );
        assert_eq!(
            RadrootsAndroidKeystoreVault::security_flags(RadrootsHostVaultPolicy {
                residency: RadrootsHostVaultResidency::DeviceLocalOnly,
                user_presence: RadrootsHostVaultUserPresencePolicy::Required,
                hardware: RadrootsHostVaultHardwarePolicy::RequireHardwareBacked,
            }),
            (true, true, true)
        );
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn vault_operations_report_unavailable_off_android() {
        let vault = RadrootsAndroidKeystoreVault::new(crate::security::ANDROID_NOSTR_SERVICE);

        let load = vault.load_secret("alice").expect_err("load off android");
        assert!(load.to_string().starts_with("secret vault access error:"));

        let store = vault
            .store_secret("alice", "deadbeef")
            .expect_err("store off android");
        assert!(store.to_string().starts_with("secret vault access error:"));

        let remove = vault
            .remove_secret("alice")
            .expect_err("remove off android");
        assert!(remove.to_string().starts_with("secret vault access error:"));
    }
}
