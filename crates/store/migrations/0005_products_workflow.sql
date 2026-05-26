CREATE TABLE products_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    subtitle TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('draft', 'published', 'paused', 'archived')),
    unit_label TEXT NOT NULL DEFAULT '',
    price_minor_units INTEGER CHECK (price_minor_units IS NULL OR price_minor_units >= 0),
    price_currency TEXT NOT NULL DEFAULT 'USD',
    stock_count INTEGER CHECK (stock_count IS NULL OR stock_count >= 0),
    availability_window_id TEXT REFERENCES fulfillment_windows(id) ON DELETE SET NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO products_v2 (
    id,
    farm_id,
    title,
    subtitle,
    status,
    unit_label,
    price_minor_units,
    price_currency,
    stock_count,
    availability_window_id,
    updated_at
)
SELECT
    id,
    farm_id,
    title,
    '',
    status,
    '',
    NULL,
    'USD',
    stock_count,
    NULL,
    updated_at
FROM products;

DROP INDEX idx_products_farm_status;
DROP TABLE products;
ALTER TABLE products_v2 RENAME TO products;

CREATE INDEX idx_products_farm_status ON products(farm_id, status);
CREATE INDEX idx_products_farm_updated_at ON products(farm_id, updated_at DESC, id DESC);
