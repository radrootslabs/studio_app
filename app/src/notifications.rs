#![forbid(unsafe_code)]

use radroots_studio_app_core::notifications::{
    RadrootsClientNotifications,
    RadrootsClientNotificationsConfig,
    RadrootsClientNotificationsDialogConfirmOpts,
    RadrootsClientNotificationsError,
    RadrootsClientNotificationsPermission,
    RadrootsClientWebNotifications,
};

use crate::app_log_debug_emit;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppNotificationsError {
    Notifications(RadrootsClientNotificationsError),
}

pub type RadrootsAppNotificationsResult<T> = Result<T, RadrootsAppNotificationsError>;

impl RadrootsAppNotificationsError {
    pub const fn message(self) -> &'static str {
        match self {
            RadrootsAppNotificationsError::Notifications(err) => err.message(),
        }
    }
}

impl std::fmt::Display for RadrootsAppNotificationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppNotificationsError {}

impl From<RadrootsClientNotificationsError> for RadrootsAppNotificationsError {
    fn from(err: RadrootsClientNotificationsError) -> Self {
        RadrootsAppNotificationsError::Notifications(err)
    }
}

pub struct RadrootsAppNotifications {
    client: RadrootsClientWebNotifications,
}

impl RadrootsAppNotifications {
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
    ) -> RadrootsAppNotificationsResult<RadrootsClientNotificationsPermission> {
        let _ = app_log_debug_emit("log.app.notifications.permission", "start", None);
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
            let permission = Self::permission_from_web(web_sys::Notification::permission());
            let _ = app_log_debug_emit(
                "log.app.notifications.permission",
                "resolved",
                Some(permission.as_str().to_string()),
            );
            Ok(permission)
        }
    }

    pub async fn request_permission(
        &self,
    ) -> RadrootsAppNotificationsResult<RadrootsClientNotificationsPermission> {
        let _ = app_log_debug_emit("log.app.notifications.request", "start", None);
        let result = self.client
            .notify_init()
            .await
            .map_err(RadrootsAppNotificationsError::from);
        if let Ok(permission) = &result {
            let _ = app_log_debug_emit(
                "log.app.notifications.request",
                "resolved",
                Some(permission.as_str().to_string()),
            );
        }
        result
    }

    pub async fn confirm_message(&self, message: &str) -> bool {
        self.client
            .confirm(RadrootsClientNotificationsDialogConfirmOpts::Message(
                message.to_string(),
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{RadrootsAppNotifications, RadrootsAppNotificationsError};
    use radroots_studio_app_core::notifications::{
        RadrootsClientNotificationsConfig,
        RadrootsClientNotificationsError,
        RadrootsClientNotificationsPermission,
    };

    #[test]
    fn permission_is_unavailable_on_native() {
        let app = RadrootsAppNotifications::new(Some(RadrootsClientNotificationsConfig {
            app_name: String::from("Radroots"),
        }));
        let permission = futures::executor::block_on(app.permission())
            .expect("permission");
        assert_eq!(permission, RadrootsClientNotificationsPermission::Unavailable);
    }

    #[test]
    fn request_permission_maps_errors() {
        let app = RadrootsAppNotifications::new(None);
        let err = futures::executor::block_on(app.request_permission())
            .expect_err("permission request error");
        assert_eq!(
            err,
            RadrootsAppNotificationsError::Notifications(RadrootsClientNotificationsError::Unavailable)
        );
        assert_eq!(err.to_string(), "error.client.notifications.unavailable");
    }
}
