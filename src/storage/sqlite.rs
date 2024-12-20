use rusqlite::Connection;

use super::storage::{Error, Storage};

pub struct SqliteStorage {
    connection: Connection,
}

impl SqliteStorage {
    pub fn new() -> SqliteStorage {
        let connection = Connection::open("./data/db.sqlite").unwrap();

        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();

        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .unwrap();

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS urls (key TEXT PRIMARY KEY, url TEXT)",
                (),
            )
            .unwrap();

        SqliteStorage { connection }
    }
}

impl Storage for SqliteStorage {
    fn get(&self, key: &str) -> Option<String> {
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

    fn set(&self, key: &str, value: &str) -> Result<(), Error> {
        let mut insert = self
            .connection
            .prepare_cached("INSERT INTO urls (key, url) VALUES (?1, ?2)")
            .map_err(|_| Error::GenericError)?;

        insert
            .execute((key, value))
            .map(|_| ())
            .map_err(|err| match err {
                _ => Error::GenericError,
            })
    }

    fn key_count(&self) -> u64 {
        self.connection
            .query_row("SELECT COUNT(*) FROM urls", (), |row| {
                row.get::<usize, u64>(0)
            })
            .unwrap_or_default()
    }
}
