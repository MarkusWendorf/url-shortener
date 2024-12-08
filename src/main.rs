#![feature(let_chains)]
mod id;
mod structs;

use std::fs;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;

use heed::types::Str;
use heed::MdbError;
use heed::PutFlags;
use heed::{Database, EnvOpenOptions};

use id::generate_id;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    pub db: heed::Database<Str, Str>,
    pub env: heed::Env,
}

#[tokio::main]
async fn main() {
    let path = std::path::Path::new("data");
    fs::create_dir_all(&path).unwrap();

    let env = unsafe {
        EnvOpenOptions::new()
            .map_size(10000 * 1024 * 1024)
            .open(path)
            .unwrap()
    };

    let mut tx = env.write_txn().unwrap();
    let db: Database<Str, Str> = env.create_database(&mut tx, None).unwrap();
    tx.commit().unwrap();

    let shared_state = Arc::new(AppState { db, env });

    let app = Router::new()
        .route("/create-short-url", post(create_short_url))
        .route("/:id", get(redirect_to_url))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_short_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let mut selected_id: Option<String> = None;

    let max_retries = 5;
    let mut retries = 0;

    loop {
        let mut tx = state.env.write_txn().unwrap();

        let generated_id = generate_id();

        let insert =
            state
                .db
                .put_with_flags(&mut tx, PutFlags::NO_OVERWRITE, &generated_id, &payload.url);

        match insert {
            Err(heed::Error::Mdb(MdbError::KeyExist)) if retries < max_retries => {
                // Retry until we have an id that does not already exist
                tx.abort();
                retries += 1;
                continue;
            }
            Err(err) => {
                println!("{:?}", err);
                tx.abort();
                break;
            }
            Ok(_) => {
                selected_id = tx.commit().ok().map(|_| generated_id);
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
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Ok(tx) = state.env.read_txn()
        && let Ok(Some(url)) = state.db.get(&tx, &id)
    {
        return Redirect::temporary(&url).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
