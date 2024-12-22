#![feature(let_chains)]
mod id;
mod metrics;
mod sqlite;
mod structs;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use flume::Sender;
use metrics::{Metric, MetricsStorage};
use rusqlite::Error::SqliteFailure;

use id::generate_id;

use sqlite::SqliteStorage;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    pub storage: SqliteStorage,
    pub sender: Sender<Metric>,
}

#[tokio::main]
async fn main() {
    let storage = SqliteStorage::new();
    let mut metrics = MetricsStorage::new();

    println!("Key count: {:?}", metrics.key_count());

    let (sender, receiver) = flume::unbounded::<Metric>();
    tokio::spawn(async move {
        while let Ok(metric) = receiver.recv_async().await {
            let _ = metrics.add(metric);
        }
    });

    let shared_state = Arc::new(Mutex::new(AppState { storage, sender }));

    let app = Router::new()
        .route("/", get(health_check))
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

async fn health_check() -> impl IntoResponse {
    StatusCode::OK.into_response()
}

async fn create_short_url(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let app = state.lock().await;

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
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let app = state.lock().await;

    if let Some(url) = app.storage.get(&id) {
        let _ = app
            .sender
            .send_async(Metric {
                ip: header_to_string(&headers, "cloudfront-viewer-address")
                    .unwrap_or_else(|| addr.ip().to_string()),
                url: url.clone(),
                shorthand_id: id,
                android: header_to_bool(&headers, "cloudfront-is-android-viewer"),
                ios: header_to_bool(&headers, "cloudfront-is-ios-viewer"),
                mobile: header_to_bool(&headers, "cloudfront-is-mobile-viewer"),
                region_name: header_to_string(&headers, "cloudfront-viewer-country-region-name"),
                country: header_to_string(&headers, "cloudfront-viewer-country"),
                city: header_to_string(&headers, "cloudfront-viewer-city"),
                zip_code: header_to_string(&headers, "cloudfront-viewer-postal-code"),
                time_zone: header_to_string(&headers, "cloudfront-viewer-time-zone"),
                user_agent: header_to_string(&headers, "user-agent"),
                longitude: header_to_float(&headers, "cloudfront-viewer-longitude"),
                latitude: header_to_float(&headers, "cloudfront-viewer-latitude"),
            })
            .await;

        return Redirect::temporary(&url).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}

fn header_to_bool(headers: &HeaderMap, key: &str) -> Option<bool> {
    headers.get(key).map(|value| value == "true")
}

fn header_to_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .map(|value| String::from(value.to_str().unwrap_or_default()))
}

fn header_to_float(headers: &HeaderMap, key: &str) -> Option<f64> {
    if let Some(value) = headers.get(key) {
        return value.to_str().unwrap_or_default().parse::<f64>().ok();
    }

    None
}
