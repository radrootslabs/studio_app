use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientNotificationsError {
    Unavailable,
    ReadFailure,
}

pub type RadrootsClientNotificationsErrorMessage = &'static str;

impl RadrootsClientNotificationsError {
    pub const fn message(self) -> RadrootsClientNotificationsErrorMessage {
        match self {
            RadrootsClientNotificationsError::Unavailable => {
                "error.client.notifications.unavailable"
            }
            RadrootsClientNotificationsError::ReadFailure => {
                "error.client.notifications.read_failure"
            }
        }
    }
}

impl fmt::Display for RadrootsClientNotificationsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientNotificationsError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientNotificationsError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientNotificationsError::Unavailable,
                "error.client.notifications.unavailable",
            ),
            (
                RadrootsClientNotificationsError::ReadFailure,
                "error.client.notifications.read_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
