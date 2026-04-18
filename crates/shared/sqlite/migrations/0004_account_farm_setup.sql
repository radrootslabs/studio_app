CREATE TABLE account_farm_setups (
    account_id TEXT PRIMARY KEY NOT NULL,
    farm_name TEXT NOT NULL,
    location_or_service_area TEXT NOT NULL,
    pickup_enabled INTEGER NOT NULL CHECK (pickup_enabled IN (0, 1)),
    delivery_enabled INTEGER NOT NULL CHECK (delivery_enabled IN (0, 1)),
    shipping_enabled INTEGER NOT NULL CHECK (shipping_enabled IN (0, 1)),
    saved_farm_id TEXT,
    saved_farm_display_name TEXT,
    saved_farm_readiness TEXT CHECK (
        saved_farm_readiness IS NULL
        OR saved_farm_readiness IN ('incomplete', 'ready')
    ),
    updated_at TEXT NOT NULL,
    CHECK (
        (
            saved_farm_id IS NULL
            AND saved_farm_display_name IS NULL
            AND saved_farm_readiness IS NULL
        )
        OR (
            saved_farm_id IS NOT NULL
            AND saved_farm_display_name IS NOT NULL
            AND saved_farm_readiness IS NOT NULL
        )
    )
);

CREATE INDEX idx_account_farm_setups_updated_at
    ON account_farm_setups(updated_at DESC, account_id DESC);
