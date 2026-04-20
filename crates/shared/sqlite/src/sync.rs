use radroots_studio_app_models::{FarmId, FulfillmentWindowId, OrderId, ProductId};
use radroots_studio_app_sync::{
    PendingSyncOperation, SyncAggregateRef, SyncCheckpointState, SyncCheckpointStatus,
    SyncConflict, SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity,
    SyncOperationKind,
};
use rusqlite::{Connection, OptionalExtension, params};
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

pub struct AppSyncRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppSyncRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
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
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    operation_id,
                    account_id,
                    operation.aggregate.aggregate_kind(),
                    aggregate_id_value(&operation.aggregate),
                    operation.operation.storage_key(),
                    operation.payload_json,
                    operation.created_at,
                    operation.available_at,
                    i64::from(operation.attempt_count),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "enqueue pending sync operation",
                source,
            })?;

        Ok(operation_id)
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
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count
                 FROM local_outbox
                 WHERE account_id = ?1
                 ORDER BY available_at ASC, created_at ASC, id ASC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare pending sync operations query",
                source,
            })?;
        let rows = statement
            .query_map([account_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, u32>(7)?,
                ))
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query pending sync operations",
                source,
            })?;

        rows.map(|row| {
            let (
                operation_id,
                aggregate_kind,
                aggregate_id,
                operation_kind,
                payload_json,
                created_at,
                available_at,
                attempt_count,
            ) = row.map_err(|source| AppSqliteError::Query {
                operation: "read pending sync operation row",
                source,
            })?;

            Ok(StoredPendingSyncOperation {
                operation_id,
                operation: PendingSyncOperation {
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
    ) -> Result<bool, AppSqliteError> {
        let updated = self
            .connection
            .execute(
                "UPDATE local_outbox
                 SET available_at = ?3, attempt_count = ?4
                 WHERE account_id = ?1 AND id = ?2",
                params![
                    account_id,
                    operation_id,
                    available_at,
                    i64::from(attempt_count)
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
                params![account_id, operation_id],
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
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
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
                params![
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
                params![
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
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
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
                params![
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

#[cfg(test)]
mod tests {
    use radroots_studio_app_models::{FarmId, ProductId};
    use radroots_studio_app_sync::{
        PendingSyncOperation, SyncAggregateRef, SyncCheckpointStatus, SyncConflict,
        SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity, SyncOperationKind,
    };

    use crate::{AppSqliteStore, DatabaseTarget};

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
    fn pending_operations_are_account_scoped_and_retryable() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let first = PendingSyncOperation {
            aggregate: SyncAggregateRef::Farm(FarmId::new()),
            operation: SyncOperationKind::Upsert,
            payload_json: "{\"farm\":\"a\"}".to_owned(),
            created_at: "2026-04-20T18:00:00Z".to_owned(),
            available_at: "2026-04-20T18:00:00Z".to_owned(),
            attempt_count: 0,
        };
        let second = PendingSyncOperation {
            aggregate: SyncAggregateRef::Product(ProductId::new()),
            operation: SyncOperationKind::Delete,
            payload_json: "{\"product\":\"b\"}".to_owned(),
            created_at: "2026-04-20T18:05:00Z".to_owned(),
            available_at: "2026-04-20T18:05:00Z".to_owned(),
            attempt_count: 0,
        };

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
                .update_pending_operation_retry("acct_a", &first_id, "2026-04-20T18:10:00Z", 2,)
                .expect("retry update should succeed")
        );
        assert!(
            !repository
                .update_pending_operation_retry("acct_b", &first_id, "2026-04-20T18:10:00Z", 3,)
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
            acct_a[0].operation.available_at,
            "2026-04-20T18:10:00Z".to_owned()
        );
        assert_eq!(acct_b.len(), 1);
    }

    #[test]
    fn conflicts_are_account_scoped_and_resolvable() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let repository = store.sync_repository();
        let first = SyncConflict {
            aggregate: SyncAggregateRef::Farm(FarmId::new()),
            kind: SyncConflictKind::RevisionMismatch,
            severity: SyncConflictSeverity::Blocking,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"farm\":\"local\"}".to_owned(),
            remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
            detected_at: "2026-04-20T18:00:00Z".to_owned(),
            resolved_at: None,
        };
        let second = SyncConflict {
            aggregate: SyncAggregateRef::Product(ProductId::new()),
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
}
