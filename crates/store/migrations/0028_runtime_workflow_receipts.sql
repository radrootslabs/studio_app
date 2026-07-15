CREATE TABLE desktop_runtime_workflow_receipts (
    id TEXT PRIMARY KEY NOT NULL,
    source_kind TEXT NOT NULL CHECK (
        source_kind IN ('app_workflow', 'shared_runtime_store')
    ),
    source_record_id TEXT NOT NULL,
    sdk_operation_kind TEXT NOT NULL,
    runtime_effect_ids_json TEXT NOT NULL,
    expected_event_id TEXT,
    actor_pubkey TEXT,
    idempotency_digest_prefix TEXT,
    workflow_state TEXT NOT NULL CHECK (
        workflow_state IN (
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

CREATE INDEX idx_desktop_runtime_workflow_receipts_source_record ON desktop_runtime_workflow_receipts(
    source_record_id
);
CREATE INDEX idx_desktop_runtime_workflow_receipts_state ON desktop_runtime_workflow_receipts(
    workflow_state,
    updated_at
);
