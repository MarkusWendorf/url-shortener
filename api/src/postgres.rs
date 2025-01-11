use std::ops::DerefMut;

use deadpool_postgres::Pool;
use tokio_postgres::NoTls;

mod postgres_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/postgres");
}

fn env(env_key: &str, default: &str) -> Option<String> {
    Some(std::env::var(env_key).unwrap_or(default.to_owned()))
}

fn env_int(env_key: &str, default: u16) -> Option<u16> {
    std::env::var(env_key)
        .map(|value| value.parse().ok())
        .unwrap_or(Some(default))
}

pub fn create_connection_pool() -> Pool {
    let deadpool = deadpool_postgres::Config {
        dbname: env("DB_DATABASE", "metrics"),
        host: env("DB_HOST", "localhost"),
        port: env_int("DB_PORT", 6432),
        user: env("DB_USER", "postgres"),
        password: env("DB_PASSWORD", "password"),
        manager: Some(deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Fast,
        }),
        ..Default::default()
    };

    deadpool
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .unwrap()
}

pub async fn run_migrations(client: &mut deadpool_postgres::Object) {
    postgres_migrations::migrations::runner()
        .run_async(client.deref_mut().deref_mut())
        .await
        .unwrap();
}
