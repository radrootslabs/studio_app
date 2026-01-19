use async_trait::async_trait;

use super::RadrootsClientNotificationsError;

pub type RadrootsClientNotificationsResult<T> =
    Result<T, RadrootsClientNotificationsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientResolveStatus {
    Info,
    Warning,
    Error,
    Success,
}

impl RadrootsClientResolveStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsClientResolveStatus::Info => "info",
            RadrootsClientResolveStatus::Warning => "warning",
            RadrootsClientResolveStatus::Error => "error",
            RadrootsClientResolveStatus::Success => "success",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "info" => Some(RadrootsClientResolveStatus::Info),
            "warning" => Some(RadrootsClientResolveStatus::Warning),
            "error" => Some(RadrootsClientResolveStatus::Error),
            "success" => Some(RadrootsClientResolveStatus::Success),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientNotificationsPermission {
    Granted,
    Denied,
    Default,
    Unavailable,
}

impl RadrootsClientNotificationsPermission {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsClientNotificationsPermission::Granted => "granted",
            RadrootsClientNotificationsPermission::Denied => "denied",
            RadrootsClientNotificationsPermission::Default => "default",
            RadrootsClientNotificationsPermission::Unavailable => "unavailable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "granted" => Some(RadrootsClientNotificationsPermission::Granted),
            "denied" => Some(RadrootsClientNotificationsPermission::Denied),
            "default" => Some(RadrootsClientNotificationsPermission::Default),
            "unavailable" => Some(RadrootsClientNotificationsPermission::Unavailable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientNotificationsDialogConfirmConfig {
    pub message: String,
    pub title: Option<String>,
    pub status: Option<RadrootsClientResolveStatus>,
    pub cancel: Option<String>,
    pub ok: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsClientNotificationsDialogConfirmOpts {
    Message(String),
    Config(RadrootsClientNotificationsDialogConfirmConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientNotificationsSendOptions {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientNotificationsConfig {
    pub app_name: String,
}

#[async_trait(?Send)]
pub trait RadrootsClientNotifications {
    async fn alert(
        &self,
        message: &str,
        title: Option<&str>,
        status: Option<RadrootsClientResolveStatus>,
    ) -> bool;
    async fn confirm(
        &self,
        opts: RadrootsClientNotificationsDialogConfirmOpts,
    ) -> bool;
    async fn notify_init(
        &self,
    ) -> RadrootsClientNotificationsResult<RadrootsClientNotificationsPermission>;
    async fn notify_send(
        &self,
        opts: RadrootsClientNotificationsSendOptions,
    ) -> RadrootsClientNotificationsResult<()>;
    async fn open_photos(
        &self,
    ) -> RadrootsClientNotificationsResult<Option<Vec<String>>>;
}

#[cfg(test)]
mod tests {
    use super::{RadrootsClientNotificationsPermission, RadrootsClientResolveStatus};

    #[test]
    fn resolve_status_roundtrip() {
        let status = RadrootsClientResolveStatus::Warning;
        assert_eq!(status.as_str(), "warning");
        assert_eq!(
            RadrootsClientResolveStatus::parse("warning"),
            Some(status)
        );
        assert_eq!(RadrootsClientResolveStatus::parse("other"), None);
    }

    #[test]
    fn notification_permission_roundtrip() {
        let permission = RadrootsClientNotificationsPermission::Granted;
        assert_eq!(permission.as_str(), "granted");
        assert_eq!(
            RadrootsClientNotificationsPermission::parse("granted"),
            Some(permission)
        );
        assert_eq!(RadrootsClientNotificationsPermission::parse("other"), None);
    }
}
