use crate::security::{
    APPLE_NOSTR_NAMESPACE, AppleSecretAccessPolicy, AppleSecretAccessibility, load_secret,
    remove_secret, remove_secret_namespace, store_secret,
};
use radroots_secret_vault::{
    RadrootsHostVaultCapabilities, RadrootsHostVaultPolicy, RadrootsHostVaultResidency,
    RadrootsHostVaultUserPresencePolicy, RadrootsSecretVault, RadrootsSecretVaultAccessError,
};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct RadrootsAppleKeychainVault {
    service_name: String,
    namespace: String,
    default_policy: RadrootsHostVaultPolicy,
}

impl RadrootsAppleKeychainVault {
    #[must_use]
    pub fn new_desktop(service_name: impl Into<String>) -> Self {
        Self::new_with_namespace_desktop(service_name, APPLE_NOSTR_NAMESPACE)
    }

    #[must_use]
    pub fn new_device_local(service_name: impl Into<String>) -> Self {
        Self::new_with_namespace_device_local(service_name, APPLE_NOSTR_NAMESPACE)
    }

    #[must_use]
    pub fn new_with_namespace_desktop(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self::new_with_namespace_and_policy(service_name, namespace, Self::desktop_policy())
    }

    #[must_use]
    pub fn new_with_namespace_device_local(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self::new_with_namespace_and_policy(service_name, namespace, Self::device_local_policy())
    }

    fn new_with_namespace_and_policy(
        service_name: impl Into<String>,
        namespace: impl Into<String>,
        default_policy: RadrootsHostVaultPolicy,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            namespace: namespace.into(),
            default_policy,
        }
    }

    #[must_use]
    pub const fn desktop_policy() -> RadrootsHostVaultPolicy {
        RadrootsHostVaultPolicy::desktop()
    }

    #[must_use]
    pub const fn device_local_policy() -> RadrootsHostVaultPolicy {
        RadrootsHostVaultPolicy::device_local()
    }

    fn capabilities() -> RadrootsHostVaultCapabilities {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            RadrootsHostVaultCapabilities {
                available: true,
                supports_device_local_only: true,
                supports_user_presence: true,
                supports_hardware_backed: false,
            }
        }

        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
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

    fn access_policy(policy: RadrootsHostVaultPolicy) -> AppleSecretAccessPolicy {
        AppleSecretAccessPolicy {
            accessibility: AppleSecretAccessibility::WhenUnlocked,
            device_local_only: matches!(
                policy.residency,
                RadrootsHostVaultResidency::DeviceLocalOnly
            ),
            user_presence_required: matches!(
                policy.user_presence,
                RadrootsHostVaultUserPresencePolicy::Required
            ),
        }
    }

    pub fn store_secret_with_policy(
        &self,
        slot: &str,
        secret: &str,
        policy: RadrootsHostVaultPolicy,
    ) -> Result<(), RadrootsSecretVaultAccessError> {
        Self::validate_policy(policy)?;
        let secret = Zeroizing::new(secret.to_owned());
        store_secret(
            self.service_name.as_str(),
            self.namespace.as_str(),
            slot,
            secret.as_bytes(),
            Self::access_policy(policy),
        )
        .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }

    pub fn purge_namespace(&self) -> Result<(), RadrootsSecretVaultAccessError> {
        remove_secret_namespace(self.service_name.as_str(), self.namespace.as_str())
            .map_err(|source| RadrootsSecretVaultAccessError::Backend(source.to_string()))
    }
}

impl RadrootsSecretVault for RadrootsAppleKeychainVault {
    fn store_secret(&self, slot: &str, secret: &str) -> Result<(), RadrootsSecretVaultAccessError> {
        self.store_secret_with_policy(slot, secret, self.default_policy)
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
                "apple keychain secret was not valid utf-8: {source}"
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
    fn desktop_policy_matches_shared_desktop_contract() {
        assert_eq!(
            RadrootsAppleKeychainVault::desktop_policy(),
            RadrootsHostVaultPolicy {
                residency: RadrootsHostVaultResidency::UserProfile,
                user_presence: RadrootsHostVaultUserPresencePolicy::NotRequired,
                hardware: RadrootsHostVaultHardwarePolicy::Any,
            }
        );
    }

    #[test]
    fn device_local_policy_matches_shared_mobile_contract() {
        assert_eq!(
            RadrootsAppleKeychainVault::device_local_policy(),
            RadrootsHostVaultPolicy {
                residency: RadrootsHostVaultResidency::DeviceLocalOnly,
                user_presence: RadrootsHostVaultUserPresencePolicy::NotRequired,
                hardware: RadrootsHostVaultHardwarePolicy::Any,
            }
        );
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    #[test]
    fn vault_operations_report_unavailable_off_apple() {
        let vault = RadrootsAppleKeychainVault::new_desktop(crate::APPLE_NOSTR_SERVICE);

        let load = vault.load_secret("alice").expect_err("load off apple");
        assert!(load.to_string().starts_with("secret vault access error:"));

        let store = vault
            .store_secret("alice", "deadbeef")
            .expect_err("store off apple");
        assert!(store.to_string().starts_with("secret vault access error:"));

        let remove = vault.remove_secret("alice").expect_err("remove off apple");
        assert!(remove.to_string().starts_with("secret vault access error:"));
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[test]
    fn hardware_backed_requirement_reports_unsupported() {
        let vault = RadrootsAppleKeychainVault::new_device_local(crate::APPLE_NOSTR_SERVICE);
        let error = vault
            .store_secret_with_policy(
                "alice",
                "deadbeef",
                RadrootsHostVaultPolicy {
                    residency: RadrootsHostVaultResidency::DeviceLocalOnly,
                    user_presence: RadrootsHostVaultUserPresencePolicy::Required,
                    hardware: RadrootsHostVaultHardwarePolicy::RequireHardwareBacked,
                },
            )
            .expect_err("apple adapter should reject hardware-backed requirement");

        assert!(error.to_string().contains("hardware_backed"));
    }
}
