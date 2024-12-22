#![feature(let_chains)]
mod id;
mod metrics;
mod sqlite;
mod structs;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use flume::Sender;
use rusqlite::Error::SqliteFailure;

use id::generate_id;
use metrics::{Metric, MetricsStorage};
use sqlite::SqliteStorage;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    storage: SqliteStorage,
    sender: Sender<Metric>,
}

#[tokio::main]
async fn main() {
    let storage = SqliteStorage::new();
    let metrics = MetricsStorage::new();

    let (sender, receiver) = flume::unbounded();
    tokio::spawn(metrics_handler(receiver, metrics));

    let state = Arc::new(Mutex::new(AppState { storage, sender }));
    let app = create_router(state);

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn create_router(state: Arc<Mutex<AppState>>) -> Router {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .route("/create-short-url", post(create_short_url))
        .route("/:id", get(redirect_to_url))
        .with_state(state)
}

async fn metrics_handler(receiver: flume::Receiver<Metric>, mut metrics: MetricsStorage) {
    while let Ok(metric) = receiver.recv_async().await {
        metrics.add(metric).ok();
    }
}

async fn create_short_url(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let app = state.lock().await;
    let mut retries = 0;

    while retries < 5 {
        let id = generate_id();
        match app.storage.set(&id, &payload.url) {
            Ok(_) => return (StatusCode::CREATED, Json(ShortUrlCreated { id })).into_response(),
            Err(SqliteFailure(err, _)) if err.extended_code == 1555 => retries += 1,
            Err(_) => break,
        }
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
        let metric = Metric {
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
        };

        app.sender.send_async(metric).await.ok();

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
        .and_then(|v| v.to_str().ok().map(String::from))
}

fn header_to_float(headers: &HeaderMap, key: &str) -> Option<f64> {
    headers.get(key).and_then(|v| v.to_str().ok()?.parse().ok())
}
