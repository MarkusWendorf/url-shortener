pub mod api;

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Extension, Json, Router};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::{
    id::generate_id,
    middleware::auth::UserSession,
    sqlite,
    structs::{CreateShortUrl, ShortUrlCreated},
};

pub struct ApiAppState {
    connection: Connection,
}

pub fn router() -> Router {
    let connection = sqlite::create_connection();
    let state = Arc::new(Mutex::new(ApiAppState { connection }));

    Router::new()
        .route("/create-short-url", post(create_short_url))
        .with_state(state)
}

async fn create_short_url(
    State(state): State<Arc<Mutex<ApiAppState>>>,
    session: Extension<UserSession>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let mut app_state = state.lock().await;
    let connection = &mut app_state.connection;
    println!("{:?}", session.user);
    let user_id = 64;

    let mut retries = 0;

    while retries < 5 {
        let id = generate_id();
        match api::create_short_url(connection, user_id, &id, &payload.url) {
            Ok(_) => return (StatusCode::CREATED, Json(ShortUrlCreated { id })).into_response(),
            // Duplicate Key (code=1555)
            Err(rusqlite::Error::SqliteFailure(err, _)) if err.extended_code == 1555 => retries += 1,
            Err(_) => break,
        }
    }

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
