CREATE TABLE users (
	id INTEGER PRIMARY KEY,
	email TEXT UNIQUE NOT NULL,
	pw_hash TEXT NOT NULL
);

CREATE INDEX email_idx ON users (email);