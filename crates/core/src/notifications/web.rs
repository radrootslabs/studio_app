use async_trait::async_trait;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

use super::{
    RadrootsClientNotifications,
    RadrootsClientNotificationsConfig,
    RadrootsClientNotificationsDialogConfirmOpts,
    RadrootsClientNotificationsError,
    RadrootsClientNotificationsPermission,
    RadrootsClientNotificationsResult,
    RadrootsClientNotificationsSendOptions,
    RadrootsClientResolveStatus,
};

pub struct RadrootsClientWebNotifications {
    config: RadrootsClientNotificationsConfig,
}

impl RadrootsClientWebNotifications {
    pub fn new(config: Option<RadrootsClientNotificationsConfig>) -> Self {
        let config = config.unwrap_or(RadrootsClientNotificationsConfig {
            app_name: String::from("Radroots"),
        });
        Self { config }
    }

    pub fn get_config(&self) -> &RadrootsClientNotificationsConfig {
        &self.config
    }

    #[cfg(target_arch = "wasm32")]
    fn notification_available(window: &web_sys::Window) -> bool {
        js_sys::Reflect::has(window.as_ref(), &JsValue::from_str("Notification"))
            .unwrap_or(false)
    }

    #[cfg(target_arch = "wasm32")]
    fn permission_from_web(permission: web_sys::NotificationPermission) -> RadrootsClientNotificationsPermission {
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

    #[cfg(target_arch = "wasm32")]
    async fn request_permission(
        &self,
        window: &web_sys::Window,
    ) -> RadrootsClientNotificationsResult<RadrootsClientNotificationsPermission> {
        if !Self::notification_available(window) {
            return Ok(RadrootsClientNotificationsPermission::Unavailable);
        }
        let promise = web_sys::Notification::request_permission()
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        let result = JsFuture::from(promise)
            .await
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        if let Some(permission) = result.as_string() {
            if let Some(parsed) = RadrootsClientNotificationsPermission::parse(&permission) {
                return Ok(parsed);
            }
        }
        Ok(Self::permission_from_web(web_sys::Notification::permission()))
    }

    #[cfg(target_arch = "wasm32")]
    async fn read_photo_data(
        &self,
        file: web_sys::File,
    ) -> RadrootsClientNotificationsResult<String> {
        let reader =
            web_sys::FileReader::new().map_err(|_| RadrootsClientNotificationsError::ReadFailure)?;
        let reader_load = reader.clone();
        let reader_error = reader.clone();
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let onload = wasm_bindgen::closure::Closure::once(move |_event: web_sys::Event| {
                match reader_load.result() {
                    Ok(value) => {
                        let _ = resolve.call1(&JsValue::NULL, &value);
                    }
                    Err(err) => {
                        let _ = reject.call1(&JsValue::NULL, &err);
                    }
                }
            });
            let onerror = wasm_bindgen::closure::Closure::once(move |_event: web_sys::Event| {
                let err = reader_error
                    .error()
                    .map(JsValue::from)
                    .unwrap_or_else(|| {
                        JsValue::from_str(RadrootsClientNotificationsError::ReadFailure.message())
                    });
                let _ = reject.call1(&JsValue::NULL, &err);
            });
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onload.forget();
            onerror.forget();
        });
        reader
            .read_as_data_url(&file)
            .map_err(|_| RadrootsClientNotificationsError::ReadFailure)?;
        let result = JsFuture::from(promise)
            .await
            .map_err(|_| RadrootsClientNotificationsError::ReadFailure)?;
        result
            .as_string()
            .ok_or(RadrootsClientNotificationsError::ReadFailure)
    }

    #[cfg(target_arch = "wasm32")]
    async fn select_photo_files(
        &self,
    ) -> RadrootsClientNotificationsResult<Option<web_sys::FileList>> {
        let window = web_sys::window().ok_or(RadrootsClientNotificationsError::Unavailable)?;
        let document = window
            .document()
            .ok_or(RadrootsClientNotificationsError::Unavailable)?;
        let input = document
            .create_element("input")
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        let input: web_sys::HtmlInputElement = input
            .dyn_into()
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        input.set_type("file");
        input.set_multiple(true);
        input.set_accept("image/png,image/jpg");
        let input_handle = input.clone();
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let onchange = wasm_bindgen::closure::Closure::once(move |_event: web_sys::Event| {
                let files = input_handle.files();
                let value = files.map(JsValue::from).unwrap_or(JsValue::NULL);
                let _ = resolve.call1(&JsValue::NULL, &value);
            });
            input_handle.set_onchange(Some(onchange.as_ref().unchecked_ref()));
            input_handle.click();
            onchange.forget();
        });
        let value = JsFuture::from(promise)
            .await
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        let list = value
            .dyn_into::<web_sys::FileList>()
            .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
        Ok(Some(list))
    }
}

#[async_trait(?Send)]
impl RadrootsClientNotifications for RadrootsClientWebNotifications {
    async fn alert(
        &self,
        message: &str,
        title: Option<&str>,
        _status: Option<RadrootsClientResolveStatus>,
    ) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (message, title);
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window = match web_sys::window() {
                Some(window) => window,
                None => return false,
            };
            let msg = if let Some(title) = title {
                format!("{title}\n\n{message}")
            } else {
                message.to_string()
            };
            window.alert_with_message(&msg).is_ok()
        }
    }

    async fn confirm(
        &self,
        opts: RadrootsClientNotificationsDialogConfirmOpts,
    ) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window = match web_sys::window() {
                Some(window) => window,
                None => return false,
            };
            let msg = match opts {
                RadrootsClientNotificationsDialogConfirmOpts::Message(message) => message,
                RadrootsClientNotificationsDialogConfirmOpts::Config(config) => config.message,
            };
            window.confirm_with_message(&msg).unwrap_or(false)
        }
    }

    async fn notify_init(
        &self,
    ) -> RadrootsClientNotificationsResult<RadrootsClientNotificationsPermission> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientNotificationsError::Unavailable);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().ok_or(RadrootsClientNotificationsError::Unavailable)?;
            if !Self::notification_available(&window) {
                return Ok(RadrootsClientNotificationsPermission::Unavailable);
            }
            let permission = Self::permission_from_web(web_sys::Notification::permission());
            match permission {
                RadrootsClientNotificationsPermission::Granted
                | RadrootsClientNotificationsPermission::Denied => Ok(permission),
                RadrootsClientNotificationsPermission::Default => {
                    self.request_permission(&window).await
                }
                RadrootsClientNotificationsPermission::Unavailable => Ok(permission),
            }
        }
    }

    async fn notify_send(
        &self,
        opts: RadrootsClientNotificationsSendOptions,
    ) -> RadrootsClientNotificationsResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = opts;
            return Err(RadrootsClientNotificationsError::Unavailable);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().ok_or(RadrootsClientNotificationsError::Unavailable)?;
            if !Self::notification_available(&window) {
                return Err(RadrootsClientNotificationsError::Unavailable);
            }
            let permission = self.notify_init().await?;
            if permission != RadrootsClientNotificationsPermission::Granted {
                return Err(RadrootsClientNotificationsError::Unavailable);
            }
            let title = opts
                .title
                .as_deref()
                .unwrap_or(&self.config.app_name);
            if let Some(body) = opts.body.as_deref() {
                let mut options = web_sys::NotificationOptions::new();
                options.set_body(body);
                web_sys::Notification::new_with_options(title, &options)
                    .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
            } else {
                web_sys::Notification::new(title)
                    .map_err(|_| RadrootsClientNotificationsError::Unavailable)?;
            }
            Ok(())
        }
    }

    async fn open_photos(
        &self,
    ) -> RadrootsClientNotificationsResult<Option<Vec<String>>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientNotificationsError::Unavailable);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let files = self.select_photo_files().await?;
            let Some(files) = files else {
                return Ok(None);
            };
            let mut results = Vec::new();
            for idx in 0..files.length() {
                let Some(file) = files.item(idx) else {
                    continue;
                };
                let data = self.read_photo_data(file).await?;
                results.push(data);
            }
            Ok(Some(results))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebNotifications;
    use crate::notifications::{
        RadrootsClientNotifications,
        RadrootsClientNotificationsConfig,
        RadrootsClientNotificationsError,
    };

    #[test]
    fn default_config_is_radroots() {
        let client = RadrootsClientWebNotifications::new(None);
        let config = RadrootsClientNotificationsConfig {
            app_name: String::from("Radroots"),
        };
        assert_eq!(client.get_config(), &config);
    }

    #[test]
    fn non_wasm_notify_init_errors() {
        let client = RadrootsClientWebNotifications::new(None);
        let err = futures::executor::block_on(client.notify_init())
            .expect_err("notify init errors");
        assert_eq!(err, RadrootsClientNotificationsError::Unavailable);
    }
}
