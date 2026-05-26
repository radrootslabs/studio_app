ALTER TABLE local_interop_imports
    ADD COLUMN event_pubkey TEXT;

ALTER TABLE local_interop_imports
    ADD COLUMN event_created_at INTEGER;

ALTER TABLE local_interop_imports
    ADD COLUMN event_tags_json TEXT;

ALTER TABLE local_interop_imports
    ADD COLUMN event_content TEXT;

ALTER TABLE local_interop_imports
    ADD COLUMN event_sig TEXT;

ALTER TABLE local_interop_imports
    ADD COLUMN raw_event_json TEXT;
