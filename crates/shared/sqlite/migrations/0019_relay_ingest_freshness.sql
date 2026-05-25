CREATE TABLE app_relay_ingest_freshness (
    scope_key TEXT NOT NULL,
    relay_url TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('fresh', 'stale', 'failed')),
    cursor_since_unix_seconds INTEGER,
    last_event_created_at_unix_seconds INTEGER,
    last_fetch_started_at TEXT NOT NULL,
    last_fetch_started_unix_seconds INTEGER NOT NULL,
    last_fetch_completed_at TEXT,
    last_fetch_completed_unix_seconds INTEGER,
    last_success_at TEXT,
    last_success_unix_seconds INTEGER,
    last_error_message TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(scope_key, relay_url)
);

CREATE INDEX idx_app_relay_ingest_freshness_scope_state
    ON app_relay_ingest_freshness(scope_key, state, relay_url);
