use rusqlite::Connection;

pub mod auth;

pub struct MiddlewareState {
    pub connection: Connection,
}
