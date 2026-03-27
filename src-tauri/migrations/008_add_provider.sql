-- 008_add_provider.sql
-- Add provider label to connections (nullable; NULL = generic engine, no provider)
ALTER TABLE connections ADD COLUMN provider TEXT;
