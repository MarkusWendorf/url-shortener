#![feature(let_chains)]
mod id;
mod metrics;
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
use metrics::MetricsStorage;
use rusqlite::Error::SqliteFailure;

use id::generate_id;

use sqlite::SqliteStorage;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    pub storage: SqliteStorage,
    pub metrics: MetricsStorage,
}

#[tokio::main]
async fn main() {
    let storage = SqliteStorage::new();
    let metrics = MetricsStorage::new();

    println!("Key count: {:?}", metrics.key_count());

    let shared_state = Arc::new(Mutex::new(AppState { storage, metrics }));

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
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let app = match state.lock() {
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Ok(app) => app,
    };

    let mut selected_id: Option<String> = None;

    let max_retries = 5;
    let mut retries = 0;

    loop {
        let generated_id = generate_id();

        match app.storage.set(&generated_id, &payload.url) {
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
    State(state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    println!("{:?}", addr.ip());

    let app = match state.lock() {
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Ok(app) => app,
    };

    if let Some(url) = app.storage.get(&id) {
        let _ = app.metrics.set(&id, &url, &addr.ip().to_string());

        return Redirect::temporary(&url).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
