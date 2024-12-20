#![feature(let_chains)]
mod id;
mod storage;
mod structs;

use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;

use id::generate_id;

use storage::lmdb::LmdbStorage;
use storage::sqlite::SqliteStorage;
use storage::storage::{Error, Storage};
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    pub storage: Box<dyn Storage>,
}

#[tokio::main]
async fn main() {
    let storage = LmdbStorage::new();
    println!("Key count: {:?}", storage.key_count());

    let shared_state = Arc::new(Mutex::new(AppState {
        storage: Box::new(storage),
    }));

    let app = Router::new()
        .route("/create-short-url", post(create_short_url))
        .route("/:id", get(redirect_to_url))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_short_url(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let mut selected_id: Option<String> = None;

    let max_retries = 5;
    let mut retries = 0;

    loop {
        let generated_id = generate_id();

        let insert = state
            .lock()
            .unwrap()
            .storage
            .set(&generated_id, &payload.url);

        match insert {
            Err(Error::DuplicateKey) if retries < max_retries => {
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
    State(state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.lock().unwrap().storage.get(&id) {
        Some(url) => Redirect::temporary(&url).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
