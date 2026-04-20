CREATE TABLE buyer_carts (
    buyer_context_key TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT REFERENCES farms(id) ON DELETE SET NULL,
    buyer_name TEXT NOT NULL DEFAULT '',
    buyer_email TEXT NOT NULL DEFAULT '',
    buyer_phone TEXT NOT NULL DEFAULT '',
    buyer_order_note TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);

CREATE TABLE buyer_cart_lines (
    buyer_context_key TEXT NOT NULL REFERENCES buyer_carts(buyer_context_key) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (buyer_context_key, product_id)
);

CREATE INDEX idx_buyer_cart_lines_context_updated_at
    ON buyer_cart_lines(buyer_context_key, updated_at DESC, product_id DESC);

ALTER TABLE orders ADD COLUMN buyer_context_key TEXT;
ALTER TABLE orders ADD COLUMN buyer_email TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN buyer_phone TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN buyer_order_note TEXT NOT NULL DEFAULT '';

CREATE INDEX idx_orders_buyer_context_updated_at
    ON orders(buyer_context_key, updated_at DESC, id DESC)
    WHERE buyer_context_key IS NOT NULL AND trim(buyer_context_key) <> '';
