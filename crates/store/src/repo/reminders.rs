use radroots_studio_app_view::{
    FarmId, ReminderDeadlineProjection, ReminderDeliveryState, ReminderFeedProjection,
    ReminderKind, ReminderLogEntryProjection, ReminderLogProjection, ReminderSurface,
    ReminderUrgency,
};
use sqlx::Row;

use crate::AppSqliteDatabase;
use std::str::FromStr;
use uuid::Uuid;

use crate::AppSqliteError;

pub struct AppRemindersRepository<'a> {
    connection: &'a AppSqliteDatabase,
}

impl<'a> AppRemindersRepository<'a> {
    pub(crate) const fn new(connection: &'a AppSqliteDatabase) -> Self {
        Self { connection }
    }

    pub fn load_reminder_schedule(
        &self,
        account_id: &str,
        farm_id: FarmId,
    ) -> Result<ReminderFeedProjection, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    reminder_id,
                    order_id,
                    fulfillment_window_id,
                    reminder_kind,
                    reminder_surface,
                    reminder_urgency,
                    title,
                    detail,
                    deadline_at,
                    action_label,
                    delivery_state
                 FROM reminder_schedules
                 WHERE account_id = ?1 AND farm_id = ?2
                 ORDER BY deadline_at ASC, reminder_id ASC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare reminder schedule query",
                source,
            })?;
        let rows = statement
            .query_map(
                crate::app_sqlite_params![account_id, farm_id.to_string()],
                |row| {
                    Ok((
                        row.try_get::<String, _>(0)?,
                        row.try_get::<Option<String>, _>(1)?,
                        row.try_get::<Option<String>, _>(2)?,
                        row.try_get::<String, _>(3)?,
                        row.try_get::<String, _>(4)?,
                        row.try_get::<String, _>(5)?,
                        row.try_get::<String, _>(6)?,
                        row.try_get::<String, _>(7)?,
                        row.try_get::<String, _>(8)?,
                        row.try_get::<Option<String>, _>(9)?,
                        row.try_get::<String, _>(10)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query reminder schedule",
                source,
            })?;

        let items = rows
            .map(|row| {
                let (
                    reminder_id,
                    order_id,
                    fulfillment_window_id,
                    reminder_kind,
                    reminder_surface,
                    reminder_urgency,
                    title,
                    detail,
                    deadline_at,
                    action_label,
                    delivery_state,
                ) = row.map_err(|source| AppSqliteError::Query {
                    operation: "read reminder schedule row",
                    source,
                })?;

                Ok(ReminderDeadlineProjection {
                    reminder_id: parse_typed_id("reminder_schedules.reminder_id", reminder_id)?,
                    farm_id,
                    order_id: parse_optional_typed_id("reminder_schedules.order_id", order_id)?,
                    fulfillment_window_id: parse_optional_typed_id(
                        "reminder_schedules.fulfillment_window_id",
                        fulfillment_window_id,
                    )?,
                    kind: parse_reminder_kind(reminder_kind)?,
                    surface: parse_reminder_surface(reminder_surface)?,
                    urgency: parse_reminder_urgency(reminder_urgency)?,
                    title,
                    detail,
                    deadline_at,
                    action_label,
                    delivery_state: parse_reminder_delivery_state(delivery_state)?,
                })
            })
            .collect::<Result<Vec<_>, AppSqliteError>>()?;

        Ok(ReminderFeedProjection { items })
    }

    pub fn replace_reminder_schedule(
        &self,
        account_id: &str,
        farm_id: FarmId,
        projection: &ReminderFeedProjection,
    ) -> Result<(), AppSqliteError> {
        self.apply_reminder_schedule_update(account_id, farm_id, projection, &[])
    }

    pub fn apply_reminder_schedule_update(
        &self,
        account_id: &str,
        farm_id: FarmId,
        projection: &ReminderFeedProjection,
        log_entries: &[ReminderLogEntryProjection],
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|source| AppSqliteError::Query {
                operation: "begin reminder schedule replacement",
                source,
            })?;

        let result = (|| -> Result<(), AppSqliteError> {
            self.connection
                .execute(
                    "DELETE FROM reminder_schedules WHERE account_id = ?1 AND farm_id = ?2",
                    crate::app_sqlite_params![account_id, farm_id.to_string()],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "clear reminder schedule",
                    source,
                })?;

            let mut statement = self
                .connection
                .prepare(
                    "INSERT INTO reminder_schedules (
                        reminder_id,
                        account_id,
                        farm_id,
                        order_id,
                        fulfillment_window_id,
                        reminder_kind,
                        reminder_surface,
                        reminder_urgency,
                        title,
                        detail,
                        deadline_at,
                        action_label,
                        delivery_state
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "prepare reminder schedule insert",
                    source,
                })?;

            for reminder in &projection.items {
                statement
                    .execute(crate::app_sqlite_params![
                        reminder.reminder_id.to_string(),
                        account_id,
                        reminder.farm_id.to_string(),
                        reminder.order_id.map(|value| value.to_string()),
                        reminder
                            .fulfillment_window_id
                            .map(|value| value.to_string()),
                        reminder.kind.storage_key(),
                        reminder.surface.storage_key(),
                        reminder.urgency.storage_key(),
                        reminder.title,
                        reminder.detail,
                        reminder.deadline_at,
                        reminder.action_label,
                        reminder.delivery_state.storage_key(),
                    ])
                    .map_err(|source| AppSqliteError::Query {
                        operation: "insert reminder schedule row",
                        source,
                    })?;
            }

            for entry in log_entries {
                let log_entry_id = Uuid::now_v7().to_string();

                self.connection
                    .execute(
                        "INSERT INTO reminder_log_entries (
                        log_entry_id,
                        account_id,
                        farm_id,
                        reminder_id,
                        reminder_kind,
                        title,
                        recorded_at,
                        delivery_state,
                        detail
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        crate::app_sqlite_params![
                            log_entry_id,
                            account_id,
                            farm_id.to_string(),
                            entry.reminder_id.to_string(),
                            entry.kind.storage_key(),
                            entry.title,
                            entry.recorded_at,
                            entry.delivery_state.storage_key(),
                            entry.detail,
                        ],
                    )
                    .map_err(|source| AppSqliteError::Query {
                        operation: "record reminder log entry",
                        source,
                    })?;
            }

            Ok(())
        })();

        if let Err(error) = result {
            let _ = self.connection.execute_batch("ROLLBACK");
            return Err(error);
        }

        self.connection
            .execute_batch("COMMIT")
            .map_err(|source| AppSqliteError::Query {
                operation: "commit reminder schedule replacement",
                source,
            })?;

        Ok(())
    }

    pub fn record_reminder_log_entry(
        &self,
        account_id: &str,
        farm_id: FarmId,
        entry: &ReminderLogEntryProjection,
    ) -> Result<String, AppSqliteError> {
        let log_entry_id = Uuid::now_v7().to_string();

        self.connection
            .execute(
                "INSERT INTO reminder_log_entries (
                    log_entry_id,
                    account_id,
                    farm_id,
                    reminder_id,
                    reminder_kind,
                    title,
                    recorded_at,
                    delivery_state,
                    detail
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                crate::app_sqlite_params![
                    log_entry_id,
                    account_id,
                    farm_id.to_string(),
                    entry.reminder_id.to_string(),
                    entry.kind.storage_key(),
                    entry.title,
                    entry.recorded_at,
                    entry.delivery_state.storage_key(),
                    entry.detail,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record reminder log entry",
                source,
            })?;

        Ok(log_entry_id)
    }

    pub fn load_reminder_log(
        &self,
        account_id: &str,
        farm_id: FarmId,
        limit: usize,
    ) -> Result<ReminderLogProjection, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    reminder_id,
                    reminder_kind,
                    title,
                    recorded_at,
                    delivery_state,
                    detail
                 FROM reminder_log_entries
                 WHERE account_id = ?1 AND farm_id = ?2
                 ORDER BY recorded_at DESC, log_entry_id DESC
                 LIMIT ?3",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare reminder log query",
                source,
            })?;
        let rows = statement
            .query_map(
                crate::app_sqlite_params![account_id, farm_id.to_string(), limit as i64],
                |row| {
                    Ok((
                        row.try_get::<String, _>(0)?,
                        row.try_get::<String, _>(1)?,
                        row.try_get::<String, _>(2)?,
                        row.try_get::<String, _>(3)?,
                        row.try_get::<String, _>(4)?,
                        row.try_get::<Option<String>, _>(5)?,
                    ))
                },
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "query reminder log",
                source,
            })?;

        let entries = rows
            .map(|row| {
                let (reminder_id, reminder_kind, title, recorded_at, delivery_state, detail) = row
                    .map_err(|source| AppSqliteError::Query {
                        operation: "read reminder log row",
                        source,
                    })?;

                Ok(ReminderLogEntryProjection {
                    reminder_id: parse_typed_id("reminder_log_entries.reminder_id", reminder_id)?,
                    kind: parse_reminder_kind(reminder_kind)?,
                    title,
                    recorded_at,
                    delivery_state: parse_reminder_delivery_state(delivery_state)?,
                    detail,
                })
            })
            .collect::<Result<Vec<_>, AppSqliteError>>()?;

        Ok(ReminderLogProjection { entries })
    }
}

fn parse_reminder_kind(value: String) -> Result<ReminderKind, AppSqliteError> {
    match value.as_str() {
        "fulfillment_window" => Ok(ReminderKind::FulfillmentWindow),
        "order_action" => Ok(ReminderKind::OrderAction),
        "sync_impact" => Ok(ReminderKind::SyncImpact),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "reminder_schedules.reminder_kind",
            value,
        }),
    }
}

fn parse_reminder_surface(value: String) -> Result<ReminderSurface, AppSqliteError> {
    match value.as_str() {
        "today" => Ok(ReminderSurface::Today),
        "orders" => Ok(ReminderSurface::Orders),
        "pack_day" => Ok(ReminderSurface::PackDay),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "reminder_schedules.reminder_surface",
            value,
        }),
    }
}

fn parse_reminder_urgency(value: String) -> Result<ReminderUrgency, AppSqliteError> {
    match value.as_str() {
        "upcoming" => Ok(ReminderUrgency::Upcoming),
        "due_soon" => Ok(ReminderUrgency::DueSoon),
        "overdue" => Ok(ReminderUrgency::Overdue),
        "blocking" => Ok(ReminderUrgency::Blocking),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "reminder_schedules.reminder_urgency",
            value,
        }),
    }
}

fn parse_reminder_delivery_state(value: String) -> Result<ReminderDeliveryState, AppSqliteError> {
    match value.as_str() {
        "scheduled" => Ok(ReminderDeliveryState::Scheduled),
        "presented" => Ok(ReminderDeliveryState::Presented),
        "acknowledged" => Ok(ReminderDeliveryState::Acknowledged),
        "resolved" => Ok(ReminderDeliveryState::Resolved),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "reminder delivery_state",
            value,
        }),
    }
}

fn parse_typed_id<T>(field: &'static str, value: String) -> Result<T, AppSqliteError>
where
    T: FromStr<Err = uuid::Error>,
{
    T::from_str(&value).map_err(|_| AppSqliteError::DecodeId { field, value })
}

fn parse_optional_typed_id<T>(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<T>, AppSqliteError>
where
    T: FromStr<Err = uuid::Error>,
{
    value.map(|value| parse_typed_id(field, value)).transpose()
}

#[cfg(test)]
mod tests {
    use super::AppRemindersRepository;
    use crate::{AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_view::{
        FarmId, OrderId, ReminderDeadlineProjection, ReminderDeliveryState, ReminderFeedProjection,
        ReminderId, ReminderKind, ReminderLogEntryProjection, ReminderSurface, ReminderUrgency,
    };

    #[test]
    fn reminder_schedule_round_trips_and_is_account_scoped() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppRemindersRepository::new(store.connection());
        let farm_id = FarmId::generate();
        let other_farm_id = FarmId::generate();
        let order_id = OrderId::generate();
        let reminder = ReminderDeadlineProjection {
            reminder_id: ReminderId::generate(),
            farm_id,
            order_id: Some(order_id),
            fulfillment_window_id: None,
            kind: ReminderKind::OrderAction,
            surface: ReminderSurface::Orders,
            urgency: ReminderUrgency::DueSoon,
            title: "Pack CSA order".to_owned(),
            detail: "Order R-1001 still needs packing.".to_owned(),
            deadline_at: "2026-04-25T14:00:00Z".to_owned(),
            action_label: Some("Review order".to_owned()),
            delivery_state: ReminderDeliveryState::Scheduled,
        };

        repository
            .replace_reminder_schedule(
                "acct_farmer",
                farm_id,
                &ReminderFeedProjection {
                    items: vec![reminder.clone()],
                },
            )
            .expect("schedule should save");
        repository
            .replace_reminder_schedule(
                "acct_other",
                other_farm_id,
                &ReminderFeedProjection {
                    items: vec![ReminderDeadlineProjection {
                        farm_id: other_farm_id,
                        ..reminder.clone()
                    }],
                },
            )
            .expect("other schedule should save");

        let loaded = repository
            .load_reminder_schedule("acct_farmer", farm_id)
            .expect("schedule should load");
        let other = repository
            .load_reminder_schedule("acct_other", other_farm_id)
            .expect("other schedule should load");

        assert_eq!(loaded.items, vec![reminder]);
        assert_eq!(other.items.len(), 1);
        assert_eq!(other.items[0].reminder_id, loaded.items[0].reminder_id);
        assert_eq!(other.items[0].farm_id, other_farm_id);
    }

    #[test]
    fn reminder_log_records_and_loads_recent_entries() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppRemindersRepository::new(store.connection());
        let farm_id = FarmId::generate();
        let first_reminder_id = ReminderId::generate();
        let second_reminder_id = ReminderId::generate();

        repository
            .record_reminder_log_entry(
                "acct_farmer",
                farm_id,
                &ReminderLogEntryProjection {
                    reminder_id: first_reminder_id,
                    kind: ReminderKind::FulfillmentWindow,
                    title: "Window closes today".to_owned(),
                    recorded_at: "2026-04-25T12:00:00Z".to_owned(),
                    delivery_state: ReminderDeliveryState::Presented,
                    detail: None,
                },
            )
            .expect("first log entry should save");
        repository
            .record_reminder_log_entry(
                "acct_farmer",
                farm_id,
                &ReminderLogEntryProjection {
                    reminder_id: second_reminder_id,
                    kind: ReminderKind::SyncImpact,
                    title: "Sync attention needed".to_owned(),
                    recorded_at: "2026-04-25T13:00:00Z".to_owned(),
                    delivery_state: ReminderDeliveryState::Acknowledged,
                    detail: Some("A local sync issue needs review.".to_owned()),
                },
            )
            .expect("second log entry should save");

        let loaded = repository
            .load_reminder_log("acct_farmer", farm_id, 1)
            .expect("log should load");

        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].reminder_id, second_reminder_id);
        assert_eq!(
            loaded.entries[0].delivery_state,
            ReminderDeliveryState::Acknowledged
        );
    }
}
