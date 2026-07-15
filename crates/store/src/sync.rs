use radroots_studio_app_sync::{
    AppRelayIngestFreshnessState, AppRelayIngestRelayFreshness, AppRelayIngestScopeFreshness,
    AppRelayIngestScopeStatus, PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef,
    SyncCheckpointState, SyncCheckpointStatus, SyncConflict, SyncConflictKind,
    SyncConflictResolutionStatus, SyncConflictSeverity, SyncOperationKind,
};
use radroots_studio_app_view::{FarmId, FulfillmentWindowId, OrderId, ProductId};
use sqlx::Row;

use crate::{AppSqliteDatabase, OptionalSqliteResult};
use uuid::Uuid;

use crate::AppSqliteError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredPendingSyncOperation {
    pub operation_id: String,
    pub operation: PendingSyncOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSyncConflict {
    pub conflict_id: String,
    pub conflict: SyncConflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredRelayIngestCursor {
    pub relay_url: String,
    pub cursor_since_unix_seconds: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppRelayIngestSuccessInput<'a> {
    pub scope_key: &'a str,
    pub relay_url: &'a str,
    pub cursor_since_unix_seconds: i64,
    pub last_event_created_at_unix_seconds: Option<i64>,
    pub started_at: &'a str,
    pub started_unix_seconds: i64,
    pub completed_at: &'a str,
    pub completed_unix_seconds: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppRelayIngestFailureInput<'a> {
    pub scope_key: &'a str,
    pub relay_url: &'a str,
    pub started_at: &'a str,
    pub started_unix_seconds: i64,
    pub completed_at: &'a str,
    pub completed_unix_seconds: i64,
    pub error_message: &'a str,
}

pub struct AppSyncRepository<'a> {
    connection: &'a AppSqliteDatabase,
}

impl<'a> AppSyncRepository<'a> {
    pub(crate) const fn new(connection: &'a AppSqliteDatabase) -> Self {
        Self { connection }
    }

    pub fn enqueue_pending_operation(
        &self,
        account_id: &str,
        operation: &PendingSyncOperation,
    ) -> Result<String, AppSqliteError> {
        let operation_id = Uuid::now_v7().to_string();

        self.connection
            .execute(
                "INSERT INTO local_outbox (
                    id,
                    account_id,
                    operation_key,
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count,
                    state,
                    last_error_message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(account_id, operation_key)
                WHERE state IN ('pending', 'in_progress', 'failed', 'blocked', 'retryable')
                DO UPDATE SET
                    aggregate_kind = excluded.aggregate_kind,
                    aggregate_id = excluded.aggregate_id,
                    operation_kind = excluded.operation_kind,
                    payload_json = excluded.payload_json,
                    created_at = excluded.created_at,
                    available_at = excluded.available_at,
                    attempt_count = 0,
                    state = 'pending',
                    last_error_message = NULL",
                crate::app_sqlite_params![
                    operation_id,
                    account_id,
                    operation.operation_key.as_str(),
                    operation.aggregate.aggregate_kind(),
                    aggregate_id_value(&operation.aggregate),
                    operation.operation.storage_key(),
                    operation.payload_json.as_str(),
                    operation.created_at.as_str(),
                    operation.available_at.as_str(),
                    i64::from(operation.attempt_count),
                    operation.state.storage_key(),
                    operation.last_error_message.as_deref(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "enqueue pending sync operation",
                source,
            })?;

        self.connection
            .query_row(
                "SELECT id
                 FROM local_outbox
                 WHERE account_id = ?1
                    AND operation_key = ?2
                    AND state IN ('pending', 'in_progress', 'failed', 'blocked', 'retryable')
                 LIMIT 1",
                crate::app_sqlite_params![account_id, operation.operation_key.as_str()],
                |row| row.try_get::<String, _>(0),
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "load pending sync operation id after enqueue",
                source,
            })
    }

    pub fn load_pending_operations(
        &self,
        account_id: &str,
    ) -> Result<Vec<StoredPendingSyncOperation>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    id,
                    operation_key,
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count,
                    state,
                    last_error_message
                 FROM local_outbox
                 WHERE account_id = ?1
                    AND state IN ('pending', 'in_progress', 'failed', 'blocked', 'retryable')
                 ORDER BY available_at ASC, created_at ASC, id ASC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pending sync operations query",
                source,
            })?;
        let rows = statement
            .query_map([account_id], |row| {
                Ok((
                    row.try_get::<String, _>(0)?,
                    row.try_get::<String, _>(1)?,
                    row.try_get::<String, _>(2)?,
                    row.try_get::<String, _>(3)?,
                    row.try_get::<String, _>(4)?,
                    row.try_get::<String, _>(5)?,
                    row.try_get::<String, _>(6)?,
                    row.try_get::<String, _>(7)?,
                    row.try_get::<u32, _>(8)?,
                    row.try_get::<String, _>(9)?,
                    row.try_get::<Option<String>, _>(10)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query pending sync operations",
                source,
            })?;

        rows.map(|row| {
            let (
                operation_id,
                operation_key,
                aggregate_kind,
                aggregate_id,
                operation_kind,
                payload_json,
                created_at,
                available_at,
                attempt_count,
                state,
                last_error_message,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read pending sync operation row",
                source,
            })?;

            Ok(StoredPendingSyncOperation {
                operation_id,
                operation: PendingSyncOperation {
                    operation_key,
                    aggregate: parse_sync_aggregate_ref(
                        "local_outbox.aggregate_kind",
                        "local_outbox.aggregate_id",
                        aggregate_kind,
                        aggregate_id,
                    )?,
                    operation: parse_sync_operation_kind(operation_kind)?,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count,
                    state: parse_pending_sync_operation_state(state)?,
                    last_error_message,
                },
            })
        })
        .collect()
    }

    pub fn update_pending_operation_retry(
        &self,
        account_id: &str,
        operation_id: &str,
        available_at: &str,
        attempt_count: u32,
        last_error_message: Option<&str>,
    ) -> Result<bool, AppSqliteError> {
        let updated = self
            .connection
            .execute(
                "UPDATE local_outbox
                 SET available_at = ?3,
                    attempt_count = ?4,
                    state = 'retryable',
                    last_error_message = ?5
                 WHERE account_id = ?1 AND id = ?2",
                crate::app_sqlite_params![
                    account_id,
                    operation_id,
                    available_at,
                    i64::from(attempt_count),
                    last_error_message
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "update pending sync operation retry",
                source,
            })?;

        Ok(updated == 1)
    }

    pub fn dequeue_pending_operation(
        &self,
        account_id: &str,
        operation_id: &str,
    ) -> Result<bool, AppSqliteError> {
        let deleted = self
            .connection
            .execute(
                "DELETE FROM local_outbox WHERE account_id = ?1 AND id = ?2",
                crate::app_sqlite_params![account_id, operation_id],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "dequeue pending sync operation",
                source,
            })?;

        Ok(deleted == 1)
    }

    pub fn load_checkpoint(
        &self,
        account_id: &str,
    ) -> Result<SyncCheckpointStatus, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "SELECT
                    state,
                    last_sync_started_at,
                    last_sync_completed_at,
                    last_remote_cursor,
                    last_error_message
                 FROM sync_checkpoints
                 WHERE account_id = ?1
                 LIMIT 1",
                [account_id],
                |row| {
                    Ok((
                        row.try_get::<String, _>(0)?,
                        row.try_get::<Option<String>, _>(1)?,
                        row.try_get::<Option<String>, _>(2)?,
                        row.try_get::<Option<String>, _>(3)?,
                        row.try_get::<Option<String>, _>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load sync checkpoint",
                source,
            })?;

        row.map_or_else(
            || Ok(SyncCheckpointStatus::never_synced()),
            |(
                state,
                last_sync_started_at,
                last_sync_completed_at,
                last_remote_cursor,
                last_error_message,
            )| {
                Ok(SyncCheckpointStatus {
                    state: parse_sync_checkpoint_state(state)?,
                    last_sync_started_at,
                    last_sync_completed_at,
                    last_remote_cursor,
                    last_error_message,
                })
            },
        )
    }

    pub fn save_checkpoint(
        &self,
        account_id: &str,
        checkpoint: &SyncCheckpointStatus,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO sync_checkpoints (
                    account_id,
                    state,
                    last_sync_started_at,
                    last_sync_completed_at,
                    last_remote_cursor,
                    last_error_message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(account_id) DO UPDATE SET
                    state = excluded.state,
                    last_sync_started_at = excluded.last_sync_started_at,
                    last_sync_completed_at = excluded.last_sync_completed_at,
                    last_remote_cursor = excluded.last_remote_cursor,
                    last_error_message = excluded.last_error_message",
                crate::app_sqlite_params![
                    account_id,
                    sync_checkpoint_state_value(checkpoint.state),
                    checkpoint.last_sync_started_at,
                    checkpoint.last_sync_completed_at,
                    checkpoint.last_remote_cursor,
                    checkpoint.last_error_message,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "save sync checkpoint",
                source,
            })?;

        Ok(())
    }

    pub fn load_relay_ingest_cursors(
        &self,
        scope_key: &str,
        relay_urls: &[String],
    ) -> Result<Vec<StoredRelayIngestCursor>, AppSqliteError> {
        relay_urls
            .iter()
            .map(|relay_url| {
                let cursor_since_unix_seconds = self
                    .connection
                    .query_row(
                        "SELECT cursor_since_unix_seconds
                         FROM app_relay_ingest_freshness
                         WHERE scope_key = ?1 AND relay_url = ?2
                         LIMIT 1",
                        crate::app_sqlite_params![scope_key, relay_url.as_str()],
                        |row| row.try_get::<Option<i64>, _>(0),
                    )
                    .optional()
                    .map_err(|source| AppSqliteError::Query {
                        operation: "load relay ingest cursor",
                        source,
                    })?
                    .flatten();

                Ok(StoredRelayIngestCursor {
                    relay_url: relay_url.clone(),
                    cursor_since_unix_seconds,
                })
            })
            .collect()
    }

    pub fn load_relay_ingest_freshness(
        &self,
        scope_key: &str,
        relay_urls: &[String],
        now_unix_seconds: i64,
        stale_after_seconds: i64,
    ) -> Result<AppRelayIngestScopeFreshness, AppSqliteError> {
        let relays = relay_urls
            .iter()
            .map(|relay_url| {
                self.load_relay_ingest_relay_freshness(
                    scope_key,
                    relay_url,
                    now_unix_seconds,
                    stale_after_seconds,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let status = relay_ingest_scope_status(relays.as_slice());

        Ok(AppRelayIngestScopeFreshness {
            scope_key: scope_key.to_owned(),
            status,
            relays,
        })
    }

    pub fn record_relay_ingest_success(
        &self,
        input: AppRelayIngestSuccessInput<'_>,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO app_relay_ingest_freshness (
                    scope_key,
                    relay_url,
                    state,
                    cursor_since_unix_seconds,
                    last_event_created_at_unix_seconds,
                    last_fetch_started_at,
                    last_fetch_started_unix_seconds,
                    last_fetch_completed_at,
                    last_fetch_completed_unix_seconds,
                    last_success_at,
                    last_success_unix_seconds,
                    last_error_message,
                    updated_at
                ) VALUES (?1, ?2, 'fresh', ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8, NULL, ?7)
                ON CONFLICT(scope_key, relay_url) DO UPDATE SET
                    state = 'fresh',
                    cursor_since_unix_seconds = excluded.cursor_since_unix_seconds,
                    last_event_created_at_unix_seconds = excluded.last_event_created_at_unix_seconds,
                    last_fetch_started_at = excluded.last_fetch_started_at,
                    last_fetch_started_unix_seconds = excluded.last_fetch_started_unix_seconds,
                    last_fetch_completed_at = excluded.last_fetch_completed_at,
                    last_fetch_completed_unix_seconds = excluded.last_fetch_completed_unix_seconds,
                    last_success_at = excluded.last_success_at,
                    last_success_unix_seconds = excluded.last_success_unix_seconds,
                    last_error_message = NULL,
                    updated_at = excluded.updated_at",
                crate::app_sqlite_params![
                    input.scope_key,
                    input.relay_url,
                    input.cursor_since_unix_seconds,
                    input.last_event_created_at_unix_seconds,
                    input.started_at,
                    input.started_unix_seconds,
                    input.completed_at,
                    input.completed_unix_seconds,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record relay ingest success",
                source,
            })?;

        Ok(())
    }

    pub fn record_relay_ingest_failure(
        &self,
        input: AppRelayIngestFailureInput<'_>,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO app_relay_ingest_freshness (
                    scope_key,
                    relay_url,
                    state,
                    cursor_since_unix_seconds,
                    last_event_created_at_unix_seconds,
                    last_fetch_started_at,
                    last_fetch_started_unix_seconds,
                    last_fetch_completed_at,
                    last_fetch_completed_unix_seconds,
                    last_success_at,
                    last_success_unix_seconds,
                    last_error_message,
                    updated_at
                ) VALUES (?1, ?2, 'failed', NULL, NULL, ?3, ?4, ?5, ?6, NULL, NULL, ?7, ?5)
                ON CONFLICT(scope_key, relay_url) DO UPDATE SET
                    state = 'failed',
                    last_fetch_started_at = excluded.last_fetch_started_at,
                    last_fetch_started_unix_seconds = excluded.last_fetch_started_unix_seconds,
                    last_fetch_completed_at = excluded.last_fetch_completed_at,
                    last_fetch_completed_unix_seconds = excluded.last_fetch_completed_unix_seconds,
                    last_error_message = excluded.last_error_message,
                    updated_at = excluded.updated_at",
                crate::app_sqlite_params![
                    input.scope_key,
                    input.relay_url,
                    input.started_at,
                    input.started_unix_seconds,
                    input.completed_at,
                    input.completed_unix_seconds,
                    input.error_message,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record relay ingest failure",
                source,
            })?;

        Ok(())
    }

    fn load_relay_ingest_relay_freshness(
        &self,
        scope_key: &str,
        relay_url: &str,
        now_unix_seconds: i64,
        stale_after_seconds: i64,
    ) -> Result<AppRelayIngestRelayFreshness, AppSqliteError> {
        let row = self
            .connection
            .query_row(
                "SELECT
                    state,
                    cursor_since_unix_seconds,
                    last_event_created_at_unix_seconds,
                    last_fetch_started_at,
                    last_fetch_completed_at,
                    last_fetch_completed_unix_seconds,
                    last_success_at,
                    last_error_message
                 FROM app_relay_ingest_freshness
                 WHERE scope_key = ?1 AND relay_url = ?2
                 LIMIT 1",
                crate::app_sqlite_params![scope_key, relay_url],
                |row| {
                    Ok((
                        row.try_get::<String, _>(0)?,
                        row.try_get::<Option<i64>, _>(1)?,
                        row.try_get::<Option<i64>, _>(2)?,
                        row.try_get::<Option<String>, _>(3)?,
                        row.try_get::<Option<String>, _>(4)?,
                        row.try_get::<Option<i64>, _>(5)?,
                        row.try_get::<Option<String>, _>(6)?,
                        row.try_get::<Option<String>, _>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load relay ingest freshness",
                source,
            })?;

        row.map_or_else(
            || {
                Ok(AppRelayIngestRelayFreshness {
                    relay_url: relay_url.to_owned(),
                    state: AppRelayIngestFreshnessState::Stale,
                    cursor_since_unix_seconds: None,
                    last_event_created_at_unix_seconds: None,
                    last_fetch_started_at: None,
                    last_fetch_completed_at: None,
                    last_success_at: None,
                    last_error_message: None,
                })
            },
            |(
                state,
                cursor_since_unix_seconds,
                last_event_created_at_unix_seconds,
                last_fetch_started_at,
                last_fetch_completed_at,
                last_fetch_completed_unix_seconds,
                last_success_at,
                last_error_message,
            )| {
                let mut state = parse_relay_ingest_freshness_state(state)?;
                if state == AppRelayIngestFreshnessState::Fresh
                    && relay_ingest_is_stale(
                        last_fetch_completed_unix_seconds,
                        now_unix_seconds,
                        stale_after_seconds,
                    )
                {
                    state = AppRelayIngestFreshnessState::Stale;
                }
                Ok(AppRelayIngestRelayFreshness {
                    relay_url: relay_url.to_owned(),
                    state,
                    cursor_since_unix_seconds,
                    last_event_created_at_unix_seconds,
                    last_fetch_started_at,
                    last_fetch_completed_at,
                    last_success_at,
                    last_error_message,
                })
            },
        )
    }

    pub fn record_conflict(
        &self,
        account_id: &str,
        conflict: &SyncConflict,
    ) -> Result<String, AppSqliteError> {
        let conflict_id = Uuid::now_v7().to_string();

        self.connection
            .execute(
                "INSERT INTO local_conflicts (
                    id,
                    account_id,
                    aggregate_kind,
                    aggregate_id,
                    conflict_kind,
                    severity,
                    resolution_status,
                    local_payload_json,
                    remote_payload_json,
                    detected_at,
                    resolved_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                crate::app_sqlite_params![
                    conflict_id,
                    account_id,
                    conflict.aggregate.aggregate_kind(),
                    aggregate_id_value(&conflict.aggregate),
                    conflict.kind.storage_key(),
                    sync_conflict_severity_value(conflict.severity),
                    sync_conflict_resolution_status_value(conflict.resolution),
                    conflict.local_payload_json,
                    conflict.remote_payload_json,
                    conflict.detected_at,
                    conflict.resolved_at,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record sync conflict",
                source,
            })?;

        Ok(conflict_id)
    }

    pub fn replace_conflicts(
        &self,
        account_id: &str,
        conflicts: &[SyncConflict],
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "DELETE FROM local_conflicts WHERE account_id = ?1",
                [account_id],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "clear sync conflicts",
                source,
            })?;

        for conflict in conflicts {
            let _ = self.record_conflict(account_id, conflict)?;
        }

        Ok(())
    }

    pub fn load_conflicts(
        &self,
        account_id: &str,
    ) -> Result<Vec<StoredSyncConflict>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    id,
                    aggregate_kind,
                    aggregate_id,
                    conflict_kind,
                    severity,
                    resolution_status,
                    local_payload_json,
                    remote_payload_json,
                    detected_at,
                    resolved_at
                 FROM local_conflicts
                 WHERE account_id = ?1
                 ORDER BY detected_at DESC, id DESC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare sync conflicts query",
                source,
            })?;
        let rows = statement
            .query_map([account_id], |row| {
                Ok((
                    row.try_get::<String, _>(0)?,
                    row.try_get::<String, _>(1)?,
                    row.try_get::<String, _>(2)?,
                    row.try_get::<String, _>(3)?,
                    row.try_get::<String, _>(4)?,
                    row.try_get::<String, _>(5)?,
                    row.try_get::<String, _>(6)?,
                    row.try_get::<Option<String>, _>(7)?,
                    row.try_get::<String, _>(8)?,
                    row.try_get::<Option<String>, _>(9)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query sync conflicts",
                source,
            })?;

        rows.map(|row| {
            let (
                conflict_id,
                aggregate_kind,
                aggregate_id,
                conflict_kind,
                severity,
                resolution_status,
                local_payload_json,
                remote_payload_json,
                detected_at,
                resolved_at,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read sync conflict row",
                source,
            })?;

            Ok(StoredSyncConflict {
                conflict_id,
                conflict: SyncConflict {
                    aggregate: parse_sync_aggregate_ref(
                        "local_conflicts.aggregate_kind",
                        "local_conflicts.aggregate_id",
                        aggregate_kind,
                        aggregate_id,
                    )?,
                    kind: parse_sync_conflict_kind(conflict_kind)?,
                    severity: parse_sync_conflict_severity(severity)?,
                    resolution: parse_sync_conflict_resolution_status(resolution_status)?,
                    local_payload_json,
                    remote_payload_json,
                    detected_at,
                    resolved_at,
                },
            })
        })
        .collect()
    }

    pub fn resolve_conflict(
        &self,
        account_id: &str,
        conflict_id: &str,
        resolution: SyncConflictResolutionStatus,
        resolved_at: &str,
    ) -> Result<bool, AppSqliteError> {
        if resolution == SyncConflictResolutionStatus::Unresolved {
            return Err(AppSqliteError::InvalidProjection {
                reason: "sync conflict resolution must be terminal",
            });
        }

        let updated = self
            .connection
            .execute(
                "UPDATE local_conflicts
                 SET resolution_status = ?3, resolved_at = ?4
                 WHERE account_id = ?1 AND id = ?2",
                crate::app_sqlite_params![
                    account_id,
                    conflict_id,
                    sync_conflict_resolution_status_value(resolution),
                    resolved_at,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "resolve sync conflict",
                source,
            })?;

        Ok(updated == 1)
    }
}

fn aggregate_id_value(aggregate: &SyncAggregateRef) -> String {
    match aggregate {
        SyncAggregateRef::Farm(farm_id) => farm_id.to_string(),
        SyncAggregateRef::FulfillmentWindow(fulfillment_window_id) => {
            fulfillment_window_id.to_string()
        }
        SyncAggregateRef::Product(product_id) => product_id.to_string(),
        SyncAggregateRef::Order(order_id) => order_id.to_string(),
    }
}

fn parse_sync_aggregate_ref(
    aggregate_kind_field: &'static str,
    aggregate_id_field: &'static str,
    aggregate_kind: String,
    aggregate_id: String,
) -> Result<SyncAggregateRef, AppSqliteError> {
    match aggregate_kind.as_str() {
        "farm" => Ok(SyncAggregateRef::Farm(
            aggregate_id
                .parse::<FarmId>()
                .map_err(|_| AppSqliteError::DecodeId {
                    field: aggregate_id_field,
                    value: aggregate_id,
                })?,
        )),
        "fulfillment_window" => Ok(SyncAggregateRef::FulfillmentWindow(
            aggregate_id
                .parse::<FulfillmentWindowId>()
                .map_err(|_| AppSqliteError::DecodeId {
                    field: aggregate_id_field,
                    value: aggregate_id,
                })?,
        )),
        "product" => Ok(SyncAggregateRef::Product(
            aggregate_id
                .parse::<ProductId>()
                .map_err(|_| AppSqliteError::DecodeId {
                    field: aggregate_id_field,
                    value: aggregate_id,
                })?,
        )),
        "order" => Ok(SyncAggregateRef::Order(
            aggregate_id
                .parse::<OrderId>()
                .map_err(|_| AppSqliteError::DecodeId {
                    field: aggregate_id_field,
                    value: aggregate_id,
                })?,
        )),
        _ => Err(AppSqliteError::DecodeEnum {
            field: aggregate_kind_field,
            value: aggregate_kind,
        }),
    }
}

fn parse_sync_operation_kind(value: String) -> Result<SyncOperationKind, AppSqliteError> {
    match value.as_str() {
        "upsert" => Ok(SyncOperationKind::Upsert),
        "delete" => Ok(SyncOperationKind::Delete),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "local_outbox.operation_kind",
            value,
        }),
    }
}

fn parse_pending_sync_operation_state(
    value: String,
) -> Result<PendingSyncOperationState, AppSqliteError> {
    match value.as_str() {
        "pending" => Ok(PendingSyncOperationState::Pending),
        "in_progress" => Ok(PendingSyncOperationState::InProgress),
        "succeeded" => Ok(PendingSyncOperationState::Succeeded),
        "failed" => Ok(PendingSyncOperationState::Failed),
        "blocked" => Ok(PendingSyncOperationState::Blocked),
        "retryable" => Ok(PendingSyncOperationState::Retryable),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "local_outbox.state",
            value,
        }),
    }
}

fn parse_sync_conflict_kind(value: String) -> Result<SyncConflictKind, AppSqliteError> {
    match value.as_str() {
        "revision_mismatch" => Ok(SyncConflictKind::RevisionMismatch),
        "remote_delete" => Ok(SyncConflictKind::RemoteDelete),
        "remote_validation_reject" => Ok(SyncConflictKind::RemoteValidationReject),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "local_conflicts.conflict_kind",
            value,
        }),
    }
}

fn parse_sync_conflict_severity(value: String) -> Result<SyncConflictSeverity, AppSqliteError> {
    match value.as_str() {
        "review_required" => Ok(SyncConflictSeverity::ReviewRequired),
        "blocking" => Ok(SyncConflictSeverity::Blocking),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "local_conflicts.severity",
            value,
        }),
    }
}

fn parse_sync_conflict_resolution_status(
    value: String,
) -> Result<SyncConflictResolutionStatus, AppSqliteError> {
    match value.as_str() {
        "unresolved" => Ok(SyncConflictResolutionStatus::Unresolved),
        "accepted_local" => Ok(SyncConflictResolutionStatus::AcceptedLocal),
        "accepted_remote" => Ok(SyncConflictResolutionStatus::AcceptedRemote),
        "dismissed" => Ok(SyncConflictResolutionStatus::Dismissed),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "local_conflicts.resolution_status",
            value,
        }),
    }
}

fn parse_sync_checkpoint_state(value: String) -> Result<SyncCheckpointState, AppSqliteError> {
    match value.as_str() {
        "never_synced" => Ok(SyncCheckpointState::NeverSynced),
        "syncing" => Ok(SyncCheckpointState::Syncing),
        "current" => Ok(SyncCheckpointState::Current),
        "failed" => Ok(SyncCheckpointState::Failed),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "sync_checkpoints.state",
            value,
        }),
    }
}

fn sync_checkpoint_state_value(state: SyncCheckpointState) -> &'static str {
    match state {
        SyncCheckpointState::NeverSynced => "never_synced",
        SyncCheckpointState::Syncing => "syncing",
        SyncCheckpointState::Current => "current",
        SyncCheckpointState::Failed => "failed",
    }
}

fn sync_conflict_severity_value(severity: SyncConflictSeverity) -> &'static str {
    match severity {
        SyncConflictSeverity::ReviewRequired => "review_required",
        SyncConflictSeverity::Blocking => "blocking",
    }
}

fn sync_conflict_resolution_status_value(resolution: SyncConflictResolutionStatus) -> &'static str {
    match resolution {
        SyncConflictResolutionStatus::Unresolved => "unresolved",
        SyncConflictResolutionStatus::AcceptedLocal => "accepted_local",
        SyncConflictResolutionStatus::AcceptedRemote => "accepted_remote",
        SyncConflictResolutionStatus::Dismissed => "dismissed",
    }
}

fn parse_relay_ingest_freshness_state(
    value: String,
) -> Result<AppRelayIngestFreshnessState, AppSqliteError> {
    match value.as_str() {
        "fresh" => Ok(AppRelayIngestFreshnessState::Fresh),
        "stale" => Ok(AppRelayIngestFreshnessState::Stale),
        "failed" => Ok(AppRelayIngestFreshnessState::Failed),
        _ => Err(AppSqliteError::DecodeEnum {
            field: "app_relay_ingest_freshness.state",
            value,
        }),
    }
}

fn relay_ingest_is_stale(
    last_fetch_completed_unix_seconds: Option<i64>,
    now_unix_seconds: i64,
    stale_after_seconds: i64,
) -> bool {
    let Some(last_fetch_completed_unix_seconds) = last_fetch_completed_unix_seconds else {
        return true;
    };
    now_unix_seconds.saturating_sub(last_fetch_completed_unix_seconds) > stale_after_seconds
}

fn relay_ingest_scope_status(relays: &[AppRelayIngestRelayFreshness]) -> AppRelayIngestScopeStatus {
    if relays.is_empty() {
        return AppRelayIngestScopeStatus::Stale;
    }
    let failed_count = relays
        .iter()
        .filter(|relay| relay.state == AppRelayIngestFreshnessState::Failed)
        .count();
    if failed_count == relays.len() {
        return AppRelayIngestScopeStatus::Failed;
    }
    if failed_count > 0 {
        return AppRelayIngestScopeStatus::Partial;
    }
    if relays
        .iter()
        .all(|relay| relay.state == AppRelayIngestFreshnessState::Fresh)
    {
        AppRelayIngestScopeStatus::Fresh
    } else {
        AppRelayIngestScopeStatus::Stale
    }
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_sync::{
        AppRelayIngestFreshnessState, AppRelayIngestScopeStatus, PendingSyncOperation,
        PendingSyncOperationState, SyncAggregateRef, SyncCheckpointStatus, SyncConflict,
        SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity, SyncOperationKind,
    };
    use radroots_studio_app_view::{FarmId, ProductId};

    use crate::{
        AppRelayIngestFailureInput, AppRelayIngestSuccessInput, AppSqliteStore, DatabaseTarget,
    };

    #[test]
    fn checkpoints_are_selected_account_scoped() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let checkpoint =
            SyncCheckpointStatus::syncing("2026-04-20T18:00:00Z", Some("cursor-1".to_owned()));

        assert_eq!(
            repository
                .load_checkpoint("acct_a")
                .expect("missing checkpoint should load"),
            SyncCheckpointStatus::never_synced()
        );

        repository
            .save_checkpoint("acct_a", &checkpoint)
            .expect("checkpoint should save");

        assert_eq!(
            repository
                .load_checkpoint("acct_a")
                .expect("saved checkpoint should load"),
            checkpoint
        );
        assert_eq!(
            repository
                .load_checkpoint("acct_b")
                .expect("other account checkpoint should load"),
            SyncCheckpointStatus::never_synced()
        );
    }

    #[test]
    fn relay_ingest_freshness_tracks_cursors_and_scope_status() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let relay_urls = vec![
            "wss://relay-a.example".to_owned(),
            "wss://relay-b.example".to_owned(),
        ];

        let initial = repository
            .load_relay_ingest_freshness("direct_relay_ingest", &relay_urls, 1_000, 60)
            .expect("freshness should load");
        assert_eq!(initial.status, AppRelayIngestScopeStatus::Stale);
        assert_eq!(initial.relays.len(), 2);
        assert!(
            initial
                .relays
                .iter()
                .all(|relay| relay.state == AppRelayIngestFreshnessState::Stale)
        );

        repository
            .record_relay_ingest_success(AppRelayIngestSuccessInput {
                scope_key: "direct_relay_ingest",
                relay_url: "wss://relay-a.example",
                cursor_since_unix_seconds: 1_010,
                last_event_created_at_unix_seconds: Some(1_009),
                started_at: "2026-05-25T20:00:00Z",
                started_unix_seconds: 1_000,
                completed_at: "2026-05-25T20:00:02Z",
                completed_unix_seconds: 1_002,
            })
            .expect("success should record");
        repository
            .record_relay_ingest_failure(AppRelayIngestFailureInput {
                scope_key: "direct_relay_ingest",
                relay_url: "wss://relay-b.example",
                started_at: "2026-05-25T20:00:00Z",
                started_unix_seconds: 1_000,
                completed_at: "2026-05-25T20:00:02Z",
                completed_unix_seconds: 1_002,
                error_message: "relay timeout",
            })
            .expect("failure should record");

        let cursors = repository
            .load_relay_ingest_cursors("direct_relay_ingest", &relay_urls)
            .expect("cursors should load");
        assert_eq!(cursors[0].cursor_since_unix_seconds, Some(1_010));
        assert_eq!(cursors[1].cursor_since_unix_seconds, None);

        let partial = repository
            .load_relay_ingest_freshness("direct_relay_ingest", &relay_urls, 1_005, 60)
            .expect("partial freshness should load");
        assert_eq!(partial.status, AppRelayIngestScopeStatus::Partial);
        assert_eq!(partial.relays[0].state, AppRelayIngestFreshnessState::Fresh);
        assert_eq!(
            partial.relays[1].state,
            AppRelayIngestFreshnessState::Failed
        );
        assert_eq!(
            partial.relays[1].last_error_message.as_deref(),
            Some("relay timeout")
        );

        let stale = repository
            .load_relay_ingest_freshness(
                "direct_relay_ingest",
                &["wss://relay-a.example".to_owned()],
                1_100,
                60,
            )
            .expect("stale freshness should load");
        assert_eq!(stale.status, AppRelayIngestScopeStatus::Stale);
        assert_eq!(stale.relays[0].state, AppRelayIngestFreshnessState::Stale);
        assert_eq!(stale.relays[0].cursor_since_unix_seconds, Some(1_010));
    }

    #[test]
    fn pending_operations_are_account_scoped_and_retryable() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let first = PendingSyncOperation::new(
            SyncAggregateRef::Farm(FarmId::generate()),
            SyncOperationKind::Upsert,
            "{\"farm\":\"a\"}",
            "2026-04-20T18:00:00Z",
        );
        let second = PendingSyncOperation::new(
            SyncAggregateRef::Product(ProductId::generate()),
            SyncOperationKind::Delete,
            "{\"product\":\"b\"}",
            "2026-04-20T18:05:00Z",
        );

        let first_id = repository
            .enqueue_pending_operation("acct_a", &first)
            .expect("first operation should save");
        let second_id = repository
            .enqueue_pending_operation("acct_a", &second)
            .expect("second operation should save");
        repository
            .enqueue_pending_operation("acct_b", &first)
            .expect("other account operation should save");

        let before_retry = repository
            .load_pending_operations("acct_a")
            .expect("pending operations should load");
        assert_eq!(before_retry.len(), 2);
        assert_eq!(before_retry[0].operation, first);
        assert_eq!(before_retry[1].operation, second);

        assert!(
            repository
                .update_pending_operation_retry(
                    "acct_a",
                    &first_id,
                    "2026-04-20T18:10:00Z",
                    2,
                    Some("relay timeout"),
                )
                .expect("retry update should succeed")
        );
        assert!(
            !repository
                .update_pending_operation_retry(
                    "acct_b",
                    &first_id,
                    "2026-04-20T18:10:00Z",
                    3,
                    Some("wrong account"),
                )
                .expect("wrong-account retry update should not succeed")
        );
        assert!(
            repository
                .dequeue_pending_operation("acct_a", &second_id)
                .expect("dequeue should succeed")
        );

        let acct_a = repository
            .load_pending_operations("acct_a")
            .expect("account operations should reload");
        let acct_b = repository
            .load_pending_operations("acct_b")
            .expect("other account operations should reload");

        assert_eq!(acct_a.len(), 1);
        assert_eq!(acct_a[0].operation_id, first_id);
        assert_eq!(acct_a[0].operation.attempt_count, 2);
        assert_eq!(
            acct_a[0].operation.state,
            PendingSyncOperationState::Retryable
        );
        assert_eq!(
            acct_a[0].operation.available_at,
            "2026-04-20T18:10:00Z".to_owned()
        );
        assert_eq!(
            acct_a[0].operation.last_error_message.as_deref(),
            Some("relay timeout")
        );
        assert_eq!(acct_b.len(), 1);
    }

    #[test]
    fn outbox_enqueue_upserts_active_operation_by_deterministic_key() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let product_id = ProductId::generate();
        let first = PendingSyncOperation::new(
            SyncAggregateRef::Product(product_id),
            SyncOperationKind::Upsert,
            "{\"title\":\"greens\"}",
            "2026-04-20T18:00:00Z",
        );
        let mut replacement = PendingSyncOperation::new(
            SyncAggregateRef::Product(product_id),
            SyncOperationKind::Upsert,
            "{\"title\":\"winter greens\"}",
            "2026-04-20T18:05:00Z",
        );
        replacement.attempt_count = 3;
        replacement.state = PendingSyncOperationState::Failed;
        replacement.last_error_message = Some("stale relay state".to_owned());

        let first_id = repository
            .enqueue_pending_operation("acct_a", &first)
            .expect("first operation should save");
        let replacement_id = repository
            .enqueue_pending_operation("acct_a", &replacement)
            .expect("replacement operation should upsert");

        let pending = repository
            .load_pending_operations("acct_a")
            .expect("pending operations should load");

        assert_eq!(replacement_id, first_id);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].operation_id, first_id);
        assert_eq!(pending[0].operation.operation_key, first.operation_key);
        assert_eq!(
            pending[0].operation.payload_json,
            "{\"title\":\"winter greens\"}"
        );
        assert_eq!(pending[0].operation.attempt_count, 0);
        assert_eq!(
            pending[0].operation.state,
            PendingSyncOperationState::Pending
        );
        assert_eq!(pending[0].operation.last_error_message, None);
    }

    #[test]
    fn conflicts_are_account_scoped_and_resolvable() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let first = SyncConflict {
            aggregate: SyncAggregateRef::Farm(FarmId::generate()),
            kind: SyncConflictKind::RevisionMismatch,
            severity: SyncConflictSeverity::Blocking,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"farm\":\"local\"}".to_owned(),
            remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
            detected_at: "2026-04-20T18:00:00Z".to_owned(),
            resolved_at: None,
        };
        let second = SyncConflict {
            aggregate: SyncAggregateRef::Product(ProductId::generate()),
            kind: SyncConflictKind::RemoteValidationReject,
            severity: SyncConflictSeverity::ReviewRequired,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"product\":\"local\"}".to_owned(),
            remote_payload_json: None,
            detected_at: "2026-04-20T18:05:00Z".to_owned(),
            resolved_at: None,
        };

        let first_id = repository
            .record_conflict("acct_a", &first)
            .expect("first conflict should save");
        repository
            .record_conflict("acct_b", &second)
            .expect("other account conflict should save");

        assert!(
            repository
                .resolve_conflict(
                    "acct_a",
                    &first_id,
                    SyncConflictResolutionStatus::AcceptedLocal,
                    "2026-04-20T18:06:00Z",
                )
                .expect("conflict resolution should succeed")
        );
        assert!(
            !repository
                .resolve_conflict(
                    "acct_b",
                    &first_id,
                    SyncConflictResolutionStatus::AcceptedRemote,
                    "2026-04-20T18:07:00Z",
                )
                .expect("wrong-account resolution should not succeed")
        );

        let acct_a = repository
            .load_conflicts("acct_a")
            .expect("account conflicts should load");
        let acct_b = repository
            .load_conflicts("acct_b")
            .expect("other account conflicts should load");

        assert_eq!(acct_a.len(), 1);
        assert_eq!(acct_a[0].conflict_id, first_id);
        assert_eq!(
            acct_a[0].conflict.resolution,
            SyncConflictResolutionStatus::AcceptedLocal
        );
        assert_eq!(
            acct_a[0].conflict.resolved_at.as_deref(),
            Some("2026-04-20T18:06:00Z")
        );
        assert_eq!(acct_b.len(), 1);
        assert_eq!(acct_b[0].conflict, second);
    }

    #[test]
    fn replacing_conflicts_clears_stale_rows_for_the_selected_account() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let first = SyncConflict {
            aggregate: SyncAggregateRef::Farm(FarmId::generate()),
            kind: SyncConflictKind::RevisionMismatch,
            severity: SyncConflictSeverity::Blocking,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"farm\":\"local\"}".to_owned(),
            remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
            detected_at: "2026-04-20T18:00:00Z".to_owned(),
            resolved_at: None,
        };
        let second = SyncConflict {
            aggregate: SyncAggregateRef::Product(ProductId::generate()),
            kind: SyncConflictKind::RemoteValidationReject,
            severity: SyncConflictSeverity::ReviewRequired,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"product\":\"local\"}".to_owned(),
            remote_payload_json: None,
            detected_at: "2026-04-20T18:05:00Z".to_owned(),
            resolved_at: None,
        };

        repository
            .record_conflict("acct_a", &first)
            .expect("first conflict should save");
        repository
            .record_conflict("acct_b", &first)
            .expect("other account conflict should save");

        repository
            .replace_conflicts("acct_a", std::slice::from_ref(&second))
            .expect("conflicts should replace");

        let acct_a = repository
            .load_conflicts("acct_a")
            .expect("account conflicts should load");
        let acct_b = repository
            .load_conflicts("acct_b")
            .expect("other account conflicts should load");

        assert_eq!(acct_a.len(), 1);
        assert_eq!(acct_a[0].conflict, second);
        assert_eq!(acct_b.len(), 1);
        assert_eq!(acct_b[0].conflict, first);
    }
}
