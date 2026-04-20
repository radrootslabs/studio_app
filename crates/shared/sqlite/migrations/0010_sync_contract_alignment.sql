ALTER TABLE local_outbox RENAME TO local_outbox_legacy;
ALTER TABLE local_conflicts RENAME TO local_conflicts_legacy;
DROP TABLE sync_checkpoints;

CREATE TABLE local_outbox (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    aggregate_kind TEXT NOT NULL CHECK (
        aggregate_kind IN ('farm', 'fulfillment_window', 'product', 'order')
    ),
    aggregate_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL CHECK (operation_kind IN ('upsert', 'delete')),
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    available_at TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE local_conflicts (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    aggregate_kind TEXT NOT NULL CHECK (
        aggregate_kind IN ('farm', 'fulfillment_window', 'product', 'order')
    ),
    aggregate_id TEXT NOT NULL,
    conflict_kind TEXT NOT NULL CHECK (
        conflict_kind IN (
            'revision_mismatch',
            'remote_delete',
            'remote_validation_reject'
        )
    ),
    severity TEXT NOT NULL CHECK (severity IN ('review_required', 'blocking')),
    resolution_status TEXT NOT NULL CHECK (
        resolution_status IN (
            'unresolved',
            'accepted_local',
            'accepted_remote',
            'dismissed'
        )
    ),
    local_payload_json TEXT NOT NULL,
    remote_payload_json TEXT,
    detected_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE sync_checkpoints (
    account_id TEXT PRIMARY KEY NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN ('never_synced', 'syncing', 'current', 'failed')
    ),
    last_sync_started_at TEXT,
    last_sync_completed_at TEXT,
    last_remote_cursor TEXT,
    last_error_message TEXT
);

DROP TABLE local_outbox_legacy;
DROP TABLE local_conflicts_legacy;

CREATE INDEX idx_local_outbox_account_available_at ON local_outbox(
    account_id,
    available_at,
    created_at,
    id
);
CREATE INDEX idx_local_outbox_account_aggregate ON local_outbox(
    account_id,
    aggregate_kind,
    aggregate_id
);
CREATE INDEX idx_local_conflicts_account_detected_at ON local_conflicts(
    account_id,
    detected_at,
    id
);
CREATE INDEX idx_local_conflicts_account_aggregate ON local_conflicts(
    account_id,
    aggregate_kind,
    aggregate_id
);
