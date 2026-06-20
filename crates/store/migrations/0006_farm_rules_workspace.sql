ALTER TABLE farms ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC';
ALTER TABLE farms ADD COLUMN currency_code TEXT NOT NULL DEFAULT 'USD';

CREATE TABLE farm_operating_rules (
    farm_id TEXT PRIMARY KEY NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    promise_lead_hours INTEGER NOT NULL CHECK (promise_lead_hours >= 0),
    substitution_policy TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE pickup_locations (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    address_line TEXT NOT NULL,
    directions TEXT,
    is_default INTEGER NOT NULL CHECK (is_default IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_pickup_locations_default_per_farm
    ON pickup_locations(farm_id)
    WHERE is_default = 1;
CREATE INDEX idx_pickup_locations_farm_updated_at
    ON pickup_locations(farm_id, updated_at DESC, id DESC);

ALTER TABLE fulfillment_windows
    ADD COLUMN pickup_location_id TEXT REFERENCES pickup_locations(id) ON DELETE SET NULL;
ALTER TABLE fulfillment_windows ADD COLUMN label TEXT NOT NULL DEFAULT '';
ALTER TABLE fulfillment_windows ADD COLUMN order_cutoff_at TEXT;

UPDATE fulfillment_windows
SET order_cutoff_at = starts_at
WHERE order_cutoff_at IS NULL OR trim(order_cutoff_at) = '';

CREATE INDEX idx_fulfillment_windows_pickup_location
    ON fulfillment_windows(pickup_location_id);

CREATE TABLE blackout_periods (
    id TEXT PRIMARY KEY NOT NULL,
    farm_id TEXT NOT NULL REFERENCES farms(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (ends_at > starts_at)
);

CREATE INDEX idx_blackout_periods_farm_starts_at
    ON blackout_periods(farm_id, starts_at, id);
