ALTER TABLE meta_schemas ADD COLUMN kind TEXT DEFAULT 'schema';
-- We can't easily enforce the enum check using CHECK because SQLite ALTER TABLE is limited, 
-- but we can try if we want. Standard practice for SQLite ALTER ADD COLUMN is usually simple.
-- Ideally: CHECK (kind IN ('database', 'schema', 'logical_group'))
