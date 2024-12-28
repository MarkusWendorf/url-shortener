use std::ops::DerefMut;

mod sqlite_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/sqlite");
}

mod postgres_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/postgres");
}

async fn run_postgres_migrations(mut conn: deadpool_postgres::Object) {
    postgres_migrations::migrations::runner()
        .run_async(conn.deref_mut().deref_mut())
        .await
        .unwrap();
}

fn run_sqlite_migrations(conn: &mut rusqlite::Connection) {
    sqlite_migrations::migrations::runner().run(conn).unwrap();
}

pub async fn run_migrations(
    sqlite_connection: &mut rusqlite::Connection,
    deadpool: &deadpool_postgres::Pool,
) {
    run_sqlite_migrations(sqlite_connection);

    let postgres_client = deadpool.get().await.unwrap();
    run_postgres_migrations(postgres_client).await;
}
