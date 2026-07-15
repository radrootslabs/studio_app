DROP INDEX IF EXISTS idx_order_lines_order_sort;
DROP INDEX IF EXISTS idx_buyer_order_coordination_context_state_updated_at;
DROP INDEX IF EXISTS idx_buyer_order_coordination_state_updated_at;
DROP INDEX IF EXISTS idx_orders_farm_status;
DROP INDEX IF EXISTS idx_orders_farm_window_status_updated_at;
DROP INDEX IF EXISTS idx_orders_buyer_context_updated_at;

ALTER TABLE order_lines RENAME TO order_lines_agreement_previous;
ALTER TABLE buyer_order_coordination_records RENAME TO buyer_order_coordination_records_agreement_previous;
ALTER TABLE orders RENAME TO orders_agreement_previous;

CREATE TABLE orders (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    fulfillment_window_id TEXT REFERENCES fulfillment_windows(id) ON DELETE SET NULL,
    order_number TEXT NOT NULL,
    customer_display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('needs_action', 'scheduled', 'packed', 'completed', 'declined', 'needs_review')
    ),
    updated_at TEXT NOT NULL,
    buyer_context_key TEXT,
    buyer_email TEXT NOT NULL DEFAULT '',
    buyer_phone TEXT NOT NULL DEFAULT '',
    buyer_order_note TEXT NOT NULL DEFAULT '',
    workflow_revision TEXT NOT NULL DEFAULT 'none' CHECK (
        workflow_revision IN ('none', 'change_proposed', 'updated', 'kept_as_placed')
    ),
    workflow_agreement TEXT NOT NULL DEFAULT 'requested' CHECK (
        workflow_agreement IN ('requested', 'agreed_pending_validation', 'committed', 'declined', 'cancelled', 'validation_expired', 'invalid')
    ),
    workflow_inventory TEXT NOT NULL DEFAULT 'needs_review' CHECK (
        workflow_inventory IN ('available', 'reserved', 'sold_out', 'needs_review')
    ),
    workflow_provenance_source TEXT NOT NULL DEFAULT 'unknown' CHECK (
        workflow_provenance_source IN ('app', 'cli', 'relay', 'runtime_store', 'unknown')
    ),
    workflow_provenance_last_event_id TEXT
);

INSERT INTO orders (
    id,
    farm_id,
    fulfillment_window_id,
    order_number,
    customer_display_name,
    status,
    updated_at,
    buyer_context_key,
    buyer_email,
    buyer_phone,
    buyer_order_note,
    workflow_revision,
    workflow_agreement,
    workflow_inventory,
    workflow_provenance_source,
    workflow_provenance_last_event_id
)
SELECT
    id,
    farm_id,
    fulfillment_window_id,
    order_number,
    customer_display_name,
    status,
    updated_at,
    buyer_context_key,
    buyer_email,
    buyer_phone,
    buyer_order_note,
    workflow_revision,
    workflow_agreement,
    workflow_inventory,
    workflow_provenance_source,
    workflow_provenance_last_event_id
FROM orders_agreement_previous;

CREATE TABLE order_lines (
    id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    quantity_value INTEGER NOT NULL CHECK (quantity_value >= 0),
    quantity_unit_label TEXT NOT NULL DEFAULT '',
    quantity_display TEXT NOT NULL,
    sort_index INTEGER NOT NULL DEFAULT 0,
    listing_bin_id TEXT,
    unit_price_minor_units INTEGER CHECK (
        unit_price_minor_units IS NULL OR unit_price_minor_units >= 0
    ),
    price_currency TEXT NOT NULL DEFAULT 'USD',
    farm_key TEXT,
    listing_addr TEXT,
    listing_event_id TEXT,
    seller_pubkey TEXT,
    listing_relays_json TEXT
);

INSERT INTO order_lines (
    id,
    order_id,
    title,
    quantity_value,
    quantity_unit_label,
    quantity_display,
    sort_index,
    listing_bin_id,
    unit_price_minor_units,
    price_currency,
    farm_key,
    listing_addr,
    listing_event_id,
    seller_pubkey,
    listing_relays_json
)
SELECT
    id,
    order_id,
    title,
    quantity_value,
    quantity_unit_label,
    quantity_display,
    sort_index,
    listing_bin_id,
    unit_price_minor_units,
    price_currency,
    farm_key,
    listing_addr,
    listing_event_id,
    seller_pubkey,
    listing_relays_json
FROM order_lines_agreement_previous;

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

INSERT INTO buyer_order_coordination_records (
    order_id,
    buyer_context_key,
    record_id,
    state,
    payload_json,
    attempt_count,
    last_error_message,
    created_at,
    updated_at,
    synced_at
)
SELECT
    order_id,
    buyer_context_key,
    record_id,
    state,
    payload_json,
    attempt_count,
    last_error_message,
    created_at,
    updated_at,
    synced_at
FROM buyer_order_coordination_records_agreement_previous;

CREATE INDEX idx_orders_farm_status ON orders(farm_id, status);
CREATE INDEX idx_orders_farm_window_status_updated_at
    ON orders(farm_id, fulfillment_window_id, status, updated_at DESC, id DESC);
CREATE INDEX idx_orders_buyer_context_updated_at
    ON orders(buyer_context_key, updated_at DESC, id DESC)
    WHERE buyer_context_key IS NOT NULL AND trim(buyer_context_key) <> '';
CREATE INDEX idx_order_lines_order_sort
    ON order_lines(order_id, sort_index, id);
CREATE INDEX idx_buyer_order_coordination_context_state_updated_at
    ON buyer_order_coordination_records(buyer_context_key, state, updated_at);
CREATE INDEX idx_buyer_order_coordination_state_updated_at
    ON buyer_order_coordination_records(state, updated_at);

DROP TABLE order_lines_agreement_previous;
DROP TABLE buyer_order_coordination_records_agreement_previous;
DROP TABLE orders_agreement_previous;
