use std::ops::DerefMut;

use deadpool_postgres::Pool;
use tokio_postgres::NoTls;

mod postgres_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/postgres");
}

pub fn create_connection_pool() -> Pool {
    let deadpool = deadpool_postgres::Config {
        dbname: Some("metrics".to_string()),
        host: Some("localhost".to_string()),
        port: Some(6432),
        user: Some("postgres".to_string()),
        password: Some("password".to_string()),
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
