-- Initial migration for testing
-- This is a minimal migration to allow tests to run

-- Create a simple table for testing purposes
CREATE TABLE IF NOT EXISTS test_table (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP DEFAULT NOW()
);
