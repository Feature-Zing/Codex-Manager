ALTER TABLE aggregate_apis ADD COLUMN last_probe_at INTEGER;
ALTER TABLE aggregate_apis ADD COLUMN last_probe_status TEXT;
ALTER TABLE aggregate_apis ADD COLUMN last_probe_error TEXT;
ALTER TABLE aggregate_apis ADD COLUMN last_probe_latency_ms INTEGER;
ALTER TABLE aggregate_apis ADD COLUMN last_probe_http_status INTEGER;
