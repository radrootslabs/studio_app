use async_trait::async_trait;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

use super::{
    RadrootsClientGeolocation,
    RadrootsClientGeolocationError,
    RadrootsClientGeolocationPosition,
    RadrootsClientGeolocationResult,
};

pub struct RadrootsClientWebGeolocation;

impl RadrootsClientWebGeolocation {
    #[cfg(target_arch = "wasm32")]
    fn policy_allows_geolocation(
        document: &web_sys::Document,
    ) -> Option<bool> {
        let policy = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("permissionsPolicy"))
            .ok()?;
        if policy.is_null() || policy.is_undefined() {
            return None;
        }
        let allows = js_sys::Reflect::get(&policy, &JsValue::from_str("allowsFeature"))
            .ok()?;
        let allows = allows.dyn_into::<js_sys::Function>().ok()?;
        let result = allows
            .call1(&policy, &JsValue::from_str("geolocation"))
            .ok()?;
        result.as_bool()
    }

    #[cfg(target_arch = "wasm32")]
    async fn permission_state(
        navigator: &web_sys::Navigator,
    ) -> Option<web_sys::PermissionState> {
        let permissions = navigator.permissions().ok()?;
        let descriptor = web_sys::PermissionDescriptor::new(web_sys::PermissionName::Geolocation);
        let promise = permissions.query(&descriptor).ok()?;
        let status = JsFuture::from(promise).await.ok()?;
        let status: web_sys::PermissionStatus = status.dyn_into().ok()?;
        Some(status.state())
    }

    #[cfg(target_arch = "wasm32")]
    async fn get_current_position(
        geolocation: &web_sys::Geolocation,
    ) -> Result<web_sys::Position, JsValue> {
        let geolocation = geolocation.clone();
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let success = wasm_bindgen::closure::Closure::once(
                move |position: web_sys::Position| {
                    let _ = resolve.call1(&JsValue::NULL, &position);
                },
            );
            let reject_failure = reject.clone();
            let failure = wasm_bindgen::closure::Closure::once(
                move |error: web_sys::PositionError| {
                    let _ = reject_failure.call1(&JsValue::NULL, &error);
                },
            );
            let options = web_sys::PositionOptions::new();
            options.set_enable_high_accuracy(true);
            options.set_timeout(10_000);
            options.set_maximum_age(30_000);
            if geolocation
                .get_current_position_with_error_callback_and_options(
                    success.as_ref().unchecked_ref(),
                    Some(failure.as_ref().unchecked_ref()),
                    &options,
                )
                .is_err()
            {
                let _ = reject.call0(&JsValue::NULL);
            }
            success.forget();
            failure.forget();
        });
        let value = JsFuture::from(promise).await?;
        value.dyn_into::<web_sys::Position>()
    }

    #[cfg(target_arch = "wasm32")]
    fn map_error(
        policy_allows: Option<bool>,
        error: &web_sys::PositionError,
    ) -> RadrootsClientGeolocationError {
        match error.code() {
            web_sys::PositionError::PERMISSION_DENIED => {
                if policy_allows == Some(false) {
                    RadrootsClientGeolocationError::BlockedByPermissionsPolicy
                } else {
                    RadrootsClientGeolocationError::PermissionDenied
                }
            }
            web_sys::PositionError::POSITION_UNAVAILABLE => {
                RadrootsClientGeolocationError::PositionUnavailable
            }
            web_sys::PositionError::TIMEOUT => RadrootsClientGeolocationError::Timeout,
            _ => RadrootsClientGeolocationError::UnknownError,
        }
    }
}

#[async_trait(?Send)]
impl RadrootsClientGeolocation for RadrootsClientWebGeolocation {
    async fn current(
        &self,
    ) -> RadrootsClientGeolocationResult<RadrootsClientGeolocationPosition> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientGeolocationError::LocationUnavailable);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window =
                web_sys::window().ok_or(RadrootsClientGeolocationError::LocationUnavailable)?;
            let document = window
                .document()
                .ok_or(RadrootsClientGeolocationError::LocationUnavailable)?;
            let navigator = window.navigator();
            let geolocation = navigator
                .geolocation()
                .map_err(|_| RadrootsClientGeolocationError::LocationUnavailable)?;

            let policy_allows = Self::policy_allows_geolocation(&document);
            let _ = Self::permission_state(&navigator).await;

            if policy_allows == Some(false) {
                return Err(RadrootsClientGeolocationError::BlockedByPermissionsPolicy);
            }

            match Self::get_current_position(&geolocation).await {
                Ok(position) => {
                    let coords = position.coords();
                    Ok(RadrootsClientGeolocationPosition {
                        lat: coords.latitude(),
                        lng: coords.longitude(),
                        accuracy: Some(coords.accuracy()),
                        altitude: coords.altitude(),
                        altitude_accuracy: coords.altitude_accuracy(),
                    })
                }
                Err(err) => {
                    if let Ok(position_error) = err.dyn_into::<web_sys::PositionError>() {
                        return Err(Self::map_error(policy_allows, &position_error));
                    }
                    Err(RadrootsClientGeolocationError::UnknownError)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebGeolocation;
    use crate::geolocation::{
        RadrootsClientGeolocation,
        RadrootsClientGeolocationError,
    };

    #[test]
    fn non_wasm_current_errors() {
        let geo = RadrootsClientWebGeolocation;
        let err = futures::executor::block_on(geo.current())
            .expect_err("location unavailable");
        assert_eq!(err, RadrootsClientGeolocationError::LocationUnavailable);
    }
}
