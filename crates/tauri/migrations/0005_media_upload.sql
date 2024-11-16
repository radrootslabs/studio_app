CREATE TABLE IF NOT EXISTS media_upload (
    id CHAR(36) PRIMARY KEY NOT NULL UNIQUE CHECK(length(id) = 36),
    created_at DATETIME NOT NULL CHECK(length(created_at) = 24),
    updated_at DATETIME NOT NULL CHECK(length(created_at) = 24),
    file_path TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    res_base TEXT NOT NULL,
    res_path TEXT NOT NULL,
    label TEXT,
    description TEXT
);