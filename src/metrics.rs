use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use rusqlite::Connection;
use uuid::Uuid;

pub struct Metric {
    pub shorthand_id: String,
    pub url: String,
    pub ip: String,
}

pub struct MetricsStorage {
    connection: Connection,
    buffer: Vec<Metric>,
    last_flush: Instant,
}

const MAX_BUFFER_SIZE: usize = 100;

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

        Self {
            connection,
            buffer: Vec::with_capacity(MAX_BUFFER_SIZE),
            last_flush: Instant::now(),
        }
    }

    pub fn add(&mut self, metric: Metric) -> Result<(), rusqlite::Error> {
        self.buffer.push(metric);

        if self.buffer.len() >= MAX_BUFFER_SIZE {
            return self.flush();
        }

        if self.last_flush.elapsed() >= Duration::from_secs(60) {
            return self.flush();
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), rusqlite::Error> {
        let tx = self.connection.transaction()?;
        println!("flush {:?}", self.buffer.len());

        for metric in self.buffer.drain(..) {
            let id = Uuid::now_v7();

            tx.execute(
                "INSERT INTO metrics (id, key, user_id, url, ip) VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    id.to_string(),
                    metric.shorthand_id,
                    9999,
                    metric.url,
                    metric.ip,
                ),
            )?;
        }

        tx.commit()?;
        self.last_flush = Instant::now();

        Ok(())
    }

    pub fn key_count(&self) -> u64 {
        self.connection
            .query_row("SELECT COUNT(*) FROM metrics", (), |row| {
                row.get::<usize, u64>(0)
            })
            .unwrap_or_default()
    }
}
