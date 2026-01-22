pub mod example;

pub use sqlx::{
    Connection, Error as SqlxError,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgQueryResult},
};

// sqlx::migrate!() 會從 crate 根目錄查找 migrations 目錄
// migrations 目錄位於 backend/core/migrations
static EMBEDDED_MIGRATE: Migrator = sqlx::migrate!();

pub async fn connect_pg(
    database_url: &str,
    max_connections: u32,
    application_name: Option<&str>,
) -> sqlx::Result<PgPool> {
    let mut opts = database_url.parse::<PgConnectOptions>()?;
    if let Some(application_name) = application_name {
        opts = opts.application_name(application_name);
    }
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(opts)
        .await
}

pub fn ensure_affected(count: u64) -> impl FnOnce(PgQueryResult) -> sqlx::Result<()> {
    move |pg_done| {
        if pg_done.rows_affected() == count {
            Ok(())
        } else {
            Err(SqlxError::RowNotFound)
        }
    }
}

pub async fn migrate(pg_pool: &PgPool) -> sqlx::Result<()> {
    let mut v: Vec<sqlx::migrate::Migration> = vec![];
    v.extend(EMBEDDED_MIGRATE.migrations.iter().cloned());

    //#NOTE lucidstream_pg::migrate generates some functions required by eg migration
    sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(v),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
    .run(pg_pool)
    .await?;
    Ok(())
}

pub async fn setup_test_db(name: &'static str) -> Result<PgPool, sqlx::Error> {
    // Initial connection using `PgConnection` instead of `PgPool`
    let mut conn = sqlx::PgConnection::connect(
        "postgres://postgres:123456@localhost/postgres?sslmode=disable",
    )
    .await?;

    let db_name = format!("test_{name}");
    let res = sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
        .execute(&mut conn)
        .await;
    if res.is_err() {
        println!("WARNING: {db_name} already exists, dropping");
        sqlx::query(&format!("DROP DATABASE \"{db_name}\""))
            .execute(&mut conn)
            .await?;
        sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
            .execute(&mut conn)
            .await?;
    }

    let db_url = format!("postgres://postgres:123456@localhost:5432/{db_name}?sslmode=disable");
    let opts = db_url.parse::<PgConnectOptions>()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;

    migrate(&pool).await?;
    Ok(pool)
}

pub async fn teardown_test_db(name: &'static str, pool: PgPool) -> Result<(), sqlx::Error> {
    drop(pool);

    let mut conn = sqlx::PgConnection::connect(
        "postgres://postgres:123456@localhost/postgres?sslmode=disable",
    )
    .await?;

    let db_name = format!("test_{name}");
    sqlx::query(&format!("DROP DATABASE \"{db_name}\""))
        .execute(&mut conn)
        .await?;
    Ok(())
}
