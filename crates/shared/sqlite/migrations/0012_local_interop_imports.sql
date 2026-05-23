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
        projected_kind IN ('farm', 'listing', 'signed_event', 'unsupported')
    ),
    projected_id TEXT,
    event_id TEXT,
    event_kind INTEGER,
    outbox_status TEXT NOT NULL CHECK (
        outbox_status IN ('none', 'pending', 'acknowledged', 'failed')
    ),
    relay_delivery_json TEXT,
    imported_at TEXT NOT NULL
);

CREATE INDEX idx_local_interop_imports_seq
    ON local_interop_imports(local_seq);

CREATE INDEX idx_local_interop_imports_owner_status
    ON local_interop_imports(owner_account_id, local_status, local_seq DESC);

CREATE INDEX idx_local_interop_imports_projected
    ON local_interop_imports(projected_kind, projected_id);
