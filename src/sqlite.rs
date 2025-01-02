use std::{fs, path};

use rusqlite::Connection;

mod sqlite_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/sqlite");
}

pub fn run_migrations(conn: &mut rusqlite::Connection) {
    sqlite_migrations::migrations::runner().run(conn).unwrap();
}

pub fn create_connection() -> Connection {
    let data_dir = path::Path::new("./data");
    fs::create_dir_all(data_dir).unwrap();

    let connection = Connection::open(data_dir.join("db2.sqlite")).unwrap();
    connection.pragma_update(None, "journal_mode", "WAL").unwrap();
    connection.pragma_update(None, "synchronous", "NORMAL").unwrap();
    connection.pragma_update(None, "wal_checkpoint", "TRUNCATE").unwrap();

    connection
}
