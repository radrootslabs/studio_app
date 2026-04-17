CREATE TABLE farms (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    readiness TEXT NOT NULL CHECK (readiness IN ('incomplete', 'ready')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE fulfillment_windows (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    capacity_limit INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'published', 'paused')),
    stock_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE orders (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    fulfillment_window_id TEXT REFERENCES fulfillment_windows(id) ON DELETE SET NULL,
    order_number TEXT NOT NULL,
    customer_display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('needs_action', 'scheduled', 'packed', 'completed', 'refunded')
    ),
    updated_at TEXT NOT NULL
);

CREATE TABLE local_outbox (
    id TEXT PRIMARY KEY NOT NULL,
    aggregate_kind TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    available_at TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE local_conflicts (
    id TEXT PRIMARY KEY NOT NULL,
    aggregate_kind TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    conflict_kind TEXT NOT NULL,
    local_payload_json TEXT NOT NULL,
    remote_payload_json TEXT,
    detected_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE sync_checkpoints (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_sync_started_at TEXT,
    last_sync_completed_at TEXT,
    last_remote_cursor TEXT,
    last_error_message TEXT
);

CREATE INDEX idx_products_farm_status ON products(farm_id, status);
CREATE INDEX idx_orders_farm_status ON orders(farm_id, status);
CREATE INDEX idx_fulfillment_windows_farm_starts_at ON fulfillment_windows(farm_id, starts_at);
CREATE INDEX idx_local_outbox_available_at ON local_outbox(available_at);
CREATE INDEX idx_local_outbox_aggregate ON local_outbox(aggregate_kind, aggregate_id);
CREATE INDEX idx_local_conflicts_aggregate ON local_conflicts(aggregate_kind, aggregate_id);

INSERT INTO sync_checkpoints (id) VALUES (1);
