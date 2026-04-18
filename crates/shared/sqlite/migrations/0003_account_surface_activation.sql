CREATE TABLE account_surface_activations (
    account_id TEXT PRIMARY KEY NOT NULL,
    selected_surface TEXT NOT NULL CHECK (selected_surface IN ('personal', 'farmer')),
    farmer_farm_id TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_account_surface_activations_updated_at
    ON account_surface_activations(updated_at DESC, account_id DESC);
