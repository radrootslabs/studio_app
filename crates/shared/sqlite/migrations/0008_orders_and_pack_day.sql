CREATE TABLE order_lines (
    id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    quantity_value INTEGER NOT NULL CHECK (quantity_value >= 0),
    quantity_unit_label TEXT NOT NULL DEFAULT '',
    quantity_display TEXT NOT NULL,
    sort_index INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_order_lines_order_sort
    ON order_lines(order_id, sort_index, id);

CREATE INDEX idx_orders_farm_window_status_updated_at
    ON orders(farm_id, fulfillment_window_id, status, updated_at DESC, id DESC);
