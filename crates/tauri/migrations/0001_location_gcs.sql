CREATE TABLE IF NOT EXISTS location_gcs (
    id CHAR(36) PRIMARY KEY NOT NULL UNIQUE CHECK(length(id) = 36),
    created_at DATETIME NOT NULL CHECK(length(created_at) = 24),
    updated_at DATETIME NOT NULL CHECK(length(created_at) = 24),
    lat FLOAT NOT NULL,
    lng FLOAT NOT NULL,
    geohash CHAR(12) NOT NULL UNIQUE,
    label TEXT,
    gc_id TEXT,
    gc_name TEXT,
    gc_admin1_id TEXT,
    gc_admin1_name TEXT,
    gc_country_id TEXT,
    gc_country_name TEXT
);