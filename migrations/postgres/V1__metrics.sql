CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS metrics (
	id TEXT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	user_id BIGINT,
	key TEXT,
	url TEXT,
	ip TEXT,
	android BOOL,
	ios BOOL,
	mobile BOOL,
	region_name TEXT,
	country TEXT,
	city TEXT,
	zip_code TEXT,
	time_zone TEXT,
	user_agent TEXT,
  visitor_id TEXT,
  location GEOGRAPHY,
	PRIMARY KEY (user_id, visitor_id, id, created_at)
);

SELECT create_hypertable('metrics', by_range('created_at'));