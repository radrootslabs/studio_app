CREATE TABLE app_sdk_migration_receipts (
    id TEXT PRIMARY KEY NOT NULL,
    source_kind TEXT NOT NULL CHECK (
        source_kind IN ('local_outbox', 'shared_local_event')
    ),
    source_record_id TEXT NOT NULL,
    sdk_operation_kind TEXT NOT NULL,
    sdk_outbox_event_ids_json TEXT NOT NULL,
    expected_event_id TEXT,
    actor_pubkey TEXT,
    idempotency_digest_prefix TEXT,
    migration_state TEXT NOT NULL CHECK (
        migration_state IN (
            'pending',
            'prepared',
            'enqueued',
            'pushed',
            'failed',
            'blocked',
            'skipped',
            'unsupported',
            'manual_review',
            'unknown'
        )
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    detail_json TEXT NOT NULL,
    UNIQUE(source_kind, source_record_id)
);

CREATE INDEX idx_app_sdk_migration_receipts_source_record ON app_sdk_migration_receipts(
    source_record_id
);
CREATE INDEX idx_app_sdk_migration_receipts_state ON app_sdk_migration_receipts(
    migration_state,
    updated_at
);
