#![feature(let_chains)]
#![feature(const_for)]
mod id;
mod metrics;
mod sqlite;
mod structs;

use std::sync::Arc;
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
use metrics::{flush_direct, Metric};
use sqlite::SqliteStorage;
use structs::{CreateShortUrl, ShortUrlCreated};

struct AppState {
    storage: SqliteStorage,
    metrics_buffer: Vec<Metric>,
    pool: deadpool_postgres::Pool,
}

const VISITOR_COOKIE: &str = "visitor-id";

#[tokio::main]
async fn main() {
    let storage = SqliteStorage::new();

    // for _ in 0..5000000 {
    //     let visitor_id: i32 = thread_rng().gen_range(1..100);

    //     metrics
    //         .add(Metric {
    //             android: Some(true),
    //             ios: Some(true),
    //             mobile: Some(true),
    //             city: Some("Rostock".to_owned()),
    //             country: Some("DE".to_owned()),
    //             region_name: Some("Mecklenburg-Vorpommern".to_owned()),
    //             time_zone: Some("Europe/Berlin".to_owned()),
    //             user_agent: Some("chrome".to_owned()),
    //             zip_code: Some("18057".to_owned()),
    //             ip: "asd".to_owned(),
    //             shorthand_id: "asd".to_owned(),
    //             url: "https://google.com".to_owned(),
    //             visitor_id: visitor_id.to_string(),
    //             latitude: Some(12.23),
    //             longitude: Some(53.23),
    //         })
    //         .await
    //         .unwrap();
    // }

    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some("metrics".to_string());
    cfg.host = Some("localhost".to_string());
    cfg.port = Some(6432);
    cfg.user = Some("postgres".to_string());
    cfg.password = Some("password".to_string());

    cfg.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    let pool = cfg
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .unwrap();

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
                flush_direct(client, metrics).await.ok();
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
            ip: header_to_string(&headers, "cloudfront-viewer-address")
                .unwrap_or_else(|| addr.ip().to_string()),
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

        if app.metrics_buffer.len() >= 1000 {
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();

            if let Ok(client) = app.pool.get().await {
                tokio::spawn(flush_direct(client, metrics));
            }
        }

        return Ok((jar, Redirect::temporary(&url)));
    }

    Err(StatusCode::NOT_FOUND)
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
