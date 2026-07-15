use sqlx::Row;

use crate::{AppSqliteDatabase, OptionalSqliteResult};
use serde_json::Value;
use uuid::Uuid;

use crate::AppSqliteError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeWorkflowReceiptSourceKind {
    AppWorkflow,
    SharedRuntimeStore,
}

impl DesktopRuntimeWorkflowReceiptSourceKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::AppWorkflow => "app_workflow",
            Self::SharedRuntimeStore => "shared_runtime_store",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppSqliteError> {
        match value {
            "app_workflow" => Ok(Self::AppWorkflow),
            "shared_runtime_store" => Ok(Self::SharedRuntimeStore),
            _ => Err(AppSqliteError::DecodeEnum {
                field: "desktop_runtime_workflow_receipts.source_kind",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeWorkflowReceiptState {
    Pending,
    Prepared,
    Enqueued,
    Pushed,
    Failed,
    Blocked,
    Skipped,
    Unsupported,
    ManualReview,
    Unknown,
}

impl DesktopRuntimeWorkflowReceiptState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Prepared => "prepared",
            Self::Enqueued => "enqueued",
            Self::Pushed => "pushed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
            Self::Unsupported => "unsupported",
            Self::ManualReview => "manual_review",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppSqliteError> {
        match value {
            "pending" => Ok(Self::Pending),
            "prepared" => Ok(Self::Prepared),
            "enqueued" => Ok(Self::Enqueued),
            "pushed" => Ok(Self::Pushed),
            "failed" => Ok(Self::Failed),
            "blocked" => Ok(Self::Blocked),
            "skipped" => Ok(Self::Skipped),
            "unsupported" => Ok(Self::Unsupported),
            "manual_review" => Ok(Self::ManualReview),
            "unknown" => Ok(Self::Unknown),
            _ => Err(AppSqliteError::DecodeEnum {
                field: "desktop_runtime_workflow_receipts.workflow_state",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeWorkflowReceiptInput {
    pub source_kind: DesktopRuntimeWorkflowReceiptSourceKind,
    pub source_record_id: String,
    pub sdk_operation_kind: String,
    pub runtime_effect_ids: Vec<String>,
    pub expected_event_id: Option<String>,
    pub actor_pubkey: Option<String>,
    pub idempotency_digest_prefix: Option<String>,
    pub workflow_state: DesktopRuntimeWorkflowReceiptState,
    pub recorded_at: String,
    pub detail_json: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeStoredWorkflowReceipt {
    pub id: String,
    pub source_kind: DesktopRuntimeWorkflowReceiptSourceKind,
    pub source_record_id: String,
    pub sdk_operation_kind: String,
    pub runtime_effect_ids: Vec<String>,
    pub expected_event_id: Option<String>,
    pub actor_pubkey: Option<String>,
    pub idempotency_digest_prefix: Option<String>,
    pub workflow_state: DesktopRuntimeWorkflowReceiptState,
    pub created_at: String,
    pub updated_at: String,
    pub detail_json: Value,
}

pub struct DesktopRuntimeWorkflowReceiptRepository<'a> {
    connection: &'a AppSqliteDatabase,
}

impl<'a> DesktopRuntimeWorkflowReceiptRepository<'a> {
    pub(crate) const fn new(connection: &'a AppSqliteDatabase) -> Self {
        Self { connection }
    }

    pub fn record_receipt(
        &self,
        input: &DesktopRuntimeWorkflowReceiptInput,
    ) -> Result<DesktopRuntimeStoredWorkflowReceipt, AppSqliteError> {
        let receipt_id = Uuid::now_v7().to_string();
        let effect_ids_json =
            serde_json::to_string(&input.runtime_effect_ids).map_err(|source| {
                AppSqliteError::EncodeJson {
                    field: "desktop_runtime_workflow_receipts.runtime_effect_ids_json",
                    source,
                }
            })?;
        let detail_json = serde_json::to_string(&input.detail_json).map_err(|source| {
            AppSqliteError::EncodeJson {
                field: "desktop_runtime_workflow_receipts.detail_json",
                source,
            }
        })?;

        self.connection
            .execute(
                "INSERT INTO desktop_runtime_workflow_receipts (
                    id,
                    source_kind,
                    source_record_id,
                    sdk_operation_kind,
                    runtime_effect_ids_json,
                    expected_event_id,
                    actor_pubkey,
                    idempotency_digest_prefix,
                    workflow_state,
                    created_at,
                    updated_at,
                    detail_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11)
                 ON CONFLICT(source_kind, source_record_id)
                 DO UPDATE SET
                    sdk_operation_kind = excluded.sdk_operation_kind,
                    runtime_effect_ids_json = excluded.runtime_effect_ids_json,
                    expected_event_id = excluded.expected_event_id,
                    actor_pubkey = excluded.actor_pubkey,
                    idempotency_digest_prefix = excluded.idempotency_digest_prefix,
                    workflow_state = excluded.workflow_state,
                    updated_at = excluded.updated_at,
                    detail_json = excluded.detail_json",
                crate::app_sqlite_params![
                    receipt_id,
                    input.source_kind.storage_key(),
                    input.source_record_id.as_str(),
                    input.sdk_operation_kind.as_str(),
                    effect_ids_json.as_str(),
                    input.expected_event_id.as_deref(),
                    input.actor_pubkey.as_deref(),
                    input.idempotency_digest_prefix.as_deref(),
                    input.workflow_state.storage_key(),
                    input.recorded_at.as_str(),
                    detail_json.as_str(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record desktop runtime workflow receipt",
                source,
            })?;

        self.load_receipt(input.source_kind, input.source_record_id.as_str())?
            .ok_or(AppSqliteError::MissingColumn {
                field: "desktop_runtime_workflow_receipts.id",
            })
    }

    pub fn load_receipt(
        &self,
        source_kind: DesktopRuntimeWorkflowReceiptSourceKind,
        source_record_id: &str,
    ) -> Result<Option<DesktopRuntimeStoredWorkflowReceipt>, AppSqliteError> {
        self.connection
            .query_row(
                "SELECT
                    id,
                    source_kind,
                    source_record_id,
                    sdk_operation_kind,
                    runtime_effect_ids_json,
                    expected_event_id,
                    actor_pubkey,
                    idempotency_digest_prefix,
                    workflow_state,
                    created_at,
                    updated_at,
                    detail_json
                 FROM desktop_runtime_workflow_receipts
                 WHERE source_kind = ?1
                    AND source_record_id = ?2
                 LIMIT 1",
                crate::app_sqlite_params![source_kind.storage_key(), source_record_id],
                decode_receipt_row,
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load desktop runtime workflow receipt",
                source,
            })
    }
}

fn decode_receipt_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<DesktopRuntimeStoredWorkflowReceipt, sqlx::Error> {
    let source_kind: String = row.try_get(1)?;
    let effect_ids_json: String = row.try_get(4)?;
    let workflow_state: String = row.try_get(8)?;
    let detail_json: String = row.try_get(11)?;
    Ok(DesktopRuntimeStoredWorkflowReceipt {
        id: row.try_get(0)?,
        source_kind: DesktopRuntimeWorkflowReceiptSourceKind::parse(source_kind.as_str())
            .map_err(decode_app_error)?,
        source_record_id: row.try_get(2)?,
        sdk_operation_kind: row.try_get(3)?,
        runtime_effect_ids: serde_json::from_str(effect_ids_json.as_str()).map_err(|source| {
            decode_app_error(AppSqliteError::DecodeJson {
                field: "desktop_runtime_workflow_receipts.runtime_effect_ids_json",
                source,
            })
        })?,
        expected_event_id: row.try_get(5)?,
        actor_pubkey: row.try_get(6)?,
        idempotency_digest_prefix: row.try_get(7)?,
        workflow_state: DesktopRuntimeWorkflowReceiptState::parse(workflow_state.as_str())
            .map_err(decode_app_error)?,
        created_at: row.try_get(9)?,
        updated_at: row.try_get(10)?,
        detail_json: serde_json::from_str(detail_json.as_str()).map_err(|source| {
            decode_app_error(AppSqliteError::DecodeJson {
                field: "desktop_runtime_workflow_receipts.detail_json",
                source,
            })
        })?,
    })
}

fn decode_app_error(error: AppSqliteError) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        AppSqliteStore, DatabaseTarget, DesktopRuntimeWorkflowReceiptInput,
        DesktopRuntimeWorkflowReceiptSourceKind, DesktopRuntimeWorkflowReceiptState,
    };

    #[test]
    fn workflow_receipts_are_idempotent_by_source_record() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let first = store
            .runtime_workflow_receipt_repository()
            .record_receipt(&DesktopRuntimeWorkflowReceiptInput {
                source_kind: DesktopRuntimeWorkflowReceiptSourceKind::AppWorkflow,
                source_record_id: "source-record-a".to_owned(),
                sdk_operation_kind: "farm.publish".to_owned(),
                runtime_effect_ids: vec!["effect-a".to_owned()],
                expected_event_id: Some("expected-a".to_owned()),
                actor_pubkey: Some("actor-a".to_owned()),
                idempotency_digest_prefix: Some("digest-a".to_owned()),
                workflow_state: DesktopRuntimeWorkflowReceiptState::Enqueued,
                recorded_at: "2026-06-18T12:00:00Z".to_owned(),
                detail_json: json!({"attempt": 1}),
            })
            .expect("record first receipt");
        let second = store
            .runtime_workflow_receipt_repository()
            .record_receipt(&DesktopRuntimeWorkflowReceiptInput {
                source_kind: DesktopRuntimeWorkflowReceiptSourceKind::AppWorkflow,
                source_record_id: "source-record-a".to_owned(),
                sdk_operation_kind: "farm.publish".to_owned(),
                runtime_effect_ids: vec!["effect-b".to_owned()],
                expected_event_id: Some("expected-b".to_owned()),
                actor_pubkey: Some("actor-b".to_owned()),
                idempotency_digest_prefix: Some("digest-b".to_owned()),
                workflow_state: DesktopRuntimeWorkflowReceiptState::Pushed,
                recorded_at: "2026-06-18T12:05:00Z".to_owned(),
                detail_json: json!({"attempt": 2}),
            })
            .expect("record second receipt");

        assert_eq!(first.id, second.id);
        assert_eq!(second.created_at, "2026-06-18T12:00:00Z");
        assert_eq!(second.updated_at, "2026-06-18T12:05:00Z");
        assert_eq!(second.runtime_effect_ids, vec!["effect-b".to_owned()]);
        assert_eq!(second.expected_event_id.as_deref(), Some("expected-b"));
        assert_eq!(second.actor_pubkey.as_deref(), Some("actor-b"));
        assert_eq!(
            second.workflow_state,
            DesktopRuntimeWorkflowReceiptState::Pushed
        );
        assert_eq!(second.detail_json, json!({"attempt": 2}));
    }
}
