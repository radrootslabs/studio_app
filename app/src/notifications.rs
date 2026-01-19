#![forbid(unsafe_code)]

use radroots_studio_app_core::notifications::{
    RadrootsClientNotifications,
    RadrootsClientNotificationsConfig,
    RadrootsClientNotificationsError,
    RadrootsClientNotificationsPermission,
    RadrootsClientWebNotifications,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppNotificationsError {
    Notifications(RadrootsClientNotificationsError),
}

pub type AppNotificationsResult<T> = Result<T, AppNotificationsError>;

impl AppNotificationsError {
    pub const fn message(self) -> &'static str {
        match self {
            AppNotificationsError::Notifications(err) => err.message(),
        }
    }
}

impl std::fmt::Display for AppNotificationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppNotificationsError {}

impl From<RadrootsClientNotificationsError> for AppNotificationsError {
    fn from(err: RadrootsClientNotificationsError) -> Self {
        AppNotificationsError::Notifications(err)
    }
}

pub struct AppNotifications {
    client: RadrootsClientWebNotifications,
}

impl AppNotifications {
    pub fn new(config: Option<RadrootsClientNotificationsConfig>) -> Self {
        Self {
            client: RadrootsClientWebNotifications::new(config),
        }
    }

    pub fn get_config(&self) -> &RadrootsClientNotificationsConfig {
        self.client.get_config()
    }

    #[cfg(target_arch = "wasm32")]
    fn notification_available(window: &web_sys::Window) -> bool {
        js_sys::Reflect::has(window.as_ref(), &JsValue::from_str("Notification"))
            .unwrap_or(false)
    }

    #[cfg(target_arch = "wasm32")]
    fn permission_from_web(
        permission: web_sys::NotificationPermission,
    ) -> RadrootsClientNotificationsPermission {
        match permission {
            web_sys::NotificationPermission::Granted => {
                RadrootsClientNotificationsPermission::Granted
            }
            web_sys::NotificationPermission::Denied => RadrootsClientNotificationsPermission::Denied,
            web_sys::NotificationPermission::Default => {
                RadrootsClientNotificationsPermission::Default
            }
            _ => RadrootsClientNotificationsPermission::Unavailable,
        }
    }

    pub async fn permission(
        &self,
    ) -> AppNotificationsResult<RadrootsClientNotificationsPermission> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Ok(RadrootsClientNotificationsPermission::Unavailable);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window =
                web_sys::window().ok_or(RadrootsClientNotificationsError::Unavailable)?;
            if !Self::notification_available(&window) {
                return Ok(RadrootsClientNotificationsPermission::Unavailable);
            }
            Ok(Self::permission_from_web(web_sys::Notification::permission()))
        }
    }

    pub async fn request_permission(
        &self,
    ) -> AppNotificationsResult<RadrootsClientNotificationsPermission> {
        self.client
            .notify_init()
            .await
            .map_err(AppNotificationsError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppNotifications, AppNotificationsError};
    use radroots_studio_app_core::notifications::{
        RadrootsClientNotificationsConfig,
        RadrootsClientNotificationsError,
        RadrootsClientNotificationsPermission,
    };

    #[test]
    fn permission_is_unavailable_on_native() {
        let app = AppNotifications::new(Some(RadrootsClientNotificationsConfig {
            app_name: String::from("Radroots"),
        }));
        let permission = futures::executor::block_on(app.permission())
            .expect("permission");
        assert_eq!(permission, RadrootsClientNotificationsPermission::Unavailable);
    }

    #[test]
    fn request_permission_maps_errors() {
        let app = AppNotifications::new(None);
        let err = futures::executor::block_on(app.request_permission())
            .expect_err("permission request error");
        assert_eq!(
            err,
            AppNotificationsError::Notifications(RadrootsClientNotificationsError::Unavailable)
        );
        assert_eq!(err.to_string(), "error.client.notifications.unavailable");
    }
}
