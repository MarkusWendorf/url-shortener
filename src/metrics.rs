use std::{fs, path::Path};

use rusqlite::Connection;
use uuid::Uuid;

pub struct MetricsStorage {
    connection: Connection,
}

impl MetricsStorage {
    pub fn new() -> Self {
        let data_dir = Path::new("./data");
        fs::create_dir_all(data_dir).unwrap();

        let connection = Connection::open(data_dir.join("metrics.sqlite")).unwrap();

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
                r"
                  CREATE TABLE IF NOT EXISTS metrics (
                    id TEXT,
                    user_id TEXT,
                    key TEXT, 
                    url TEXT, 
                    created_at INTEGER DEFAULT CURRENT_TIMESTAMP, 
                    ip TEXT,
                    PRIMARY KEY (user_id, key, id)
                  )
                  ",
                (),
            )
            .unwrap();

        Self { connection }
    }

    pub fn set(&self, key: &str, url: &str, ip: &str) -> Result<usize, rusqlite::Error> {
        let mut insert = self.connection.prepare_cached(
            "INSERT INTO metrics (id, key, user_id, url, ip) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        let id = Uuid::now_v7();
        insert.execute((id.to_string(), key, 9999, url, ip))
    }

    pub fn key_count(&self) -> u64 {
        self.connection
            .query_row("SELECT COUNT(*) FROM metrics", (), |row| {
                row.get::<usize, u64>(0)
            })
            .unwrap_or_default()
    }
}
