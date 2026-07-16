DROP INDEX IF EXISTS idx_order_validation_receipts_order_time;
DROP INDEX IF EXISTS idx_order_validation_receipts_root_order;
DROP INDEX IF EXISTS idx_order_lines_order_sort;
DROP INDEX IF EXISTS idx_buyer_order_coordination_context_state_updated_at;
DROP INDEX IF EXISTS idx_buyer_order_coordination_state_updated_at;
DROP INDEX IF EXISTS idx_orders_farm_status;
DROP INDEX IF EXISTS idx_orders_farm_window_status_updated_at;
DROP INDEX IF EXISTS idx_orders_buyer_context_updated_at;
DROP INDEX IF EXISTS idx_buyer_cart_lines_context_updated_at;

ALTER TABLE order_validation_receipts RENAME TO order_validation_receipts_public_private_previous;
ALTER TABLE order_lines RENAME TO order_lines_public_private_previous;
ALTER TABLE buyer_order_coordination_records RENAME TO buyer_order_coordination_records_public_private_previous;
ALTER TABLE orders RENAME TO orders_public_private_previous;
ALTER TABLE buyer_cart_lines RENAME TO buyer_cart_lines_public_private_previous;
ALTER TABLE buyer_carts RENAME TO buyer_carts_public_private_previous;

CREATE TABLE buyer_carts (
    buyer_context_key TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT REFERENCES farms(id) ON DELETE SET NULL,
    buyer_name TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);

INSERT INTO buyer_carts (
    buyer_context_key,
    farm_id,
    buyer_name,
    updated_at
)
SELECT
    buyer_context_key,
    farm_id,
    buyer_name,
    updated_at
FROM buyer_carts_public_private_previous;

CREATE TABLE buyer_cart_lines (
    buyer_context_key TEXT NOT NULL REFERENCES buyer_carts(buyer_context_key) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    updated_at TEXT NOT NULL,
    listing_bin_id TEXT,
    quantity_unit_label TEXT,
    unit_price_minor_units INTEGER CHECK (
        unit_price_minor_units IS NULL OR unit_price_minor_units >= 0
    ),
    price_currency TEXT,
    farm_key TEXT,
    listing_addr TEXT,
    listing_event_id TEXT,
    seller_pubkey TEXT,
    listing_relays_json TEXT,
    PRIMARY KEY (buyer_context_key, product_id)
);

INSERT INTO buyer_cart_lines (
    buyer_context_key,
    product_id,
    quantity,
    updated_at,
    listing_bin_id,
    quantity_unit_label,
    unit_price_minor_units,
    price_currency,
    farm_key,
    listing_addr,
    listing_event_id,
    seller_pubkey,
    listing_relays_json
)
SELECT
    buyer_context_key,
    product_id,
    quantity,
    updated_at,
    listing_bin_id,
    quantity_unit_label,
    unit_price_minor_units,
    price_currency,
    farm_key,
    listing_addr,
    listing_event_id,
    seller_pubkey,
    listing_relays_json
FROM buyer_cart_lines_public_private_previous;

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
    workflow_revision TEXT NOT NULL DEFAULT 'none' CHECK (
        workflow_revision IN ('none', 'change_proposed', 'updated', 'kept_as_placed')
    ),
    workflow_agreement TEXT NOT NULL DEFAULT 'requested' CHECK (
        workflow_agreement IN ('requested', 'committed', 'contested', 'declined', 'cancelled', 'invalid')
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
    workflow_revision,
    CASE workflow_agreement
        WHEN 'agreed_pending_validation' THEN 'contested'
        WHEN 'validation_expired' THEN 'invalid'
        ELSE workflow_agreement
    END,
    workflow_inventory,
    workflow_provenance_source,
    workflow_provenance_last_event_id
FROM orders_public_private_previous;

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
FROM order_lines_public_private_previous;

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
FROM buyer_order_coordination_records_public_private_previous;

CREATE TABLE order_validation_receipts (
    event_id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT REFERENCES orders(id) ON DELETE CASCADE,
    raw_order_id TEXT NOT NULL,
    root_event_id TEXT NOT NULL,
    listing_event_id TEXT NOT NULL,
    target_event_id TEXT NOT NULL,
    receipt_type TEXT NOT NULL CHECK (
        receipt_type IN ('listing_validation', 'trade_transition', 'inventory_state', 'state_checkpoint')
    ),
    result TEXT NOT NULL CHECK (
        result IN ('valid', 'needs_review')
    ),
    proof_system TEXT NOT NULL CHECK (
        proof_system IN ('none', 'sp1_core', 'sp1_compressed', 'sp1_groth16', 'sp1_plonk')
    ),
    event_set_root TEXT NOT NULL,
    reducer_output_root TEXT NOT NULL,
    public_values_hash TEXT NOT NULL,
    event_created_at INTEGER NOT NULL CHECK (event_created_at >= 0)
);

INSERT INTO order_validation_receipts (
    event_id,
    order_id,
    raw_order_id,
    root_event_id,
    listing_event_id,
    target_event_id,
    receipt_type,
    result,
    proof_system,
    event_set_root,
    reducer_output_root,
    public_values_hash,
    event_created_at
)
SELECT
    event_id,
    order_id,
    raw_order_id,
    root_event_id,
    listing_event_id,
    target_event_id,
    receipt_type,
    result,
    proof_system,
    event_set_root,
    reducer_output_root,
    public_values_hash,
    event_created_at
FROM order_validation_receipts_public_private_previous;

CREATE INDEX idx_buyer_cart_lines_context_updated_at
    ON buyer_cart_lines(buyer_context_key, updated_at DESC, product_id DESC);
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
CREATE INDEX idx_order_validation_receipts_order_time
    ON order_validation_receipts(order_id, event_created_at DESC, event_id DESC)
    WHERE order_id IS NOT NULL;
CREATE INDEX idx_order_validation_receipts_root_order
    ON order_validation_receipts(root_event_id, raw_order_id);

DROP TABLE order_validation_receipts_public_private_previous;
DROP TABLE order_lines_public_private_previous;
DROP TABLE buyer_order_coordination_records_public_private_previous;
DROP TABLE orders_public_private_previous;
DROP TABLE buyer_cart_lines_public_private_previous;
DROP TABLE buyer_carts_public_private_previous;
