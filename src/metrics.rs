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
    pub android: Option<bool>,
    pub ios: Option<bool>,
    pub mobile: Option<bool>,
    pub region_name: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub zip_code: Option<String>,
    pub time_zone: Option<String>,
    pub user_agent: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

pub struct MetricsStorage {
    connection: Connection,
    buffer: Vec<Metric>,
    last_flush: Instant,
}

const MAX_BUFFER_SIZE: usize = 10;

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
                    android INTEGER,
                    ios INTEGER,
                    mobile INTEGER,
                    region_name TEXT,
                    country TEXT,
                    city TEXT,
                    zip_code TEXT,
                    time_zone TEXT,
                    user_agent TEXT,
                    longitude REAL,
                    latitude REAL,
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
                "INSERT INTO metrics 
                    (id, key, user_id, url, ip, android, ios, mobile, region_name, country, city, zip_code, time_zone, user_agent, longitude, latitude) VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                (
                    id.to_string(),
                    metric.shorthand_id,
                    9999,
                    metric.url,
                    metric.ip,
                    metric.android,
                    metric.ios,
                    metric.mobile,
                    metric.region_name,
                    metric.country,
                    metric.city,
                    metric.zip_code,
                    metric.time_zone,
                    metric.user_agent,
                    metric.longitude,
                    metric.latitude,
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
