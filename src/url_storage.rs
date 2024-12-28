use rusqlite::Connection;

pub struct UrlStorage {
    connection: Connection,
}

impl UrlStorage {
    pub fn new(connection: Connection) -> UrlStorage {
        Self { connection }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let query = self.connection.prepare_cached("SELECT url FROM urls WHERE key = ?1");

        match query {
            Ok(mut query) => query.query_row([key], |row| row.get::<usize, String>(0)).ok(),
            Err(_) => None,
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<usize, rusqlite::Error> {
        let mut insert = self
            .connection
            .prepare_cached("INSERT INTO urls (key, url, user_id) VALUES (?1, ?2, ?3)")?;

        insert.execute((key, value, 9999))
    }
}
