DROP INDEX IF EXISTS idx_local_interop_imports_seq;
DROP INDEX IF EXISTS idx_local_interop_imports_owner_status;
DROP INDEX IF EXISTS idx_local_interop_imports_projected;

ALTER TABLE local_interop_imports RENAME TO local_interop_imports_validation_receipt_projection_kind_legacy;

CREATE TABLE local_interop_imports (
    record_id TEXT PRIMARY KEY NOT NULL,
    local_seq INTEGER NOT NULL CHECK (local_seq >= 0),
    record_family TEXT NOT NULL CHECK (record_family IN ('local_work', 'signed_event')),
    local_status TEXT NOT NULL CHECK (
        local_status IN (
            'local_draft',
            'local_saved',
            'pending_publish',
            'published',
            'failed',
            'conflict'
        )
    ),
    source_runtime TEXT NOT NULL,
    owner_account_id TEXT,
    owner_pubkey TEXT,
    farm_key TEXT,
    listing_addr TEXT,
    projected_kind TEXT NOT NULL CHECK (
        projected_kind IN ('farm', 'listing', 'signed_event', 'validation_receipt', 'unsupported')
    ),
    projected_id TEXT,
    event_id TEXT,
    event_kind INTEGER,
    outbox_status TEXT NOT NULL CHECK (
        outbox_status IN ('none', 'pending', 'acknowledged', 'failed')
    ),
    relay_delivery_json TEXT,
    imported_at TEXT NOT NULL,
    event_pubkey TEXT,
    event_created_at INTEGER,
    event_tags_json TEXT,
    event_content TEXT,
    event_sig TEXT,
    raw_event_json TEXT
);

INSERT INTO local_interop_imports (
    record_id,
    local_seq,
    record_family,
    local_status,
    source_runtime,
    owner_account_id,
    owner_pubkey,
    farm_key,
    listing_addr,
    projected_kind,
    projected_id,
    event_id,
    event_kind,
    outbox_status,
    relay_delivery_json,
    imported_at,
    event_pubkey,
    event_created_at,
    event_tags_json,
    event_content,
    event_sig,
    raw_event_json
)
SELECT
    record_id,
    local_seq,
    record_family,
    local_status,
    source_runtime,
    owner_account_id,
    owner_pubkey,
    farm_key,
    listing_addr,
    projected_kind,
    projected_id,
    event_id,
    event_kind,
    outbox_status,
    relay_delivery_json,
    imported_at,
    event_pubkey,
    event_created_at,
    event_tags_json,
    event_content,
    event_sig,
    raw_event_json
FROM local_interop_imports_validation_receipt_projection_kind_legacy;

CREATE INDEX idx_local_interop_imports_seq
    ON local_interop_imports(local_seq);

CREATE INDEX idx_local_interop_imports_owner_status
    ON local_interop_imports(owner_account_id, local_status, local_seq DESC);

CREATE INDEX idx_local_interop_imports_projected
    ON local_interop_imports(projected_kind, projected_id);

DROP TABLE local_interop_imports_validation_receipt_projection_kind_legacy;
