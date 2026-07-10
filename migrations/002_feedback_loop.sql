ALTER TABLE revision_notes ADD COLUMN platform TEXT;
ALTER TABLE revision_notes ADD COLUMN status TEXT NOT NULL DEFAULT 'delivered';
ALTER TABLE revision_notes ADD COLUMN source TEXT NOT NULL DEFAULT 'mcp';
ALTER TABLE generations ADD COLUMN review_state TEXT NOT NULL DEFAULT 'open';
