ALTER TABLE products ADD COLUMN listing_bin_id TEXT;

ALTER TABLE buyer_cart_lines ADD COLUMN listing_bin_id TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN quantity_unit_label TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN unit_price_minor_units INTEGER CHECK (
    unit_price_minor_units IS NULL OR unit_price_minor_units >= 0
);
ALTER TABLE buyer_cart_lines ADD COLUMN price_currency TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN farm_key TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN listing_addr TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN listing_event_id TEXT;
ALTER TABLE buyer_cart_lines ADD COLUMN seller_pubkey TEXT;

ALTER TABLE order_lines ADD COLUMN listing_bin_id TEXT;
ALTER TABLE order_lines ADD COLUMN unit_price_minor_units INTEGER CHECK (
    unit_price_minor_units IS NULL OR unit_price_minor_units >= 0
);
ALTER TABLE order_lines ADD COLUMN price_currency TEXT NOT NULL DEFAULT 'USD';
ALTER TABLE order_lines ADD COLUMN farm_key TEXT;
ALTER TABLE order_lines ADD COLUMN listing_addr TEXT;
ALTER TABLE order_lines ADD COLUMN listing_event_id TEXT;
ALTER TABLE order_lines ADD COLUMN seller_pubkey TEXT;
