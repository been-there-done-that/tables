-- Migration 001: Create themes table
-- Simple theme storage for built-in and user themes

CREATE TABLE IF NOT EXISTS themes (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  description TEXT,
  theme_data TEXT NOT NULL,
  is_builtin INTEGER DEFAULT 0,
  is_active INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_themes_is_active ON themes(is_active);
CREATE INDEX IF NOT EXISTS idx_themes_is_builtin ON themes(is_builtin);
CREATE INDEX IF NOT EXISTS idx_themes_name ON themes(name COLLATE NOCASE);