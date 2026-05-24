ALTER TABLE local_outbox RENAME TO local_outbox_legacy;

CREATE TABLE local_outbox (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    operation_key TEXT NOT NULL,
    aggregate_kind TEXT NOT NULL CHECK (
        aggregate_kind IN ('farm', 'fulfillment_window', 'product', 'order')
    ),
    aggregate_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL CHECK (operation_kind IN ('upsert', 'delete')),
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    available_at TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (
        state IN (
            'pending',
            'in_progress',
            'succeeded',
            'failed',
            'blocked',
            'retryable'
        )
    ),
    last_error_message TEXT
);

CREATE UNIQUE INDEX idx_local_outbox_account_operation_key_active ON local_outbox(
    account_id,
    operation_key
)
WHERE state IN ('pending', 'in_progress', 'failed', 'blocked', 'retryable');

INSERT OR REPLACE INTO local_outbox (
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
)
SELECT
    id,
    account_id,
    aggregate_kind || ':' || aggregate_id || ':' || operation_kind,
    aggregate_kind,
    aggregate_id,
    operation_kind,
    payload_json,
    created_at,
    available_at,
    attempt_count,
    'pending',
    NULL
FROM local_outbox_legacy
ORDER BY available_at ASC, created_at ASC, id ASC;

DROP TABLE local_outbox_legacy;

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
