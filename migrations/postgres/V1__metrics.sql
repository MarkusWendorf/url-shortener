CREATE TABLE IF NOT EXISTS metrics (
	id TEXT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	user_id TEXT,
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
	longitude FLOAT8,
	latitude FLOAT8,
  visitor_id TEXT,
	PRIMARY KEY (user_id, visitor_id, id, created_at)
);

SELECT create_hypertable('metrics', by_range('created_at'));