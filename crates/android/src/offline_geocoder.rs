#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use radroots_studio_app_core::{
    RadrootsOfflineGeocoderState, RadrootsOfflineGeocoderUnavailableKind,
};
#[cfg(target_os = "android")]
use radroots_geocoder::Geocoder;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JString};
#[cfg(target_os = "android")]
use jni::sys::jobject;
#[cfg(target_os = "android")]
use jni::{JNIEnv, JavaVM};
#[cfg(target_os = "android")]
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsError;

#[cfg(target_os = "android")]
const ANDROID_APP_BRIDGE_CLASS: &str = "org.radroots.app.android.RadRootsAndroidAppBridge";

#[derive(Clone)]
pub(crate) struct AndroidOfflineGeocoder {
    current: Arc<Mutex<RadrootsOfflineGeocoderState>>,
    changed: Arc<AtomicBool>,
}

impl AndroidOfflineGeocoder {
    pub(crate) fn from_state(state: RadrootsOfflineGeocoderState) -> Self {
        Self {
            current: Arc::new(Mutex::new(state)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn start() -> Self {
        let tracker = Self::from_state(RadrootsOfflineGeocoderState::Initializing);
        let current = Arc::clone(&tracker.current);
        let changed = Arc::clone(&tracker.changed);

        std::thread::spawn(move || {
            let state = initialize_offline_geocoder();
            if let RadrootsOfflineGeocoderState::Unavailable { debug_message, .. } = &state {
                log::warn!("android offline geocoder unavailable: {debug_message}");
            }
            if let Ok(mut slot) = current.lock() {
                *slot = state;
                changed.store(true, Ordering::Release);
            }
        });

        tracker
    }

    pub(crate) fn current_state(&self) -> RadrootsOfflineGeocoderState {
        self.current
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| {
                RadrootsOfflineGeocoderState::unavailable(
                    RadrootsOfflineGeocoderUnavailableKind::InternalError,
                    "android offline geocoder state lock poisoned",
                )
            })
    }

    pub(crate) fn take_update(&self) -> Option<RadrootsOfflineGeocoderState> {
        if self.changed.swap(false, Ordering::AcqRel) {
            Some(self.current_state())
        } else {
            None
        }
    }
}

#[cfg(target_os = "android")]
fn initialize_offline_geocoder() -> RadrootsOfflineGeocoderState {
    match initialize_offline_geocoder_inner() {
        Ok(()) => RadrootsOfflineGeocoderState::Ready,
        Err((kind, debug_message)) => {
            RadrootsOfflineGeocoderState::unavailable(kind, debug_message)
        }
    }
}

#[cfg(target_os = "android")]
fn initialize_offline_geocoder_inner(
) -> Result<(), (RadrootsOfflineGeocoderUnavailableKind, String)> {
    let staged_path = stage_offline_geocoder_asset()?;
    Geocoder::open_path(staged_path.as_str())
        .map(|_| ())
        .map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
                format!("failed to open staged android geocoder db: {source}"),
            )
        })
}

#[cfg(target_os = "android")]
fn stage_offline_geocoder_asset() -> Result<String, (RadrootsOfflineGeocoderUnavailableKind, String)> {
    let java_vm = android_java_vm().map_err(|source| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InternalError,
            source.to_string(),
        )
    })?;
    let mut env = java_vm
        .attach_current_thread()
        .map_err(jni_error)
        .map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InternalError,
                source.to_string(),
            )
        })?;
    let bridge_class = bridge_class(&mut env).map_err(|source| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InternalError,
            source.to_string(),
        )
    })?;
    let value = env
        .call_static_method(
            &bridge_class,
            "stageOfflineGeocoderAsset",
            "()Ljava/lang/String;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)
        .map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InternalError,
                source.to_string(),
            )
        })?;

    if value.is_null() {
        let error_kind = take_last_error_kind(&mut env, &bridge_class).map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InternalError,
                source.to_string(),
            )
        })?;
        let debug_message = take_last_error_message(&mut env, &bridge_class)
            .map_err(|source| {
                (
                    RadrootsOfflineGeocoderUnavailableKind::InternalError,
                    source.to_string(),
                )
            })?
            .unwrap_or_else(|| "android app bridge returned no staged geocoder path".to_owned());
        return Err((error_kind, debug_message));
    }

    let value = JString::from(value);
    env.get_string(&value)
        .map(|value| value.into())
        .map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InternalError,
                jni_error(source).to_string(),
            )
        })
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
fn android_java_vm() -> Result<JavaVM, RadrootsNostrAccountsError> {
    let context = ndk_context::android_context();
    // SAFETY: ndk_context is initialized by the Android runtime before this code runs and
    // returns a stable JavaVM pointer for the current process.
    unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(jni_error)
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
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
        .new_string(ANDROID_APP_BRIDGE_CLASS)
        .map_err(jni_error)?;
    let class_name = JObject::from(class_name);
    let bridge_class = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[jni::objects::JValue::Object(&class_name)],
        )
        .and_then(|value| value.l())
        .map_err(jni_error)?;
    Ok(JClass::from(bridge_class))
}

#[cfg(target_os = "android")]
fn take_last_error_kind(
    env: &mut JNIEnv<'_>,
    bridge_class: &JClass<'_>,
) -> Result<RadrootsOfflineGeocoderUnavailableKind, RadrootsNostrAccountsError> {
    let value = env
        .call_static_method(bridge_class, "takeLastErrorKind", "()I", &[])
        .and_then(|value| value.i())
        .map_err(jni_error)?;
    match value {
        1 => Ok(RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset),
        2 => Ok(RadrootsOfflineGeocoderUnavailableKind::InitializationFailed),
        3 => Ok(RadrootsOfflineGeocoderUnavailableKind::InternalError),
        _ => Ok(RadrootsOfflineGeocoderUnavailableKind::InitializationFailed),
    }
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
fn jni_error(error: jni::errors::Error) -> RadrootsNostrAccountsError {
    RadrootsNostrAccountsError::Store(format!("android jni error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_asset_maps_to_build_unavailable_message() {
        let state = RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            "android bundled geocoder asset missing at assets/geocoder/geonames.db",
        );

        assert_eq!(
            state,
            RadrootsOfflineGeocoderState::Unavailable {
                kind: RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                debug_message:
                    "android bundled geocoder asset missing at assets/geocoder/geonames.db"
                        .to_owned(),
            }
        );
    }
}
