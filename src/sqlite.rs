use std::{fs, path::Path};

use rusqlite::Connection;

pub struct SqliteStorage {
    connection: Connection,
}

impl SqliteStorage {
    pub fn new() -> SqliteStorage {
        let data_dir = Path::new("./data");
        fs::create_dir_all(data_dir).unwrap();

        let connection = Connection::open(data_dir.join("db.sqlite")).unwrap();

        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();

        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .unwrap();

        connection
            .pragma_update(None, "wal_checkpoint", "TRUNCATE")
            .unwrap();

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS urls (key TEXT PRIMARY KEY, url TEXT, created_at INTEGER DEFAULT CURRENT_TIMESTAMP, user_id INTEGER)",
                (),
            )
            .unwrap();

        Self { connection }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let query = self
            .connection
            .prepare_cached("SELECT url FROM urls WHERE key = ?1");

        match query {
            Ok(mut query) => query
                .query_row([key], |row| row.get::<usize, String>(0))
                .ok(),
            Err(_) => None,
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<usize, rusqlite::Error> {
        let mut insert = self
            .connection
            .prepare_cached("INSERT INTO urls (key, url, user_id) VALUES (?1, ?2, ?3)")?;

        insert.execute((key, value, 9999))
    }

    pub fn key_count(&self) -> u64 {
        self.connection
            .query_row("SELECT COUNT(*) FROM urls", (), |row| {
                row.get::<usize, u64>(0)
            })
            .unwrap_or_default()
    }
}
