use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::Redirect,
    routing::get,
    Router,
};

use axum_extra::extract::cookie::{Cookie, CookieJar};
use rusqlite::Connection;
use time::OffsetDateTime;
use tokio::{sync::Mutex, time::interval};

use crate::{
    headers::TypedHeaderValues,
    id::generate_id,
    metrics::{persist_metrics, Metric},
    sqlite,
};

const BUFFER_SIZE: usize = 1000;
const VISITOR_COOKIE: &str = "visitor-id";

pub struct PublicAppState {
    connection: Connection,
    metrics_buffer: Vec<Metric>,
    pg_pool: deadpool_postgres::Pool,
}

pub fn router(pg_pool: deadpool_postgres::Pool) -> Router {
    let connection = sqlite::create_connection();

    let state = Arc::new(Mutex::new(PublicAppState {
        connection,
        metrics_buffer: Vec::with_capacity(BUFFER_SIZE),
        pg_pool,
    }));

    let mut interval = interval(Duration::from_secs(10));
    let app_state = state.clone();

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let mut app = app_state.lock().await;
            let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();

            if let Ok(client) = app.pg_pool.get().await {
                persist_metrics(client, metrics).await.unwrap();
            }
        }
    });

    Router::new().route("/:id", get(redirect_to_url)).with_state(state)
}

async fn redirect_to_url(
    headers: HeaderMap,
    mut jar: CookieJar,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Mutex<PublicAppState>>>,
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

    let query = app
        .connection
        .prepare_cached("SELECT url FROM urls WHERE key = ?1")
        .map(|mut q| q.query_row([&id], |row| row.get::<usize, String>(0)).ok());

    let url = match query {
        Ok(Some(url)) => url,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let metric = Metric {
        visitor_id,
        shorthand_id: id,
        created_at: OffsetDateTime::now_utc(),
        ip: headers
            .string("cloudfront-viewer-address")
            .unwrap_or_else(|| addr.ip().to_string()),
        url: url.clone(),
        android: headers.bool("cloudfront-is-android-viewer"),
        ios: headers.bool("cloudfront-is-ios-viewer"),
        mobile: headers.bool("cloudfront-is-mobile-viewer"),
        region_name: headers.string("cloudfront-viewer-country-region-name"),
        country: headers.string("cloudfront-viewer-country"),
        city: headers.string("cloudfront-viewer-city"),
        zip_code: headers.string("cloudfront-viewer-postal-code"),
        time_zone: headers.string("cloudfront-viewer-time-zone"),
        user_agent: headers.string("user-agent"),
        longitude: headers.float("cloudfront-viewer-longitude"),
        latitude: headers.float("cloudfront-viewer-latitude"),
    };

    app.metrics_buffer.push(metric);

    if app.metrics_buffer.len() >= BUFFER_SIZE
        && let Ok(client) = app.pg_pool.get().await
    {
        let metrics: Vec<Metric> = app.metrics_buffer.drain(..).collect();
        tokio::spawn(persist_metrics(client, metrics));
    }

    Ok((jar, Redirect::temporary(&url)))
}
