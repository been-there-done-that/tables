-- Generic Key-Value Settings Store
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Seed defaults
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES 
('editor_font_family', '"Fira Code", monospace', strftime('%s', 'now')),
('editor_font_size', '14', strftime('%s', 'now'));
