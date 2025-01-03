use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::entities::User;

use super::MiddlewareState;

pub const SESSION_COOKIE: &str = "session";

#[derive(Clone)]
pub struct UserSession {
    pub session_id: String,
    pub user: User,
}

pub async fn authorization_middleware(
    State(state): State<Arc<Mutex<MiddlewareState>>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let jar = CookieJar::from_headers(req.headers());

    let session_id = match jar.get(SESSION_COOKIE) {
        Some(cookie) => cookie.value(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let mut state = state.lock().await;

    let user = match find_user_by_session_id(&mut state.connection, session_id) {
        Ok(user) => user,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    drop(state);

    req.extensions_mut().insert(UserSession {
        user,
        session_id: session_id.to_owned(),
    });

    Ok(next.run(req).await)
}

pub fn find_user_by_session_id(connection: &mut Connection, session_id: &str) -> Result<User, rusqlite::Error> {
    connection.query_row(
        r"SELECT id, email FROM users WHERE id = (
              SELECT user_id FROM sessions WHERE session_id = ?1 AND unixepoch() <= expires_at
            )",
        [session_id],
        |row| {
            let id: u64 = row.get(0)?;
            let email: String = row.get(1)?;

            Ok(User { email, id })
        },
    )
}
