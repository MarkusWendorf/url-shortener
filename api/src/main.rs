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

use axum::middleware::from_fn_with_state;
use middleware::auth::AuthMiddlewareState;
use routes::{api, auth, shorten};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::Router;

#[tokio::main]
async fn main() {
    let pg_pool = postgres::create_connection_pool();
    let mut pg_conn = pg_pool.get().await.unwrap();
    postgres::run_migrations(&mut pg_conn).await;

    let mut sqlite_conn = sqlite::create_connection();
    sqlite::run_migrations(&mut sqlite_conn);

    let middleware_state = Arc::new(Mutex::new(AuthMiddlewareState {
        connection: sqlite_conn,
    }));

    let auth_middleware = from_fn_with_state(middleware_state, middleware::auth::authorization_middleware);

    let app = Router::new()
        .merge(shorten::router(pg_pool))
        .nest("/auth", auth::router())
        .nest("/api", api::router(pg_conn).layer(auth_middleware));

    println!("API started!");

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
