ALTER TABLE orders
    ADD COLUMN workflow_agreement TEXT NOT NULL DEFAULT 'ordered' CHECK (
        workflow_agreement IN ('ordered', 'confirmed', 'declined', 'cancelled', 'completed', 'needs_review')
    );

ALTER TABLE orders
    ADD COLUMN workflow_fulfillment TEXT CHECK (
        workflow_fulfillment IS NULL OR workflow_fulfillment IN ('confirmed', 'preparing', 'ready_for_pickup', 'out_for_delivery', 'delivered', 'cancelled')
    );

ALTER TABLE orders
    ADD COLUMN workflow_inventory TEXT NOT NULL DEFAULT 'needs_review' CHECK (
        workflow_inventory IN ('available', 'reserved', 'sold_out', 'needs_review')
    );

ALTER TABLE orders
    ADD COLUMN workflow_payment TEXT NOT NULL DEFAULT 'not_recorded' CHECK (
        workflow_payment IN ('not_recorded', 'recorded', 'needs_review')
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
        WHEN 'completed' THEN 'completed'
        WHEN 'declined' THEN 'declined'
        WHEN 'refunded' THEN 'needs_review'
        ELSE 'ordered'
    END,
    workflow_fulfillment = CASE status
        WHEN 'scheduled' THEN 'confirmed'
        WHEN 'packed' THEN 'ready_for_pickup'
        WHEN 'completed' THEN 'delivered'
        WHEN 'declined' THEN 'cancelled'
        ELSE NULL
    END,
    workflow_inventory = CASE status
        WHEN 'scheduled' THEN 'reserved'
        WHEN 'packed' THEN 'reserved'
        WHEN 'completed' THEN 'reserved'
        WHEN 'declined' THEN 'available'
        ELSE 'needs_review'
    END,
    workflow_payment = CASE status
        WHEN 'refunded' THEN 'needs_review'
        ELSE 'not_recorded'
    END;
