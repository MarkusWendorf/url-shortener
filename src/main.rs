#![feature(let_chains)]
mod entities;
mod headers;
mod id;
mod metrics;
mod middleware;
mod postgres;
mod routes;
mod sqlite;
mod structs;
mod url_storage;

use ::time::OffsetDateTime;
use axum::middleware::from_fn_with_state;
use headers::*;
use middleware::MiddlewareState;
use routes::{api, auth};
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::{sync::Mutex, time};

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
    let pg_pool = postgres::create_connection_pool();
    let pg_conn = pg_pool.get().await.unwrap();
    postgres::run_migrations(pg_conn).await;

    let mut sqlite_conn = sqlite::create_connection();
    sqlite::run_migrations(&mut sqlite_conn);

    let storage = UrlStorage::new(sqlite_conn);

    let state = Arc::new(Mutex::new(AppState {
        storage,
        metrics_buffer: Vec::with_capacity(100000),
        pool: pg_pool,
    }));

    let mut interval = time::interval(Duration::from_secs(10));
    let app_state = state.clone();

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let mut app = app_state.lock().await;
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();

            if let Ok(client) = app.pool.get().await {
                persist_metrics(client, metrics).await.unwrap();
            }
        }
    });

    let middleware_state = Arc::new(Mutex::new(MiddlewareState {
        connection: sqlite::create_connection(),
    }));

    let app = Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .nest("/auth", auth::router())
        .nest(
            "/api",
            api::router().layer(from_fn_with_state(
                middleware_state,
                middleware::auth::authorization_middleware,
            )),
        );

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
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
            created_at: OffsetDateTime::now_utc(),
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
            longitude: Some(52.1),
            latitude: Some(12.123),
            // longitude: header_to_float(&headers, "cloudfront-viewer-longitude"),
            // latitude: header_to_float(&headers, "cloudfront-viewer-latitude"),
        };

        app.metrics_buffer.push(metric);

        if app.metrics_buffer.len() >= 1000
            && let Ok(client) = app.pool.get().await
        {
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();
            tokio::spawn(persist_metrics(client, metrics));
        }

        return Ok((jar, Redirect::temporary(&url)));
    }

    Err(StatusCode::NOT_FOUND)
}
