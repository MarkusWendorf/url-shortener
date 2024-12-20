#![feature(let_chains)]
mod id;
mod sqlite;
mod structs;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use rusqlite::Error::SqliteFailure;

use id::generate_id;

use sqlite::SqliteStorage;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    pub storage: Mutex<SqliteStorage>,
}

#[tokio::main]
async fn main() {
    let storage = SqliteStorage::new();
    println!("Key count: {:?}", storage.key_count());

    let shared_state = Arc::new(AppState {
        storage: Mutex::new(storage),
    });

    let app = Router::new()
        .route("/create-short-url", post(create_short_url))
        .route("/:id", get(redirect_to_url))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn create_short_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let storage = match state.storage.lock() {
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Ok(storage) => storage,
    };

    let mut selected_id: Option<String> = None;

    let max_retries = 5;
    let mut retries = 0;

    loop {
        let generated_id = generate_id();

        match storage.set(&generated_id, &payload.url) {
            // Duplicate key, try again
            Err(SqliteFailure(err, _)) if err.extended_code == 1555 && retries < max_retries => {
                retries += 1;
                continue;
            }
            Err(err) => {
                println!("Error {:?}", err);
                break;
            }
            Ok(_) => {
                selected_id = Some(generated_id);
                break;
            }
        }
    }

    if let Some(id) = selected_id {
        return (StatusCode::CREATED, Json(ShortUrlCreated { id })).into_response();
    }

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

async fn redirect_to_url(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    println!("{:?}", addr.ip());

    let storage = match state.storage.lock() {
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Ok(storage) => storage,
    };

    match storage.get(&id) {
        Some(url) => Redirect::temporary(&url).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
