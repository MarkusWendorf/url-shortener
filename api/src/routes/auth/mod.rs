pub mod auth;

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::{
    middleware::auth::SESSION_COOKIE,
    sqlite,
    structs::{Login, Signup},
};

pub struct AuthAppState {
    connection: Connection,
}

pub fn router() -> Router {
    let connection = sqlite::create_connection();
    let state = Arc::new(Mutex::new(AuthAppState { connection }));

    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/logout", get(logout))
        .with_state(state)
}

async fn signup(State(state): State<Arc<Mutex<AuthAppState>>>, Json(payload): Json<Signup>) -> impl IntoResponse {
    let mut app_state = state.lock().await;
    let connection = &mut app_state.connection;
    println!("Signup {}", &payload.email);
    if let Ok(user) = auth::create_user(connection, &payload.email, &payload.password) {
        return (StatusCode::CREATED, format!("User created, id: {}", user.id)).into_response();
    }

    // TODO: error handling
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

async fn login(
    State(state): State<Arc<Mutex<AuthAppState>>>,
    mut jar: CookieJar,
    Json(payload): Json<Login>,
) -> impl IntoResponse {
    let mut app_state = state.lock().await;
    let connection = &mut app_state.connection;

    let user = match auth::verify_password(connection, &payload.email, &payload.password) {
        Ok(user) => user,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let (session_id, expires_at) = match auth::create_session(connection, user.id) {
        Ok(session) => session,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut cookie = Cookie::new(SESSION_COOKIE, session_id);
    cookie.set_expires(expires_at);
    jar = jar.add(cookie);

    (jar, StatusCode::OK).into_response()
}

async fn logout(State(state): State<Arc<Mutex<AuthAppState>>>, jar: CookieJar) -> impl IntoResponse {
    let mut app_state = state.lock().await;

    if let Some(session_cookie) = jar.get(SESSION_COOKIE) {
        auth::logout(&mut app_state.connection, session_cookie.value());
    }

    (jar.remove(SESSION_COOKIE), Redirect::temporary("/")).into_response()
}
