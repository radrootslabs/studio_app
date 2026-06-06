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

CREATE INDEX idx_order_validation_receipts_order_time
    ON order_validation_receipts(order_id, event_created_at DESC, event_id DESC)
    WHERE order_id IS NOT NULL;

CREATE INDEX idx_order_validation_receipts_root_order
    ON order_validation_receipts(root_event_id, raw_order_id);
