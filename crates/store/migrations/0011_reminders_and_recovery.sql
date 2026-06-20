CREATE TABLE reminder_schedules (
    reminder_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    farm_id TEXT NOT NULL,
    order_id TEXT,
    fulfillment_window_id TEXT,
    reminder_kind TEXT NOT NULL CHECK (
        reminder_kind IN (
            'fulfillment_window',
            'order_action',
            'missed_pickup_recovery',
            'sync_impact'
        )
    ),
    reminder_surface TEXT NOT NULL CHECK (
        reminder_surface IN ('today', 'orders', 'pack_day')
    ),
    reminder_urgency TEXT NOT NULL CHECK (
        reminder_urgency IN ('upcoming', 'due_soon', 'overdue', 'blocking')
    ),
    title TEXT NOT NULL,
    detail TEXT NOT NULL,
    deadline_at TEXT NOT NULL,
    action_label TEXT,
    delivery_state TEXT NOT NULL CHECK (
        delivery_state IN ('scheduled', 'presented', 'acknowledged', 'resolved')
    ),
    PRIMARY KEY (account_id, farm_id, reminder_id)
);

CREATE TABLE reminder_log_entries (
    log_entry_id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    farm_id TEXT NOT NULL,
    reminder_id TEXT NOT NULL,
    reminder_kind TEXT NOT NULL CHECK (
        reminder_kind IN (
            'fulfillment_window',
            'order_action',
            'missed_pickup_recovery',
            'sync_impact'
        )
    ),
    title TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    delivery_state TEXT NOT NULL CHECK (
        delivery_state IN ('scheduled', 'presented', 'acknowledged', 'resolved')
    ),
    detail TEXT
);

CREATE TABLE order_recovery_records (
    recovery_record_id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    farm_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    recovery_kind TEXT NOT NULL CHECK (
        recovery_kind IN ('missed_pickup')
    ),
    recovery_state TEXT NOT NULL CHECK (
        recovery_state IN ('open', 'in_review', 'resolved')
    ),
    summary TEXT NOT NULL,
    note TEXT,
    last_updated_at TEXT NOT NULL,
    UNIQUE(account_id, order_id, recovery_kind)
);

CREATE INDEX idx_reminder_schedules_account_farm_deadline ON reminder_schedules(
    account_id,
    farm_id,
    deadline_at,
    reminder_id
);
CREATE INDEX idx_reminder_schedules_account_farm_surface ON reminder_schedules(
    account_id,
    farm_id,
    reminder_surface,
    deadline_at
);
CREATE INDEX idx_reminder_log_entries_account_farm_recorded_at ON reminder_log_entries(
    account_id,
    farm_id,
    recorded_at,
    log_entry_id
);
CREATE INDEX idx_reminder_log_entries_account_farm_reminder ON reminder_log_entries(
    account_id,
    farm_id,
    reminder_id
);
CREATE INDEX idx_order_recovery_records_account_farm_updated_at ON order_recovery_records(
    account_id,
    farm_id,
    last_updated_at,
    recovery_record_id
);
CREATE INDEX idx_order_recovery_records_account_order_kind ON order_recovery_records(
    account_id,
    order_id,
    recovery_kind
);
