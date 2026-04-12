ALTER TABLE request_logs ADD COLUMN source TEXT;
CREATE INDEX IF NOT EXISTS idx_request_logs_source_created_at ON request_logs(source, created_at DESC);
