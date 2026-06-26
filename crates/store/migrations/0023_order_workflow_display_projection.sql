ALTER TABLE orders
    ADD COLUMN workflow_agreement TEXT NOT NULL DEFAULT 'ordered' CHECK (
        workflow_agreement IN ('ordered', 'pending_rhi', 'confirmed', 'declined', 'cancelled', 'needs_review')
    );

ALTER TABLE orders
    ADD COLUMN workflow_inventory TEXT NOT NULL DEFAULT 'needs_review' CHECK (
        workflow_inventory IN ('available', 'reserved', 'sold_out', 'needs_review')
    );

ALTER TABLE orders
    ADD COLUMN workflow_provenance_source TEXT NOT NULL DEFAULT 'unknown' CHECK (
        workflow_provenance_source IN ('app', 'cli', 'relay', 'local_events', 'unknown')
    );

ALTER TABLE orders
    ADD COLUMN workflow_provenance_last_event_id TEXT;

UPDATE orders
SET
    workflow_agreement = CASE status
        WHEN 'scheduled' THEN 'confirmed'
        WHEN 'packed' THEN 'confirmed'
        WHEN 'completed' THEN 'confirmed'
        WHEN 'declined' THEN 'declined'
        ELSE 'ordered'
    END,
    workflow_inventory = CASE status
        WHEN 'scheduled' THEN 'reserved'
        WHEN 'packed' THEN 'reserved'
        WHEN 'completed' THEN 'reserved'
        WHEN 'declined' THEN 'available'
        ELSE 'needs_review'
    END;
