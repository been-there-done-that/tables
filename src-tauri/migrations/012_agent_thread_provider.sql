-- Migration 012: Add provider column to agent_threads
-- Existing rows default to 'claude' (backward compatible)
ALTER TABLE agent_threads ADD COLUMN provider TEXT NOT NULL DEFAULT 'claude';
