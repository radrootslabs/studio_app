use radroots_nostr_accounts::prelude::RadrootsNostrAccountsError;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::ffi::CStr;
use std::ffi::CString;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::os::raw::{c_char, c_int};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::ptr;

pub const APPLE_NOSTR_SERVICE: &str = "org.radroots.app.nostr";
pub const APPLE_NOSTR_NAMESPACE: &str = "nostr";

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppleSecretStatus {
    Success = 0,
    NotFound = 1,
    InvalidInput = 2,
    Error = 3,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl AppleSecretStatus {
    fn from_raw(value: i32) -> Result<Self, RadrootsNostrAccountsError> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::NotFound),
            2 => Ok(Self::InvalidInput),
            3 => Ok(Self::Error),
            other => Err(RadrootsNostrAccountsError::Vault(format!(
                "unknown apple security ffi status {other}"
            ))),
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
unsafe extern "C" {
    fn radroots_studio_apple_secret_store_put(
        service_prefix: *const c_char,
        namespace: *const c_char,
        name: *const c_char,
        value_ptr: *const u8,
        value_len: isize,
        accessibility_raw: i32,
        device_local_only_raw: i32,
        user_presence_required_raw: i32,
        error_out: *mut *mut c_char,
    ) -> i32;

    fn radroots_studio_apple_secret_store_get(
        service_prefix: *const c_char,
        namespace: *const c_char,
        name: *const c_char,
        value_out: *mut *mut u8,
        value_len_out: *mut isize,
        error_out: *mut *mut c_char,
    ) -> i32;

    fn radroots_studio_apple_secret_store_delete(
        service_prefix: *const c_char,
        namespace: *const c_char,
        name: *const c_char,
        error_out: *mut *mut c_char,
    ) -> i32;

    fn radroots_studio_apple_buffer_free(buffer: *mut u8, length: isize);
    fn radroots_studio_apple_c_string_free(string: *mut c_char);
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
struct FfiErrorString {
    ptr: *mut c_char,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl FfiErrorString {
    fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
        }
    }

    fn as_mut_ptr(&mut self) -> *mut *mut c_char {
        &mut self.ptr
    }

    fn message(&self) -> Option<String> {
        if self.ptr.is_null() {
            return None;
        }
        // SAFETY: the Swift FFI returns a null-terminated string pointer that remains valid
        // until released through the paired free function.
        unsafe { Some(CStr::from_ptr(self.ptr).to_string_lossy().into_owned()) }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Drop for FfiErrorString {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        // SAFETY: the pointer originated from the Swift FFI string allocator.
        unsafe {
            radroots_studio_apple_c_string_free(self.ptr);
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
struct FfiDataBuffer {
    ptr: *mut u8,
    len: isize,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl FfiDataBuffer {
    fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    fn as_mut_ptr(&mut self) -> *mut *mut u8 {
        &mut self.ptr
    }

    fn len_mut_ptr(&mut self) -> *mut isize {
        &mut self.len
    }

    fn to_vec(&self) -> Result<Vec<u8>, RadrootsNostrAccountsError> {
        if self.len < 0 {
            return Err(RadrootsNostrAccountsError::Vault(
                "apple security ffi returned a negative buffer length".to_owned(),
            ));
        }
        if self.ptr.is_null() {
            if self.len == 0 {
                return Ok(Vec::new());
            }
            return Err(RadrootsNostrAccountsError::Vault(
                "apple security ffi returned a null buffer pointer".to_owned(),
            ));
        }
        // SAFETY: the pointer and length pair came from the Swift FFI and stays valid until
        // released by the paired free function. We copy into an owned Vec before dropping.
        unsafe { Ok(std::slice::from_raw_parts(self.ptr, self.len as usize).to_vec()) }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Drop for FfiDataBuffer {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        // SAFETY: the pointer originated from the Swift FFI buffer allocator.
        unsafe {
            radroots_studio_apple_buffer_free(self.ptr, self.len);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppleSecretAccessibility {
    WhenUnlocked = 0,
}

#[derive(Debug, Clone, Copy)]
pub struct AppleSecretAccessPolicy {
    pub accessibility: AppleSecretAccessibility,
    pub device_local_only: bool,
    pub user_presence_required: bool,
}

impl AppleSecretAccessPolicy {
    pub const SECURE_LOCAL_SECRET: Self = Self {
        accessibility: AppleSecretAccessibility::WhenUnlocked,
        device_local_only: true,
        user_presence_required: false,
    };
}

pub fn store_secret(
    service: &str,
    namespace: &str,
    name: &str,
    value: &[u8],
    policy: AppleSecretAccessPolicy,
) -> Result<(), RadrootsNostrAccountsError> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let service = c_string(service)?;
        let namespace = c_string(namespace)?;
        let name = c_string(name)?;
        let mut ffi_error = FfiErrorString::new();
        let status = unsafe {
            // SAFETY: all pointers are derived from live CString values and valid slices.
            radroots_studio_apple_secret_store_put(
                service.as_ptr(),
                namespace.as_ptr(),
                name.as_ptr(),
                value.as_ptr(),
                value.len() as isize,
                policy.accessibility as i32,
                bool_to_c_int(policy.device_local_only),
                bool_to_c_int(policy.user_presence_required),
                ffi_error.as_mut_ptr(),
            )
        };
        return match AppleSecretStatus::from_raw(status)? {
            AppleSecretStatus::Success => Ok(()),
            AppleSecretStatus::NotFound => Err(vault_error(
                ffi_error,
                "apple security ffi reported not found during store",
            )),
            AppleSecretStatus::InvalidInput => Err(vault_error(
                ffi_error,
                "apple security ffi rejected the store request",
            )),
            AppleSecretStatus::Error => Err(vault_error(ffi_error, "apple keychain store failed")),
        };
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = (service, namespace, name, value, policy);
        Err(RadrootsNostrAccountsError::Vault(
            "apple keychain storage is only available on ios and macos".to_owned(),
        ))
    }
}

pub fn load_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<Option<Vec<u8>>, RadrootsNostrAccountsError> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let service = c_string(service)?;
        let namespace = c_string(namespace)?;
        let name = c_string(name)?;
        let mut ffi_error = FfiErrorString::new();
        let mut ffi_buffer = FfiDataBuffer::new();
        let status = unsafe {
            // SAFETY: all output pointers reference live local storage for the duration
            // of the call, and all input strings are backed by live CString values.
            radroots_studio_apple_secret_store_get(
                service.as_ptr(),
                namespace.as_ptr(),
                name.as_ptr(),
                ffi_buffer.as_mut_ptr(),
                ffi_buffer.len_mut_ptr(),
                ffi_error.as_mut_ptr(),
            )
        };
        return match AppleSecretStatus::from_raw(status)? {
            AppleSecretStatus::Success => ffi_buffer.to_vec().map(Some),
            AppleSecretStatus::NotFound => Ok(None),
            AppleSecretStatus::InvalidInput => Err(vault_error(
                ffi_error,
                "apple security ffi rejected the load request",
            )),
            AppleSecretStatus::Error => Err(vault_error(ffi_error, "apple keychain load failed")),
        };
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = (service, namespace, name);
        Err(RadrootsNostrAccountsError::Vault(
            "apple keychain storage is only available on ios and macos".to_owned(),
        ))
    }
}

pub fn remove_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<(), RadrootsNostrAccountsError> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let service = c_string(service)?;
        let namespace = c_string(namespace)?;
        let name = c_string(name)?;
        let mut ffi_error = FfiErrorString::new();
        let status = unsafe {
            // SAFETY: all pointers are backed by live CString values for the duration
            // of the call.
            radroots_studio_apple_secret_store_delete(
                service.as_ptr(),
                namespace.as_ptr(),
                name.as_ptr(),
                ffi_error.as_mut_ptr(),
            )
        };
        return match AppleSecretStatus::from_raw(status)? {
            AppleSecretStatus::Success | AppleSecretStatus::NotFound => Ok(()),
            AppleSecretStatus::InvalidInput => Err(vault_error(
                ffi_error,
                "apple security ffi rejected the delete request",
            )),
            AppleSecretStatus::Error => Err(vault_error(ffi_error, "apple keychain delete failed")),
        };
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = (service, namespace, name);
        Err(RadrootsNostrAccountsError::Vault(
            "apple keychain storage is only available on ios and macos".to_owned(),
        ))
    }
}

fn c_string(value: &str) -> Result<CString, RadrootsNostrAccountsError> {
    CString::new(value).map_err(|_| {
        RadrootsNostrAccountsError::Vault(
            "apple security ffi input contained an interior nul".into(),
        )
    })
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn bool_to_c_int(value: bool) -> c_int {
    if value { 1 } else { 0 }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn vault_error(
    ffi_error: FfiErrorString,
    fallback: impl Into<String>,
) -> RadrootsNostrAccountsError {
    let fallback = fallback.into();
    let message = ffi_error.message().unwrap_or(fallback);
    RadrootsNostrAccountsError::Vault(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_local_secret_policy_defaults_to_when_unlocked_device_local() {
        let policy = AppleSecretAccessPolicy::SECURE_LOCAL_SECRET;

        assert!(matches!(
            policy.accessibility,
            AppleSecretAccessibility::WhenUnlocked
        ));
        assert!(policy.device_local_only);
        assert!(!policy.user_presence_required);
    }

    #[test]
    fn c_string_rejects_interior_nul() {
        let err = c_string("bad\0value").expect_err("interior nul");
        assert!(err.to_string().starts_with("vault error:"));
    }
}
