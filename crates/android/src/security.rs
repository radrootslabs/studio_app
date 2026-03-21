use radroots_nostr_accounts::prelude::RadrootsNostrAccountsError;
use std::path::PathBuf;

pub(crate) const ANDROID_NOSTR_SERVICE: &str = "org.radroots.app.nostr";
pub(crate) const ANDROID_NOSTR_NAMESPACE: &str = "nostr";

#[cfg(target_os = "android")]
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
#[cfg(target_os = "android")]
use jni::sys::{jboolean, jobject};
#[cfg(target_os = "android")]
use jni::{JNIEnv, JavaVM};

#[cfg(target_os = "android")]
const ANDROID_SECURITY_BRIDGE_CLASS: &str =
    "org.radroots.app.android.security.RadRootsAndroidSecurityBridge";

#[cfg(target_os = "android")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AndroidSecretStatus {
    Success,
    NotFound,
    InvalidInput,
    Error,
}

#[cfg(target_os = "android")]
impl AndroidSecretStatus {
    fn from_raw(value: i32) -> Result<Self, RadrootsNostrAccountsError> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::NotFound),
            2 => Ok(Self::InvalidInput),
            3 => Ok(Self::Error),
            other => Err(RadrootsNostrAccountsError::Vault(format!(
                "unknown android security bridge status {other}"
            ))),
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) fn store_secret(
    service: &str,
    namespace: &str,
    name: &str,
    value: &[u8],
    device_local_only: bool,
    user_presence_required: bool,
    prefer_strong_box: bool,
) -> Result<(), RadrootsNostrAccountsError> {
    let java_vm = android_java_vm()?;
    let mut env = java_vm.attach_current_thread().map_err(jni_error)?;
    let bridge_class = bridge_class(&mut env)?;
    let service = java_string_arg(&mut env, service)?;
    let namespace = java_string_arg(&mut env, namespace)?;
    let name = java_string_arg(&mut env, name)?;
    let value = env.byte_array_from_slice(value).map_err(jni_error)?;
    let value = JObject::from(value);

    let status = env
        .call_static_method(
            &bridge_class,
            "putSecret",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;[BZZZ)I",
            &[
                JValue::Object(&service),
                JValue::Object(&namespace),
                JValue::Object(&name),
                JValue::Object(&value),
                JValue::Bool(bool_to_jboolean(device_local_only)),
                JValue::Bool(bool_to_jboolean(user_presence_required)),
                JValue::Bool(bool_to_jboolean(prefer_strong_box)),
            ],
        )
        .and_then(|value| value.i())
        .map_err(jni_error)?;

    match AndroidSecretStatus::from_raw(status)? {
        AndroidSecretStatus::Success => Ok(()),
        AndroidSecretStatus::NotFound => Err(bridge_vault_error(
            &mut env,
            &bridge_class,
            "android security bridge reported not found during store",
        )),
        AndroidSecretStatus::InvalidInput => Err(bridge_vault_error(
            &mut env,
            &bridge_class,
            "android security bridge rejected the store request",
        )),
        AndroidSecretStatus::Error => Err(bridge_vault_error(
            &mut env,
            &bridge_class,
            "android keystore store failed",
        )),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn store_secret(
    service: &str,
    namespace: &str,
    name: &str,
    value: &[u8],
    device_local_only: bool,
    user_presence_required: bool,
    prefer_strong_box: bool,
) -> Result<(), RadrootsNostrAccountsError> {
    let _ = (
        service,
        namespace,
        name,
        value,
        device_local_only,
        user_presence_required,
        prefer_strong_box,
    );
    Err(RadrootsNostrAccountsError::Vault(
        "android keystore storage is only available on android".to_owned(),
    ))
}

#[cfg(target_os = "android")]
pub(crate) fn load_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<Option<Vec<u8>>, RadrootsNostrAccountsError> {
    let java_vm = android_java_vm()?;
    let mut env = java_vm.attach_current_thread().map_err(jni_error)?;
    let bridge_class = bridge_class(&mut env)?;
    let service = java_string_arg(&mut env, service)?;
    let namespace = java_string_arg(&mut env, namespace)?;
    let name = java_string_arg(&mut env, name)?;

    let value = env
        .call_static_method(
            &bridge_class,
            "getSecret",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)[B",
            &[
                JValue::Object(&service),
                JValue::Object(&namespace),
                JValue::Object(&name),
            ],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)?;

    if value.is_null() {
        let Some(message) = take_last_error_message(&mut env, &bridge_class)? else {
            return Ok(None);
        };
        return Err(RadrootsNostrAccountsError::Vault(message));
    }

    let value = JByteArray::from(value);
    env.convert_byte_array(&value).map(Some).map_err(jni_error)
}

#[cfg(not(target_os = "android"))]
pub(crate) fn load_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<Option<Vec<u8>>, RadrootsNostrAccountsError> {
    let _ = (service, namespace, name);
    Err(RadrootsNostrAccountsError::Vault(
        "android keystore storage is only available on android".to_owned(),
    ))
}

#[cfg(target_os = "android")]
pub(crate) fn remove_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<(), RadrootsNostrAccountsError> {
    let java_vm = android_java_vm()?;
    let mut env = java_vm.attach_current_thread().map_err(jni_error)?;
    let bridge_class = bridge_class(&mut env)?;
    let service = java_string_arg(&mut env, service)?;
    let namespace = java_string_arg(&mut env, namespace)?;
    let name = java_string_arg(&mut env, name)?;

    let status = env
        .call_static_method(
            &bridge_class,
            "deleteSecret",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                JValue::Object(&service),
                JValue::Object(&namespace),
                JValue::Object(&name),
            ],
        )
        .and_then(|value| value.i())
        .map_err(jni_error)?;

    match AndroidSecretStatus::from_raw(status)? {
        AndroidSecretStatus::Success | AndroidSecretStatus::NotFound => Ok(()),
        AndroidSecretStatus::InvalidInput => Err(bridge_vault_error(
            &mut env,
            &bridge_class,
            "android security bridge rejected the delete request",
        )),
        AndroidSecretStatus::Error => Err(bridge_vault_error(
            &mut env,
            &bridge_class,
            "android keystore delete failed",
        )),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn remove_secret(
    service: &str,
    namespace: &str,
    name: &str,
) -> Result<(), RadrootsNostrAccountsError> {
    let _ = (service, namespace, name);
    Err(RadrootsNostrAccountsError::Vault(
        "android keystore storage is only available on android".to_owned(),
    ))
}

#[cfg(target_os = "android")]
pub(crate) fn resolve_nostr_storage_root() -> Result<PathBuf, RadrootsNostrAccountsError> {
    let java_vm = android_java_vm()?;
    let mut env = java_vm.attach_current_thread().map_err(jni_error)?;
    let bridge_class = bridge_class(&mut env)?;
    let value = env
        .call_static_method(
            &bridge_class,
            "resolveNostrStorageRoot",
            "()Ljava/lang/String;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)?;

    if value.is_null() {
        return Err(bridge_store_error(
            &mut env,
            &bridge_class,
            "android security bridge returned no storage root",
        ));
    }

    let value = JString::from(value);
    let path: String = env.get_string(&value).map_err(jni_error)?.into();
    Ok(PathBuf::from(path))
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub(crate) fn resolve_nostr_storage_root() -> Result<PathBuf, RadrootsNostrAccountsError> {
    Err(RadrootsNostrAccountsError::Store(
        "android no-backup storage is only available on android".to_owned(),
    ))
}

#[cfg(target_os = "android")]
fn android_java_vm() -> Result<JavaVM, RadrootsNostrAccountsError> {
    let context = ndk_context::android_context();
    // SAFETY: ndk_context is initialized by the Android runtime before this code runs and
    // returns a stable JavaVM pointer for the current process.
    unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(jni_error)
}

#[cfg(target_os = "android")]
fn bridge_class<'local>(
    env: &mut JNIEnv<'local>,
) -> Result<JClass<'local>, RadrootsNostrAccountsError> {
    let context = ndk_context::android_context();
    // SAFETY: ndk_context stores a live process-wide Context jobject for the active Android app.
    let context = unsafe { JObject::from_raw(context.context() as jobject) };
    let context = env.new_local_ref(&context).map_err(jni_error)?;
    let class_loader = env
        .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .and_then(|value| value.l())
        .map_err(jni_error)?;
    let class_name = env
        .new_string(ANDROID_SECURITY_BRIDGE_CLASS)
        .map_err(jni_error)?;
    let class_name = JObject::from(class_name);
    let bridge_class = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)?;
    Ok(JClass::from(bridge_class))
}

#[cfg(target_os = "android")]
fn java_string_arg<'local>(
    env: &mut JNIEnv<'local>,
    value: &str,
) -> Result<JObject<'local>, RadrootsNostrAccountsError> {
    env.new_string(value).map(JObject::from).map_err(jni_error)
}

#[cfg(target_os = "android")]
fn take_last_error_message(
    env: &mut JNIEnv<'_>,
    bridge_class: &JClass<'_>,
) -> Result<Option<String>, RadrootsNostrAccountsError> {
    let value = env
        .call_static_method(
            bridge_class,
            "takeLastErrorMessage",
            "()Ljava/lang/String;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)?;
    if value.is_null() {
        return Ok(None);
    }
    let value = JString::from(value);
    let value: String = env.get_string(&value).map_err(jni_error)?.into();
    Ok(Some(value))
}

#[cfg(target_os = "android")]
fn bridge_vault_error(
    env: &mut JNIEnv<'_>,
    bridge_class: &JClass<'_>,
    fallback: &str,
) -> RadrootsNostrAccountsError {
    let message = take_last_error_message(env, bridge_class)
        .ok()
        .flatten()
        .unwrap_or_else(|| fallback.to_owned());
    RadrootsNostrAccountsError::Vault(message)
}

#[cfg(target_os = "android")]
fn bridge_store_error(
    env: &mut JNIEnv<'_>,
    bridge_class: &JClass<'_>,
    fallback: &str,
) -> RadrootsNostrAccountsError {
    let message = take_last_error_message(env, bridge_class)
        .ok()
        .flatten()
        .unwrap_or_else(|| fallback.to_owned());
    RadrootsNostrAccountsError::Store(message)
}

#[cfg(target_os = "android")]
fn jni_error(error: jni::errors::Error) -> RadrootsNostrAccountsError {
    RadrootsNostrAccountsError::Vault(format!("android jni error: {error}"))
}

#[cfg(target_os = "android")]
fn bool_to_jboolean(value: bool) -> jboolean {
    if value { 1 } else { 0 }
}
