#![feature(let_chains)]
#![feature(const_for)]
mod headers;
mod id;
mod metrics;
mod migrations;
mod structs;
mod url_storage;

use headers::*;
use rusqlite::Connection;
use std::sync::Arc;
use std::{fs, path};
use std::{net::SocketAddr, time::Duration};
use tokio::{sync::Mutex, time};
use tokio_postgres::NoTls;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};

use axum_extra::extract::cookie::{Cookie, CookieJar};

use rusqlite::Error::SqliteFailure;

use id::generate_id;
use metrics::{persist_metrics, Metric};
use structs::{CreateShortUrl, ShortUrlCreated};
use url_storage::UrlStorage;

struct AppState {
    storage: UrlStorage,
    metrics_buffer: Vec<Metric>,
    pool: deadpool_postgres::Pool,
}

const VISITOR_COOKIE: &str = "visitor-id";

#[tokio::main]
async fn main() {
    let data_dir = path::Path::new("./data");
    fs::create_dir_all(data_dir).unwrap();

    let mut connection = Connection::open(data_dir.join("db2.sqlite")).unwrap();
    connection.pragma_update(None, "journal_mode", "WAL").unwrap();
    connection.pragma_update(None, "synchronous", "NORMAL").unwrap();
    connection.pragma_update(None, "wal_checkpoint", "TRUNCATE").unwrap();

    let mut deadpool = deadpool_postgres::Config::new();
    deadpool.dbname = Some("metrics".to_string());
    deadpool.host = Some("localhost".to_string());
    deadpool.port = Some(6432);
    deadpool.user = Some("postgres".to_string());
    deadpool.password = Some("password".to_string());

    deadpool.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    let pool = deadpool
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .unwrap();

    migrations::run_migrations(&mut connection, &pool).await;

    let storage = UrlStorage::new(connection);

    let state = Arc::new(Mutex::new(AppState {
        storage,
        metrics_buffer: Vec::with_capacity(100000),
        pool,
    }));

    let mut interval = time::interval(Duration::from_secs(10));
    let app_state = state.clone();

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let mut app = app_state.lock().await;
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();

            if let Ok(client) = app.pool.get().await {
                persist_metrics(client, metrics).await.ok();
            }
        }
    });

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
    mut jar: CookieJar,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Mutex<AppState>>>,
    Path(id): Path<String>,
) -> Result<(CookieJar, Redirect), StatusCode> {
    let visitor_id = match jar.get(VISITOR_COOKIE) {
        Some(cookie) => cookie.value().to_owned(),
        None => {
            let id = generate_id();
            let cookie = Cookie::new(VISITOR_COOKIE, id.clone());
            jar = jar.add(cookie);
            id
        }
    };

    let mut app = state.lock().await;

    if let Some(url) = app.storage.get(&id) {
        let metric = Metric {
            visitor_id,
            shorthand_id: id,
            ip: header_to_string(&headers, "cloudfront-viewer-address").unwrap_or_else(|| addr.ip().to_string()),
            url: url.clone(),
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

        app.metrics_buffer.push(metric);

        if app.metrics_buffer.len() >= 1000
            && let Ok(client) = app.pool.get().await
        {
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();
            tokio::spawn(async { persist_metrics(client, metrics).await.unwrap() });
        }

        return Ok((jar, Redirect::temporary(&url)));
    }

    Err(StatusCode::NOT_FOUND)
}
