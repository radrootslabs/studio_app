ALTER TABLE buyer_cart_lines ADD COLUMN listing_relays_json TEXT;

ALTER TABLE order_lines ADD COLUMN listing_relays_json TEXT;
