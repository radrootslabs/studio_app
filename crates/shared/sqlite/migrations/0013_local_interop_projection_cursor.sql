CREATE TABLE local_interop_projection_cursor (
    consumer_id TEXT PRIMARY KEY NOT NULL,
    last_change_seq INTEGER NOT NULL CHECK (last_change_seq >= 0),
    updated_at TEXT NOT NULL
);
