ALTER TABLE orders
    ADD COLUMN workflow_agreement TEXT NOT NULL DEFAULT 'requested' CHECK (
        workflow_agreement IN ('requested', 'agreed_pending_validation', 'committed', 'declined', 'cancelled', 'validation_expired', 'invalid')
    );

ALTER TABLE orders
    ADD COLUMN workflow_inventory TEXT NOT NULL DEFAULT 'needs_review' CHECK (
        workflow_inventory IN ('available', 'reserved', 'sold_out', 'needs_review')
    );

ALTER TABLE orders
    ADD COLUMN workflow_provenance_source TEXT NOT NULL DEFAULT 'unknown' CHECK (
        workflow_provenance_source IN ('app', 'cli', 'relay', 'runtime_store', 'unknown')
    );

ALTER TABLE orders
    ADD COLUMN workflow_provenance_last_event_id TEXT;

UPDATE orders
SET
    workflow_agreement = CASE status
        WHEN 'scheduled' THEN 'committed'
        WHEN 'packed' THEN 'committed'
        WHEN 'completed' THEN 'committed'
        WHEN 'declined' THEN 'declined'
        WHEN 'needs_review' THEN 'invalid'
        ELSE 'requested'
    END,
    workflow_inventory = CASE status
        WHEN 'scheduled' THEN 'reserved'
        WHEN 'packed' THEN 'reserved'
        WHEN 'completed' THEN 'reserved'
        WHEN 'declined' THEN 'available'
        ELSE 'needs_review'
    END;
