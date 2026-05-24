CREATE TABLE buyer_order_coordination_records (
    order_id TEXT PRIMARY KEY NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    buyer_context_key TEXT NOT NULL,
    record_id TEXT,
    state TEXT NOT NULL CHECK (state IN ('pending', 'synced', 'failed')),
    payload_json TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    synced_at TEXT
);

CREATE INDEX idx_buyer_order_coordination_context_state_updated_at
    ON buyer_order_coordination_records(buyer_context_key, state, updated_at);

CREATE INDEX idx_buyer_order_coordination_state_updated_at
    ON buyer_order_coordination_records(state, updated_at);
