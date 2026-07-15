use radroots_studio_app_view::{
    ActivityEventId, AppActivityContext, AppActivityEvent, AppActivityKind, SettingsPreference,
    SettingsSection,
};
use sqlx::Row;

use crate::{AppSqliteDatabase, AppSqliteError};

pub const APP_ACTIVITY_CONTEXT_LIMIT: usize = 64;
pub const APP_ACTIVITY_RETENTION_LIMIT: i64 = 5_000;

pub struct AppActivityRepository<'a> {
    connection: &'a AppSqliteDatabase,
}

impl<'a> AppActivityRepository<'a> {
    pub(crate) fn new(connection: &'a AppSqliteDatabase) -> Self {
        Self { connection }
    }

    pub fn record(&self, kind: &AppActivityKind) -> Result<(), AppSqliteError> {
        let activity_event_id = ActivityEventId::generate().to_string();
        let event_kind = kind.storage_key();
        let settings_section = settings_section_value(kind);
        let settings_preference = settings_preference_value(kind);
        let preference_enabled = preference_enabled_value(kind);

        self.connection
            .execute_statement(
                "INSERT INTO activity_events (
                    activity_event_id,
                    event_kind,
                    settings_section,
                    settings_preference,
                    preference_enabled
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                crate::app_sqlite_params![
                    activity_event_id,
                    event_kind,
                    settings_section,
                    settings_preference,
                    preference_enabled,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record activity event",
                source,
            })?;

        self.trim_retained_events(APP_ACTIVITY_RETENTION_LIMIT)?;

        Ok(())
    }

    pub fn load_recent(&self, limit: usize) -> Result<Vec<AppActivityEvent>, AppSqliteError> {
        let query_limit = i64::try_from(limit).map_err(|_| AppSqliteError::InvalidProjection {
            reason: "activity query limit exceeds sqlite integer range",
        })?;
        let rows = self
            .connection
            .fetch_mapped(
                "SELECT
                    activity_event_id,
                    recorded_at,
                    event_kind,
                    settings_section,
                    settings_preference,
                    preference_enabled
                 FROM activity_events
                 ORDER BY recorded_at DESC, activity_event_id DESC
                 LIMIT ?1",
                crate::app_sqlite_params![query_limit],
                |row| {
                    let activity_event_id = row.try_get::<String, _>(0)?;
                    let recorded_at = row.try_get::<String, _>(1)?;
                    let event_kind = row.try_get::<String, _>(2)?;
                    let settings_section = row.try_get::<Option<String>, _>(3)?;
                    let settings_preference = row.try_get::<Option<String>, _>(4)?;
                    let preference_enabled = row.try_get::<Option<i64>, _>(5)?;

                    Ok((
                        activity_event_id,
                        recorded_at,
                        event_kind,
                        settings_section,
                        settings_preference,
                        preference_enabled,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query recent activity events",
                source,
            })?;

        rows.map(|row| {
            let (
                activity_event_id,
                recorded_at,
                event_kind,
                settings_section,
                settings_preference,
                preference_enabled,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read recent activity event row",
                source,
            })?;

            decode_activity_event(
                &activity_event_id,
                recorded_at,
                event_kind,
                settings_section,
                settings_preference,
                preference_enabled,
            )
        })
        .collect()
    }

    pub fn load_context(&self, limit: usize) -> Result<AppActivityContext, AppSqliteError> {
        Ok(AppActivityContext::from_recent_events(
            self.load_recent(limit)?,
        ))
    }

    fn trim_retained_events(&self, retention_limit: i64) -> Result<(), AppSqliteError> {
        self.connection
            .execute_statement(
                "DELETE FROM activity_events
                 WHERE activity_event_id IN (
                     SELECT activity_event_id
                     FROM activity_events
                     ORDER BY recorded_at DESC, activity_event_id DESC
                     LIMIT -1 OFFSET ?1
                 )",
                crate::app_sqlite_params![retention_limit],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "trim retained activity events",
                source,
            })?;

        Ok(())
    }
}

fn decode_activity_event(
    activity_event_id: &str,
    recorded_at: String,
    event_kind: String,
    settings_section: Option<String>,
    settings_preference: Option<String>,
    preference_enabled: Option<i64>,
) -> Result<AppActivityEvent, AppSqliteError> {
    let kind = match event_kind.as_str() {
        "home_opened" => AppActivityKind::HomeOpened,
        "settings_opened" => AppActivityKind::SettingsOpened {
            section: decode_settings_section("settings_section", settings_section)?,
        },
        "settings_section_selected" => AppActivityKind::SettingsSectionSelected {
            section: decode_settings_section("settings_section", settings_section)?,
        },
        "settings_preference_updated" => AppActivityKind::SettingsPreferenceUpdated {
            preference: decode_settings_preference("settings_preference", settings_preference)?,
            enabled: decode_preference_enabled(preference_enabled)?,
        },
        other => {
            return Err(AppSqliteError::DecodeEnum {
                field: "event_kind",
                value: other.to_owned(),
            });
        }
    };

    Ok(AppActivityEvent {
        activity_event_id: activity_event_id
            .parse()
            .map_err(|_| AppSqliteError::DecodeId {
                field: "activity_event_id",
                value: activity_event_id.to_owned(),
            })?,
        recorded_at,
        kind,
    })
}

fn decode_settings_section(
    field: &'static str,
    value: Option<String>,
) -> Result<SettingsSection, AppSqliteError> {
    match value.as_deref() {
        Some("account") => Ok(SettingsSection::Account),
        Some("farm") => Ok(SettingsSection::Farm),
        Some("settings") => Ok(SettingsSection::Settings),
        Some("about") => Ok(SettingsSection::About),
        Some(other) => Err(AppSqliteError::DecodeEnum {
            field,
            value: other.to_owned(),
        }),
        None => Err(AppSqliteError::MissingColumn { field }),
    }
}

fn decode_settings_preference(
    field: &'static str,
    value: Option<String>,
) -> Result<SettingsPreference, AppSqliteError> {
    match value.as_deref() {
        Some("allow_relay_connections") => Ok(SettingsPreference::AllowRelayConnections),
        Some("use_media_servers") => Ok(SettingsPreference::UseMediaServers),
        Some("use_nip05") => Ok(SettingsPreference::UseNip05),
        Some("launch_at_login") => Ok(SettingsPreference::LaunchAtLogin),
        Some(other) => Err(AppSqliteError::DecodeEnum {
            field,
            value: other.to_owned(),
        }),
        None => Err(AppSqliteError::MissingColumn { field }),
    }
}

fn decode_preference_enabled(value: Option<i64>) -> Result<bool, AppSqliteError> {
    match value {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        Some(other) => Err(AppSqliteError::DecodeEnum {
            field: "preference_enabled",
            value: other.to_string(),
        }),
        None => Err(AppSqliteError::MissingColumn {
            field: "preference_enabled",
        }),
    }
}

fn settings_section_value(kind: &AppActivityKind) -> Option<&'static str> {
    match kind {
        AppActivityKind::SettingsOpened { section }
        | AppActivityKind::SettingsSectionSelected { section } => Some(match section {
            SettingsSection::Account => "account",
            SettingsSection::Farm => "farm",
            SettingsSection::Settings => "settings",
            SettingsSection::About => "about",
        }),
        _ => None,
    }
}

fn settings_preference_value(kind: &AppActivityKind) -> Option<&'static str> {
    match kind {
        AppActivityKind::SettingsPreferenceUpdated { preference, .. } => {
            Some(preference.storage_key())
        }
        _ => None,
    }
}

fn preference_enabled_value(kind: &AppActivityKind) -> Option<i64> {
    match kind {
        AppActivityKind::SettingsPreferenceUpdated { enabled, .. } => Some(i64::from(*enabled)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_view::{AppActivityKind, SettingsPreference, SettingsSection};
    use sqlx::Row;

    use crate::{AppSqliteDatabase, AppSqliteStore, DatabaseTarget, empty_params};

    use super::{APP_ACTIVITY_CONTEXT_LIMIT, APP_ACTIVITY_RETENTION_LIMIT, AppActivityRepository};

    #[test]
    fn activity_repository_records_and_loads_typed_recent_events() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.activity_repository();

        repository
            .record(&AppActivityKind::HomeOpened)
            .expect("record home opened");
        repository
            .record(&AppActivityKind::SettingsOpened {
                section: SettingsSection::Farm,
            })
            .expect("record settings opened");
        repository
            .record(&AppActivityKind::SettingsPreferenceUpdated {
                preference: SettingsPreference::LaunchAtLogin,
                enabled: true,
            })
            .expect("record settings preference");

        let recent = repository.load_recent(8).expect("load recent events");

        assert_eq!(recent.len(), 3);
        assert_eq!(
            recent[0].kind,
            AppActivityKind::SettingsPreferenceUpdated {
                preference: SettingsPreference::LaunchAtLogin,
                enabled: true,
            }
        );
        assert_eq!(
            recent[1].kind,
            AppActivityKind::SettingsOpened {
                section: SettingsSection::Farm,
            }
        );
        assert_eq!(recent[2].kind, AppActivityKind::HomeOpened);
    }

    #[test]
    fn activity_repository_load_context_uses_default_context_limit() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.activity_repository();

        repository
            .record(&AppActivityKind::HomeOpened)
            .expect("record home opened");

        let context = repository
            .load_context(APP_ACTIVITY_CONTEXT_LIMIT)
            .expect("load activity context");

        assert_eq!(context.recent_events.len(), 1);
        assert_eq!(context.recent_events[0].kind, AppActivityKind::HomeOpened);
    }

    #[test]
    fn activity_repository_trims_events_to_retention_limit() {
        let connection = AppSqliteDatabase::open_in_memory().expect("open in-memory connection");
        connection
            .execute_script(include_str!("../../migrations/0001_init.sql"))
            .expect("apply init migration");
        connection
            .execute_script(include_str!("../../migrations/0002_activity_journal.sql"))
            .expect("apply activity migration");
        let repository = AppActivityRepository::new(&connection);

        for _ in 0..(APP_ACTIVITY_RETENTION_LIMIT + 8) {
            repository
                .record(&AppActivityKind::HomeOpened)
                .expect("record activity event");
        }

        let retained = count_rows(&connection, "activity_events");

        assert_eq!(retained, APP_ACTIVITY_RETENTION_LIMIT);
    }

    fn count_rows(connection: &AppSqliteDatabase, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        connection
            .fetch_one(&sql, empty_params(), |row| row.try_get(0))
            .expect("row count query should succeed")
    }
}
