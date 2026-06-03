ALTER TABLE orders
    ADD COLUMN workflow_revision TEXT NOT NULL DEFAULT 'none' CHECK (
        workflow_revision IN ('none', 'change_proposed', 'updated', 'kept_as_placed')
    );
