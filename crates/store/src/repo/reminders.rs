use radroots_studio_app_view::{
    FarmId, OrderId, OrderRecoveryProjection, RecoveryKind, RecoveryQueueProjection, RecoveryState,
    ReminderDeadlineProjection, ReminderDeliveryState, ReminderFeedProjection, ReminderKind,
    ReminderLogEntryProjection, ReminderLogProjection, ReminderSurface, ReminderUrgency,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::str::FromStr;
use uuid::Uuid;

use crate::AppSqliteError;

pub struct AppRemindersRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppRemindersRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
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
            .query_map(params![account_id, farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                ))
            })
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
        let transaction =
            self.connection
                .unchecked_transaction()
                .map_err(|source| AppSqliteError::Query {
                    operation: "begin reminder schedule replacement",
                    source,
                })?;

        transaction
            .execute(
                "DELETE FROM reminder_schedules WHERE account_id = ?1 AND farm_id = ?2",
                params![account_id, farm_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear reminder schedule",
                source,
            })?;

        {
            let mut statement = transaction
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
                    .execute(params![
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
        }

        for entry in log_entries {
            let log_entry_id = Uuid::now_v7().to_string();

            transaction
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
                    params![
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

        transaction
            .commit()
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
                params![
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
                params![account_id, farm_id.to_string(), limit as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
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

    pub fn load_recovery_queue(
        &self,
        account_id: &str,
        farm_id: FarmId,
    ) -> Result<RecoveryQueueProjection, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    recovery_record_id,
                    order_id,
                    recovery_kind,
                    recovery_state,
                    summary,
                    note,
                    last_updated_at
                 FROM order_recovery_records
                 WHERE account_id = ?1 AND farm_id = ?2
                 ORDER BY last_updated_at DESC, recovery_record_id DESC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare recovery queue query",
                source,
            })?;
        let rows = statement
            .query_map(params![account_id, farm_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query recovery queue",
                source,
            })?;

        let items = rows
            .map(|row| {
                let (
                    recovery_record_id,
                    order_id,
                    recovery_kind,
                    recovery_state,
                    summary,
                    note,
                    last_updated_at,
                ) = row.map_err(|source| AppSqliteError::Query {
                    operation: "read recovery queue row",
                    source,
                })?;

                Ok(OrderRecoveryProjection {
                    recovery_record_id: parse_typed_id(
                        "order_recovery_records.recovery_record_id",
                        recovery_record_id,
                    )?,
                    order_id: parse_typed_id("order_recovery_records.order_id", order_id)?,
                    kind: parse_recovery_kind(recovery_kind)?,
                    state: parse_recovery_state(recovery_state)?,
                    summary,
                    note,
                    last_updated_at,
                })
            })
            .collect::<Result<Vec<_>, AppSqliteError>>()?;

        Ok(RecoveryQueueProjection { items })
    }

    pub fn load_recovery_record(
        &self,
        account_id: &str,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<Option<OrderRecoveryProjection>, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "SELECT
                    recovery_record_id,
                    order_id,
                    recovery_kind,
                    recovery_state,
                    summary,
                    note,
                    last_updated_at
                 FROM order_recovery_records
                 WHERE account_id = ?1 AND order_id = ?2 AND recovery_kind = ?3
                 LIMIT 1",
                params![account_id, order_id.to_string(), kind.storage_key()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load recovery record",
                source,
            })?;

        row.map_or_else(
            || Ok(None),
            |(
                recovery_record_id,
                order_id,
                recovery_kind,
                recovery_state,
                summary,
                note,
                last_updated_at,
            )| {
                Ok(Some(OrderRecoveryProjection {
                    recovery_record_id: parse_typed_id(
                        "order_recovery_records.recovery_record_id",
                        recovery_record_id,
                    )?,
                    order_id: parse_typed_id("order_recovery_records.order_id", order_id)?,
                    kind: parse_recovery_kind(recovery_kind)?,
                    state: parse_recovery_state(recovery_state)?,
                    summary,
                    note,
                    last_updated_at,
                }))
            },
        )
    }

    pub fn save_recovery_record(
        &self,
        account_id: &str,
        farm_id: FarmId,
        record: &OrderRecoveryProjection,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO order_recovery_records (
                    recovery_record_id,
                    account_id,
                    farm_id,
                    order_id,
                    recovery_kind,
                    recovery_state,
                    summary,
                    note,
                    last_updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(account_id, order_id, recovery_kind) DO UPDATE SET
                    recovery_record_id = excluded.recovery_record_id,
                    farm_id = excluded.farm_id,
                    recovery_state = excluded.recovery_state,
                    summary = excluded.summary,
                    note = excluded.note,
                    last_updated_at = excluded.last_updated_at",
                params![
                    record.recovery_record_id.to_string(),
                    account_id,
                    farm_id.to_string(),
                    record.order_id.to_string(),
                    record.kind.storage_key(),
                    record.state.storage_key(),
                    record.summary,
                    record.note,
                    record.last_updated_at,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save recovery record",
                source,
            })?;

        Ok(())
    }
}

fn parse_reminder_kind(value: String) -> Result<ReminderKind, AppSqliteError> {
    match value.as_str() {
        "fulfillment_window" => Ok(ReminderKind::FulfillmentWindow),
        "order_action" => Ok(ReminderKind::OrderAction),
        "missed_pickup_recovery" => Ok(ReminderKind::MissedPickupRecovery),
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

fn parse_recovery_kind(value: String) -> Result<RecoveryKind, AppSqliteError> {
    match value.as_str() {
        "missed_pickup" => Ok(RecoveryKind::MissedPickup),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "order_recovery_records.recovery_kind",
            value,
        }),
    }
}

fn parse_recovery_state(value: String) -> Result<RecoveryState, AppSqliteError> {
    match value.as_str() {
        "open" => Ok(RecoveryState::Open),
        "in_review" => Ok(RecoveryState::InReview),
        "resolved" => Ok(RecoveryState::Resolved),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "order_recovery_records.recovery_state",
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
        FarmId, OrderId, OrderRecoveryProjection, RecoveryKind, RecoveryRecordId, RecoveryState,
        ReminderDeadlineProjection, ReminderDeliveryState, ReminderFeedProjection, ReminderId,
        ReminderKind, ReminderLogEntryProjection, ReminderSurface, ReminderUrgency,
    };

    #[test]
    fn reminder_schedule_round_trips_and_is_account_scoped() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppRemindersRepository::new(store.connection());
        let farm_id = FarmId::new();
        let other_farm_id = FarmId::new();
        let order_id = OrderId::new();
        let reminder = ReminderDeadlineProjection {
            reminder_id: ReminderId::new(),
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
        let farm_id = FarmId::new();
        let first_reminder_id = ReminderId::new();
        let second_reminder_id = ReminderId::new();

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
                    kind: ReminderKind::MissedPickupRecovery,
                    title: "Pickup follow-up pending".to_owned(),
                    recorded_at: "2026-04-25T13:00:00Z".to_owned(),
                    delivery_state: ReminderDeliveryState::Acknowledged,
                    detail: Some("Customer requested a callback.".to_owned()),
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

    #[test]
    fn recovery_records_round_trip_and_upsert_by_account_order_and_kind() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = AppRemindersRepository::new(store.connection());
        let farm_id = FarmId::new();
        let order_id = OrderId::new();

        let first = OrderRecoveryProjection {
            recovery_record_id: RecoveryRecordId::new(),
            order_id,
            kind: RecoveryKind::MissedPickup,
            state: RecoveryState::Open,
            summary: "Customer missed pickup".to_owned(),
            note: Some("Hold until Friday".to_owned()),
            last_updated_at: "2026-04-25T17:00:00Z".to_owned(),
        };
        let updated = OrderRecoveryProjection {
            recovery_record_id: RecoveryRecordId::new(),
            order_id,
            kind: RecoveryKind::MissedPickup,
            state: RecoveryState::InReview,
            summary: "Pickup follow-up underway".to_owned(),
            note: Some("Customer will confirm by tonight".to_owned()),
            last_updated_at: "2026-04-25T18:00:00Z".to_owned(),
        };

        repository
            .save_recovery_record("acct_farmer", farm_id, &first)
            .expect("first recovery should save");
        repository
            .save_recovery_record("acct_farmer", farm_id, &updated)
            .expect("updated recovery should save");

        let loaded = repository
            .load_recovery_queue("acct_farmer", farm_id)
            .expect("recovery queue should load");
        let one = repository
            .load_recovery_record("acct_farmer", order_id, RecoveryKind::MissedPickup)
            .expect("recovery record should load")
            .expect("recovery record should exist");

        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0], updated);
        assert_eq!(one, updated);
    }
}
