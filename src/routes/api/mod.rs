pub mod api;

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::{
    entities::MetricsWithinInterval,
    id::generate_id,
    middleware::auth::UserSession,
    sqlite,
    structs::{CreateShortUrl, ShortUrlCreated},
};

pub struct ApiAppState {
    pg_conn: deadpool_postgres::Object,
    connection: Connection,
}

pub fn router(pg_conn: deadpool_postgres::Object) -> Router {
    let connection = sqlite::create_connection();
    let state = Arc::new(Mutex::new(ApiAppState { connection, pg_conn }));

    Router::new()
        .route("/create-short-url", post(create_short_url))
        .route("/metrics", get(get_metrics))
        .with_state(state)
}

async fn create_short_url(
    State(state): State<Arc<Mutex<ApiAppState>>>,
    session: Extension<UserSession>,
    Json(payload): Json<CreateShortUrl>,
) -> impl IntoResponse {
    let mut app_state = state.lock().await;
    let connection = &mut app_state.connection;

    let mut retries = 0;

    while retries < 5 {
        let id = generate_id();
        match api::create_short_url(connection, session.user.id, &id, &payload.url) {
            Ok(_) => return (StatusCode::CREATED, Json(ShortUrlCreated { id })).into_response(),
            // Duplicate Key (code=1555)
            Err(rusqlite::Error::SqliteFailure(err, _)) if err.extended_code == 1555 => retries += 1,
            Err(_) => break,
        }
    }

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

async fn get_metrics(
    State(state): State<Arc<Mutex<ApiAppState>>>,
    session: Extension<UserSession>,
) -> impl IntoResponse {
    let app_state = state.lock().await;
    let user_id = session.user.id;

    let interval = "1 minute";

    let query = app_state
        .pg_conn
        .query(
            r"
        SELECT 
          time_bucket('1 minute', created_at) AS bucket,
          count(*) AS count,
          count(DISTINCT visitor_id) AS unique_count
        FROM
          metrics
        WHERE 
          user_id = $1
        GROUP BY 
          bucket
        ORDER BY
          bucket DESC
        ",
            &[&user_id],
        )
        .await;

    let metrics: Vec<MetricsWithinInterval> = match query {
        Ok(rows) => rows
            .iter()
            .map(|row| MetricsWithinInterval {
                count: row.get("count"),
                unique_count: row.get("unique_count"),
            })
            .collect(),
        Err(err) => {
            println!("{:?}", err);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    println!("{:?}", metrics);

    // TODO: set cache-control headers
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
