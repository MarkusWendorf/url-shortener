use rusqlite::Connection;

pub fn create_short_url(
    connection: &mut Connection,
    user_id: u64,
    key: &str,
    value: &str,
) -> Result<usize, rusqlite::Error> {
    let mut insert = connection.prepare_cached("INSERT INTO urls (key, url, user_id) VALUES (?1, ?2, ?3)")?;

    insert.execute((key, value, user_id))
}
