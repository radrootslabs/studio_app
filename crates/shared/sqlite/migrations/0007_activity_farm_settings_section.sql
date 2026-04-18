ALTER TABLE activity_events RENAME TO activity_events_old;

CREATE TABLE activity_events (
    activity_event_id TEXT PRIMARY KEY,
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    event_kind TEXT NOT NULL,
    settings_section TEXT,
    settings_preference TEXT,
    preference_enabled INTEGER,
    CHECK (
        event_kind IN (
            'home_opened',
            'settings_opened',
            'settings_section_selected',
            'settings_preference_updated'
        )
    ),
    CHECK (
        settings_section IS NULL
        OR settings_section IN ('account', 'farm', 'settings', 'about')
    ),
    CHECK (
        settings_preference IS NULL
        OR settings_preference IN (
            'allow_relay_connections',
            'use_media_servers',
            'use_nip05',
            'launch_at_login'
        )
    ),
    CHECK (preference_enabled IS NULL OR preference_enabled IN (0, 1))
);

INSERT INTO activity_events (
    activity_event_id,
    recorded_at,
    event_kind,
    settings_section,
    settings_preference,
    preference_enabled
)
SELECT
    activity_event_id,
    recorded_at,
    event_kind,
    settings_section,
    settings_preference,
    preference_enabled
FROM activity_events_old;

DROP TABLE activity_events_old;

CREATE INDEX activity_events_recorded_at_idx
    ON activity_events(recorded_at DESC, activity_event_id DESC);
